//! Procedural macros for bw-scripting Rhai integration.
//!
//! Provides derive macros to reduce boilerplate when working with Rhai's Dynamic type:
//!
//! - `#[derive(RhaiSerialize)]` - Generates `to_dynamic()` method
//! - `#[derive(RhaiDeserialize)]` - Generates `from_dynamic()` method
//! - `#[derive(RhaiSchema)]` - Generates `schema()` method for validation
//!
//! # Field Attributes
//!
//! ## `#[rhai(skip)]`
//! Skip this field during serialization/deserialization.
//!
//! ## `#[rhai(rename = "other_name")]`
//! Use a different key name in the Rhai map.
//!
//! ## `#[rhai(as_string)]`
//! Convert the field to/from a string (useful for Uuid, enums).
//!
//! ## `#[rhai(default)]`
//! Use Default::default() if the field is missing during deserialization.
//!
//! ## `#[rhai(default = "expression")]`
//! Use a specific default value if the field is missing.
//!
//! ## `#[rhai(flatten)]`
//! Flatten nested struct fields into the parent map.
//!
//! ## `#[rhai(with = "module::function")]`
//! Use a custom function for serialization/deserialization.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse_macro_input, Data, DeriveInput, Fields, Ident, Type, Attribute,
    Lit, Expr,
};

/// Derive macro for generating `to_dynamic()` method.
///
/// # Example
///
/// ```ignore
/// #[derive(RhaiSerialize)]
/// struct Position {
///     pub x: f64,
///     pub y: f64,
///     #[rhai(as_string)]
///     pub id: Uuid,
/// }
/// ```
#[proc_macro_derive(RhaiSerialize, attributes(rhai))]
pub fn derive_rhai_serialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match impl_rhai_serialize(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive macro for generating `from_dynamic()` method.
///
/// # Example
///
/// ```ignore
/// #[derive(RhaiDeserialize)]
/// struct ShipStats {
///     #[rhai(default = 10.0)]
///     pub attack: f32,
///     #[rhai(default = 10.0)]
///     pub defense: f32,
/// }
/// ```
#[proc_macro_derive(RhaiDeserialize, attributes(rhai))]
pub fn derive_rhai_deserialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match impl_rhai_deserialize(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

// =============================================================================
// Field Attributes
// =============================================================================

#[derive(Default)]
struct FieldAttrs {
    skip: bool,
    rename: Option<String>,
    as_string: bool,
    default: Option<DefaultValue>,
    flatten: bool,
    with_fn: Option<String>,
}

enum DefaultValue {
    Default,
    Expr(String),
}

fn parse_field_attrs(attrs: &[Attribute]) -> syn::Result<FieldAttrs> {
    let mut result = FieldAttrs::default();

    for attr in attrs {
        if !attr.path().is_ident("rhai") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                result.skip = true;
            } else if meta.path.is_ident("as_string") {
                result.as_string = true;
            } else if meta.path.is_ident("flatten") {
                result.flatten = true;
            } else if meta.path.is_ident("rename") {
                let value: Lit = meta.value()?.parse()?;
                if let Lit::Str(s) = value {
                    result.rename = Some(s.value());
                }
            } else if meta.path.is_ident("default") {
                if meta.input.peek(syn::Token![=]) {
                    let value: Expr = meta.value()?.parse()?;
                    let expr_str = quote!(#value).to_string();
                    result.default = Some(DefaultValue::Expr(expr_str));
                } else {
                    result.default = Some(DefaultValue::Default);
                }
            } else if meta.path.is_ident("with") {
                let value: Lit = meta.value()?.parse()?;
                if let Lit::Str(s) = value {
                    result.with_fn = Some(s.value());
                }
            }
            Ok(())
        })?;
    }

    Ok(result)
}

// =============================================================================
// RhaiSerialize Implementation
// =============================================================================

fn impl_rhai_serialize(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => return Err(syn::Error::new_spanned(input, "Only named fields are supported")),
        },
        _ => return Err(syn::Error::new_spanned(input, "Only structs are supported")),
    };

    let mut insertions = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let attrs = parse_field_attrs(&field.attrs)?;

        if attrs.skip {
            continue;
        }

        let key_name = attrs.rename.clone()
            .unwrap_or_else(|| field_name.to_string());

        if attrs.flatten {
            // Flatten: merge nested struct's map into this one
            insertions.push(quote! {
                let nested = self.#field_name.to_dynamic();
                if let Some(nested_map) = nested.try_cast::<rhai::Map>() {
                    for (k, v) in nested_map {
                        map.insert(k, v);
                    }
                }
            });
        } else if let Some(with_fn) = &attrs.with_fn {
            // Custom serialization function
            let fn_path: syn::Path = syn::parse_str(with_fn)?;
            insertions.push(quote! {
                map.insert(#key_name.into(), #fn_path(&self.#field_name));
            });
        } else {
            // Standard serialization
            let conversion = serialize_field_expr(field_name, field_type, &attrs);
            insertions.push(quote! {
                map.insert(#key_name.into(), #conversion);
            });
        }
    }

    Ok(quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            /// Convert to Rhai Dynamic map.
            pub fn to_dynamic(&self) -> rhai::Dynamic {
                let mut map = rhai::Map::new();
                #(#insertions)*
                rhai::Dynamic::from(map)
            }
        }
    })
}

fn serialize_field_expr(field_name: &Ident, field_type: &Type, attrs: &FieldAttrs) -> TokenStream2 {
    let type_str = quote!(#field_type).to_string().replace(" ", "");

    // Handle as_string attribute
    if attrs.as_string {
        return quote! { self.#field_name.to_string().into() };
    }

    // Type-specific conversions
    if type_str == "Uuid" || type_str == "uuid::Uuid" {
        quote! { self.#field_name.to_string().into() }
    } else if type_str.starts_with("Option<") {
        // Extract inner type
        if type_str.contains("Uuid") {
            quote! { self.#field_name.map(|v| v.to_string()).unwrap_or_default().into() }
        } else {
            quote! { self.#field_name.clone().map(|v| rhai::Dynamic::from(v)).unwrap_or(rhai::Dynamic::UNIT) }
        }
    } else if type_str == "String" {
        quote! { self.#field_name.clone().into() }
    } else if type_str == "f32" {
        quote! { (self.#field_name as f64).into() }
    } else if type_str == "i32" || type_str == "u32" || type_str == "i16" || type_str == "u16" {
        quote! { (self.#field_name as i64).into() }
    } else if type_str == "f64" || type_str == "i64" || type_str == "bool" {
        quote! { self.#field_name.into() }
    } else if type_str.starts_with("Vec<") {
        // Check if inner type has to_dynamic
        if type_str.contains("Snapshot") || type_str.contains("Upgrade") || type_str.contains("Cargo") {
            quote! {
                rhai::Dynamic::from(
                    self.#field_name.iter()
                        .map(|item| item.to_dynamic())
                        .collect::<Vec<rhai::Dynamic>>()
                )
            }
        } else if type_str.contains("Uuid") {
            quote! {
                rhai::Dynamic::from(
                    self.#field_name.iter()
                        .map(|id| rhai::Dynamic::from(id.to_string()))
                        .collect::<Vec<rhai::Dynamic>>()
                )
            }
        } else if type_str.contains("String") {
            quote! {
                rhai::Dynamic::from(
                    self.#field_name.iter()
                        .map(|s| rhai::Dynamic::from(s.clone()))
                        .collect::<Vec<rhai::Dynamic>>()
                )
            }
        } else {
            quote! {
                rhai::Dynamic::from(
                    self.#field_name.iter()
                        .map(|v| rhai::Dynamic::from(v.clone()))
                        .collect::<Vec<rhai::Dynamic>>()
                )
            }
        }
    } else {
        // Assume it has to_dynamic() method
        quote! { self.#field_name.to_dynamic() }
    }
}

// =============================================================================
// RhaiDeserialize Implementation
// =============================================================================

fn impl_rhai_deserialize(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => return Err(syn::Error::new_spanned(input, "Only named fields are supported")),
        },
        _ => return Err(syn::Error::new_spanned(input, "Only structs are supported")),
    };

    let mut field_parsers = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let attrs = parse_field_attrs(&field.attrs)?;

        if attrs.skip {
            field_parsers.push(quote! {
                #field_name: Default::default()
            });
            continue;
        }

        let key_name = attrs.rename.clone()
            .unwrap_or_else(|| field_name.to_string());

        let parser = if let Some(with_fn) = &attrs.with_fn {
            // Custom deserialization function
            let fn_path: syn::Path = syn::parse_str(with_fn)?;
            // Respect default attribute for custom functions too
            let default_handler = match &attrs.default {
                Some(DefaultValue::Default) => quote! { .unwrap_or_default() },
                Some(DefaultValue::Expr(expr)) => {
                    let expr_tokens: TokenStream2 = expr.parse()?;
                    quote! { .unwrap_or(#expr_tokens) }
                }
                None => quote! { ? },
            };
            quote! { #fn_path(map.get(#key_name)) #default_handler }
        } else if attrs.flatten {
            // Flatten: parse from the entire map
            let inner_type = field_type;
            quote! { #inner_type::from_dynamic(&rhai::Dynamic::from(map.clone()))? }
        } else {
            deserialize_field_expr(&key_name, field_type, &attrs)?
        };

        field_parsers.push(quote! {
            #field_name: #parser
        });
    }

    Ok(quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            /// Parse from a Rhai Dynamic value.
            pub fn from_dynamic(value: &rhai::Dynamic) -> Option<Self> {
                let map = value.clone().try_cast::<rhai::Map>()?;

                Some(Self {
                    #(#field_parsers),*
                })
            }
        }
    })
}

fn deserialize_field_expr(key: &str, field_type: &Type, attrs: &FieldAttrs) -> syn::Result<TokenStream2> {
    let type_str = quote!(#field_type).to_string().replace(" ", "");

    // Handle default attribute
    let default_handler = match &attrs.default {
        Some(DefaultValue::Default) => quote! { .unwrap_or_default() },
        Some(DefaultValue::Expr(expr)) => {
            let expr_tokens: TokenStream2 = expr.parse()?;
            quote! { .unwrap_or(#expr_tokens) }
        }
        None => quote! { ? },
    };

    // Handle as_string attribute (for Uuid parsing)
    if attrs.as_string {
        return Ok(quote! {
            map.get(#key)
                .and_then(|v| v.clone().into_string().ok())
                .and_then(|s| s.parse().ok())
                #default_handler
        });
    }

    // Type-specific parsing
    let parser = if type_str == "String" {
        quote! {
            map.get(#key)
                .and_then(|v| v.clone().into_string().ok())
                #default_handler
        }
    } else if type_str == "f32" {
        quote! {
            map.get(#key)
                .and_then(|v| v.as_float().ok().map(|f| f as f32)
                    .or_else(|| v.as_int().ok().map(|i| i as f32)))
                #default_handler
        }
    } else if type_str == "f64" {
        quote! {
            map.get(#key)
                .and_then(|v| v.as_float().ok()
                    .or_else(|| v.as_int().ok().map(|i| i as f64)))
                #default_handler
        }
    } else if type_str == "i32" {
        quote! {
            map.get(#key)
                .and_then(|v| v.as_int().ok().map(|i| i as i32))
                #default_handler
        }
    } else if type_str == "i64" {
        quote! {
            map.get(#key)
                .and_then(|v| v.as_int().ok())
                #default_handler
        }
    } else if type_str == "u32" {
        quote! {
            map.get(#key)
                .and_then(|v| v.as_int().ok().map(|i| i as u32))
                #default_handler
        }
    } else if type_str == "bool" {
        quote! {
            map.get(#key)
                .and_then(|v| v.as_bool().ok())
                #default_handler
        }
    } else if type_str.starts_with("Option<") {
        // Optional field - extract inner type handling
        if type_str.contains("String") {
            quote! {
                map.get(#key)
                    .and_then(|v| v.clone().into_string().ok())
            }
        } else if type_str.contains("f32") {
            quote! {
                map.get(#key)
                    .and_then(|v| v.as_float().ok().map(|f| f as f32)
                        .or_else(|| v.as_int().ok().map(|i| i as f32)))
            }
        } else if type_str.contains("f64") {
            quote! {
                map.get(#key)
                    .and_then(|v| v.as_float().ok()
                        .or_else(|| v.as_int().ok().map(|i| i as f64)))
            }
        } else if type_str.contains("i32") {
            quote! {
                map.get(#key)
                    .and_then(|v| v.as_int().ok().map(|i| i as i32))
            }
        } else if type_str.contains("i64") {
            quote! {
                map.get(#key)
                    .and_then(|v| v.as_int().ok())
            }
        } else if type_str.contains("u32") {
            quote! {
                map.get(#key)
                    .and_then(|v| v.as_int().ok().map(|i| i as u32))
            }
        } else if type_str.contains("bool") {
            quote! {
                map.get(#key)
                    .and_then(|v| v.as_bool().ok())
            }
        } else {
            quote! {
                map.get(#key)
                    .cloned()
            }
        }
    } else if type_str.starts_with("Vec<") {
        // Vector field - parse array
        if type_str.contains("String") {
            quote! {
                map.get(#key)
                    .and_then(|v| v.clone().try_cast::<rhai::Array>())
                    .map(|arr| arr.into_iter()
                        .filter_map(|item| item.into_string().ok())
                        .collect())
                    #default_handler
            }
        } else {
            // Assume inner type has from_dynamic
            quote! {
                map.get(#key)
                    .and_then(|v| v.clone().try_cast::<rhai::Array>())
                    .map(|arr| arr.into_iter()
                        .filter_map(|item| Self::parse_array_item(&item))
                        .collect())
                    #default_handler
            }
        }
    } else {
        // Assume it has from_dynamic() method
        quote! {
            map.get(#key)
                .and_then(|v| #field_type::from_dynamic(v))
                #default_handler
        }
    };

    Ok(parser)
}

// =============================================================================
// RhaiSchema Implementation
// =============================================================================

/// Derive macro for generating schema information for validation.
///
/// Generates an implementation of the `RhaiSchema` trait which provides
/// static schema information about the struct's fields for script validation.
///
/// # Example
///
/// ```ignore
/// #[derive(RhaiSchema)]
/// struct ShipArchetype {
///     pub id: String,           // required
///     pub name: String,         // required
///     #[rhai(default)]
///     pub tier: u32,            // optional (has default)
///     #[rhai(rename = "hp")]
///     pub health: f32,          // renamed in Rhai
/// }
/// ```
#[proc_macro_derive(RhaiSchema, attributes(rhai))]
pub fn derive_rhai_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match impl_rhai_schema(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn impl_rhai_schema(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let name_str = name.to_string();
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => return Err(syn::Error::new_spanned(input, "Only named fields are supported")),
        },
        _ => return Err(syn::Error::new_spanned(input, "Only structs are supported")),
    };

    let mut field_schemas = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let attrs = parse_field_attrs(&field.attrs)?;

        if attrs.skip {
            continue;
        }

        // Get the Rhai key name (respecting rename)
        let key_name = attrs.rename.clone()
            .unwrap_or_else(|| field_name.to_string());

        // Get the Rust type as a string for schema
        let type_str = simplify_type_name(&quote!(#field_type).to_string());

        // Field is required if it has no default attribute
        let required = attrs.default.is_none();

        field_schemas.push(quote! {
            crate::schema::FieldSchema {
                name: #key_name,
                rust_type: #type_str,
                required: #required,
            }
        });
    }

    let field_count = field_schemas.len();

    Ok(quote! {
        impl #impl_generics crate::schema::RhaiSchema for #name #ty_generics #where_clause {
            fn schema() -> crate::schema::ArchetypeSchema {
                static FIELDS: [crate::schema::FieldSchema; #field_count] = [
                    #(#field_schemas),*
                ];
                crate::schema::ArchetypeSchema {
                    name: #name_str,
                    fields: &FIELDS,
                }
            }
        }
    })
}

/// Simplify type name for schema display.
fn simplify_type_name(type_str: &str) -> String {
    let s = type_str.replace(" ", "");
    // Simplify common patterns
    if s.starts_with("Vec<") {
        format!("Vec<{}>", simplify_inner_type(&s[4..s.len()-1]))
    } else if s.starts_with("Option<") {
        format!("Option<{}>", simplify_inner_type(&s[7..s.len()-1]))
    } else if s.starts_with("HashMap<") {
        s.to_string()
    } else {
        simplify_inner_type(&s)
    }
}

fn simplify_inner_type(s: &str) -> String {
    // Remove path prefixes like std::string::String -> String
    s.rsplit("::").next().unwrap_or(s).to_string()
}

// Note: Schema types (ArchetypeSchema, FieldSchema, RhaiSchema trait)
// are defined in bw-scripting::schema module since proc-macro crates
// cannot export regular items.
