//! Schema browser component for viewing archetype schemas

use leptos::prelude::*;
use bw_shared::dto::{ArchetypeSchemaDto, FieldSchemaDto};

use crate::api::admin_ws;

/// Schema browser component
#[component]
pub fn SchemaBrowser() -> impl IntoView {
    // State
    let schemas = RwSignal::new(Vec::<ArchetypeSchemaDto>::new());
    let selected_type = RwSignal::new(Option::<String>::None);
    let fields = RwSignal::new(Vec::<FieldSchemaDto>::new());
    let loading = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);

    // Load schemas on mount
    Effect::new(move |_| {
        loading.set(true);
        admin_ws::send_get_archetype_schemas();
    });

    // Handle incoming messages
    Effect::new(move |_| {
        if let Some(msg) = admin_ws::poll_message() {
            use bw_shared::AdminServerMessage;
            match msg {
                AdminServerMessage::ArchetypeSchemas { schemas: s } => {
                    schemas.set(s);
                    loading.set(false);
                }
                AdminServerMessage::ArchetypeSchemaDetail { archetype_type: _, name: _, fields: f } => {
                    fields.set(f);
                    loading.set(false);
                }
                AdminServerMessage::AdminError { code: _, message } => {
                    error.set(Some(message));
                    loading.set(false);
                }
                _ => {}
            }
        }
    });

    // Select a schema type
    let on_select = move |type_name: String| {
        selected_type.set(Some(type_name.clone()));
        fields.set(vec![]);
        loading.set(true);
        admin_ws::send_get_archetype_schema(&type_name);
    };

    view! {
        <div class="h-full flex flex-col p-4">
            // Header
            <div class="flex items-center justify-between mb-4">
                <h2 class="text-lg font-semibold text-amber-500">"Archetype Schemas"</h2>
                <button
                    class="px-3 py-1 bg-slate-700 hover:bg-slate-600 text-sm rounded"
                    on:click=move |_| {
                        loading.set(true);
                        admin_ws::send_get_archetype_schemas();
                    }
                >
                    "Refresh"
                </button>
            </div>

            // Error display
            <Show when=move || error.get().is_some()>
                <div class="mb-4 p-2 bg-red-900/50 border border-red-500 rounded text-red-300 text-sm">
                    {move || error.get().unwrap_or_default()}
                </div>
            </Show>

            // Main content
            <div class="flex-1 flex gap-4 overflow-hidden">
                // Schema type list
                <div class="w-48 flex-shrink-0 overflow-auto">
                    <div class="space-y-1">
                        <For
                            each=move || schemas.get()
                            key=|s| s.archetype_type.clone()
                            children=move |schema| {
                                let type_name = schema.archetype_type.clone();
                                let type_name_click = type_name.clone();
                                let is_selected = move || {
                                    selected_type.get().as_deref() == Some(&type_name)
                                };
                                view! {
                                    <button
                                        class="w-full text-left px-3 py-2 rounded text-sm transition-colors"
                                        class:bg-amber-500/20=is_selected
                                        class:text-amber-500=is_selected
                                        class:hover:bg-slate-700=move || !is_selected()
                                        on:click=move |_| on_select(type_name_click.clone())
                                    >
                                        <div class="font-medium">{schema.name.clone()}</div>
                                        <div class="text-xs text-slate-400">
                                            {schema.field_count}" fields, "{schema.required_count}" required"
                                        </div>
                                    </button>
                                }
                            }
                        />
                    </div>
                </div>

                // Field details
                <div class="flex-1 overflow-auto">
                    <Show
                        when=move || selected_type.get().is_some()
                        fallback=|| view! {
                            <div class="flex items-center justify-center h-full text-slate-400">
                                "Select an archetype type to view its fields"
                            </div>
                        }
                    >
                        <Show
                            when=move || loading.get()
                            fallback=move || view! {
                                <FieldsTable fields=fields />
                            }
                        >
                            <div class="flex items-center justify-center h-32 text-slate-400">
                                "Loading..."
                            </div>
                        </Show>
                    </Show>
                </div>
            </div>
        </div>
    }
}

/// Table showing field details
#[component]
fn FieldsTable(fields: RwSignal<Vec<FieldSchemaDto>>) -> impl IntoView {
    view! {
        <table class="w-full text-sm">
            <thead class="bg-slate-800 sticky top-0">
                <tr>
                    <th class="px-3 py-2 text-left text-slate-300">"Field"</th>
                    <th class="px-3 py-2 text-left text-slate-300">"Type"</th>
                    <th class="px-3 py-2 text-left text-slate-300">"Rust Type"</th>
                    <th class="px-3 py-2 text-center text-slate-300">"Required"</th>
                </tr>
            </thead>
            <tbody>
                <For
                    each=move || fields.get()
                    key=|f| f.name.clone()
                    children=move |field| {
                        view! {
                            <tr class="border-b border-slate-700 hover:bg-slate-800/50">
                                <td class="px-3 py-2 font-mono text-amber-400">{field.name.clone()}</td>
                                <td class="px-3 py-2">
                                    <TypeBadge field_type=field.field_type.clone() />
                                </td>
                                <td class="px-3 py-2 font-mono text-xs text-slate-400">{field.rust_type.clone()}</td>
                                <td class="px-3 py-2 text-center">
                                    {if field.required {
                                        view! { <span class="text-red-400">"*"</span> }.into_any()
                                    } else {
                                        view! { <span class="text-slate-500">"-"</span> }.into_any()
                                    }}
                                </td>
                            </tr>
                        }
                    }
                />
            </tbody>
        </table>
    }
}

/// Badge showing field type
#[component]
fn TypeBadge(field_type: String) -> impl IntoView {
    let (bg_class, text_class) = match field_type.as_str() {
        "string" => ("bg-green-900/50", "text-green-400"),
        "number" => ("bg-blue-900/50", "text-blue-400"),
        "bool" => ("bg-purple-900/50", "text-purple-400"),
        "array" => ("bg-orange-900/50", "text-orange-400"),
        "map" => ("bg-cyan-900/50", "text-cyan-400"),
        _ => ("bg-slate-700", "text-slate-300"),
    };

    view! {
        <span class=format!("px-2 py-0.5 rounded text-xs {} {}", bg_class, text_class)>
            {field_type}
        </span>
    }
}
