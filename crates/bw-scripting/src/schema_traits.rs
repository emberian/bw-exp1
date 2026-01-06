//! Common traits for schema systems.
//!
//! Provides unified interfaces for the various schema types used in validation.
//! This allows generic code to work with ArchetypeSchema, DefinitionSchema,
//! and ObjectSchema using a common interface.

/// Type category for schema fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeCategory {
    String,
    Number,
    Bool,
    Array,
    Map,
    Optional,
    Unknown,
}

/// Trait for schema fields.
pub trait SchemaField {
    /// Get the field name.
    fn name(&self) -> &str;

    /// Get the type name for error messages.
    fn type_name(&self) -> &str;

    /// Get the type category.
    fn type_category(&self) -> TypeCategory;

    /// Check if this field is required.
    fn is_required(&self) -> bool;
}

/// Trait for object/struct schemas.
pub trait Schema {
    /// Field type for this schema.
    type Field: SchemaField;

    /// Get the schema name.
    fn name(&self) -> &str;

    /// Get all fields.
    fn fields(&self) -> &[Self::Field];

    /// Check if a field exists by name.
    fn has_field(&self, name: &str) -> bool {
        self.fields().iter().any(|f| f.name() == name)
    }

    /// Get a field by name.
    fn get_field(&self, name: &str) -> Option<&Self::Field> {
        self.fields().iter().find(|f| f.name() == name)
    }

    /// Get names of required fields.
    fn required_field_names(&self) -> impl Iterator<Item = &str> {
        self.fields().iter().filter(|f| f.is_required()).map(|f| f.name())
    }

    /// Get all field names.
    fn field_names(&self) -> impl Iterator<Item = &str> {
        self.fields().iter().map(|f| f.name())
    }
}

/// Trait for schema registries that store multiple schemas.
pub trait SchemaRegistry {
    /// Schema type stored in this registry.
    type Schema: Schema;

    /// Get a schema by name.
    fn get_schema(&self, name: &str) -> Option<&Self::Schema>;

    /// Check if a schema exists.
    fn has_schema(&self, name: &str) -> bool {
        self.get_schema(name).is_some()
    }
}
