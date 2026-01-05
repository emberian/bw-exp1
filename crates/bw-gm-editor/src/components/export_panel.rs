//! Export panel component for creating and managing exports

use leptos::prelude::*;
use bw_shared::dto::{ExportConfigDto, ExportFormat, ExportSummaryDto, ExportStatus};

use crate::api::admin_ws;

/// Export panel component
#[component]
pub fn ExportPanel() -> impl IntoView {
    // State
    let exports = RwSignal::new(Vec::<ExportSummaryDto>::new());
    let loading = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);
    let creating = RwSignal::new(false);

    // Export config form state
    let export_name = RwSignal::new(String::new());
    let include_players = RwSignal::new(true);
    let include_npcs = RwSignal::new(true);
    let include_missions = RwSignal::new(true);
    let include_scripts = RwSignal::new(true);
    let include_sqlite = RwSignal::new(false);

    // Load exports on mount
    Effect::new(move |_| {
        loading.set(true);
        admin_ws::send_list_exports();
    });

    // Handle incoming messages
    Effect::new(move |_| {
        if let Some(msg) = admin_ws::poll_message() {
            use bw_shared::AdminServerMessage;
            match msg {
                AdminServerMessage::ExportList { exports: e } => {
                    exports.set(e);
                    loading.set(false);
                }
                AdminServerMessage::ExportCreated { export_id: _, name } => {
                    creating.set(false);
                    export_name.set(String::new());
                    // Refresh list
                    admin_ws::send_list_exports();
                    tracing::info!("Export '{}' created", name);
                }
                AdminServerMessage::ExportProgress { export_id: _, phase, percent } => {
                    tracing::info!("Export progress: {} ({}%)", phase, percent);
                }
                AdminServerMessage::ExportCompleted { export_id: _, size_bytes, download_url } => {
                    tracing::info!("Export completed: {} bytes, url: {}", size_bytes, download_url);
                    admin_ws::send_list_exports();
                }
                AdminServerMessage::ExportFailed { export_id: _, error: e } => {
                    error.set(Some(format!("Export failed: {}", e)));
                    creating.set(false);
                    admin_ws::send_list_exports();
                }
                AdminServerMessage::ExportDeleted { export_id: _ } => {
                    admin_ws::send_list_exports();
                }
                AdminServerMessage::AdminError { code: _, message } => {
                    error.set(Some(message));
                    loading.set(false);
                    creating.set(false);
                }
                _ => {}
            }
        }
    });

    let on_create_export = move |_| {
        let name = export_name.get();
        if name.is_empty() {
            error.set(Some("Export name is required".to_string()));
            return;
        }

        creating.set(true);
        error.set(None);

        let config = ExportConfigDto {
            sectors: vec![],
            include_players: include_players.get(),
            include_npcs: include_npcs.get(),
            include_missions: include_missions.get(),
            include_scripts: include_scripts.get(),
            include_definitions: true,
            include_sqlite: include_sqlite.get(),
            format: if include_sqlite.get() { ExportFormat::Both } else { ExportFormat::Cbor },
        };

        admin_ws::send_create_export(&name, config);
    };

    view! {
        <div class="h-full flex flex-col p-4">
            // Header
            <div class="flex items-center justify-between mb-4">
                <h2 class="text-lg font-semibold text-amber-500">"Export Manager"</h2>
                <button
                    class="px-3 py-1 bg-slate-700 hover:bg-slate-600 text-sm rounded"
                    on:click=move |_| {
                        loading.set(true);
                        admin_ws::send_list_exports();
                    }
                >
                    "Refresh"
                </button>
            </div>

            // Error display
            <Show when=move || error.get().is_some()>
                <div class="mb-4 p-2 bg-red-900/50 border border-red-500 rounded text-red-300 text-sm">
                    {move || error.get().unwrap_or_default()}
                    <button
                        class="ml-2 text-red-400 hover:text-red-300"
                        on:click=move |_| error.set(None)
                    >
                        "×"
                    </button>
                </div>
            </Show>

            <div class="flex gap-4 flex-1 overflow-hidden">
                // Create export form
                <div class="w-80 flex-shrink-0 bg-slate-800/50 rounded p-4">
                    <h3 class="text-sm font-medium text-slate-300 mb-4">"Create New Export"</h3>

                    <div class="space-y-4">
                        // Name input
                        <div>
                            <label class="block text-xs text-slate-400 mb-1">"Export Name"</label>
                            <input
                                type="text"
                                class="w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded text-sm"
                                placeholder="my-export"
                                prop:value=move || export_name.get()
                                on:input=move |ev| export_name.set(event_target_value(&ev))
                            />
                        </div>

                        // Include options
                        <div class="space-y-2">
                            <label class="block text-xs text-slate-400">"Include"</label>

                            <CheckboxOption
                                label="Players"
                                checked=include_players
                            />
                            <CheckboxOption
                                label="NPCs"
                                checked=include_npcs
                            />
                            <CheckboxOption
                                label="Missions"
                                checked=include_missions
                            />
                            <CheckboxOption
                                label="Scripts"
                                checked=include_scripts
                            />
                            <CheckboxOption
                                label="SQLite Snapshot"
                                checked=include_sqlite
                            />
                        </div>

                        // Create button
                        <button
                            class="w-full px-4 py-2 bg-amber-600 hover:bg-amber-500 text-white rounded disabled:opacity-50"
                            disabled=move || creating.get()
                            on:click=on_create_export
                        >
                            {move || if creating.get() { "Creating..." } else { "Create Export" }}
                        </button>
                    </div>
                </div>

                // Export list
                <div class="flex-1 overflow-auto">
                    <h3 class="text-sm font-medium text-slate-300 mb-4">"Available Exports"</h3>

                    <Show
                        when=move || loading.get()
                        fallback=move || view! {
                            <ExportList exports=exports />
                        }
                    >
                        <div class="flex items-center justify-center h-32 text-slate-400">
                            "Loading..."
                        </div>
                    </Show>
                </div>
            </div>
        </div>
    }
}

/// Checkbox option component
#[component]
fn CheckboxOption(
    label: &'static str,
    checked: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <label class="flex items-center gap-2 cursor-pointer">
            <input
                type="checkbox"
                class="rounded border-slate-600 bg-slate-700 text-amber-500"
                prop:checked=move || checked.get()
                on:change=move |ev| checked.set(event_target_checked(&ev))
            />
            <span class="text-sm text-slate-300">{label}</span>
        </label>
    }
}

/// Export list component
#[component]
fn ExportList(exports: RwSignal<Vec<ExportSummaryDto>>) -> impl IntoView {
    view! {
        <Show
            when=move || !exports.get().is_empty()
            fallback=|| view! {
                <div class="text-center text-slate-400 py-8">
                    "No exports yet. Create one to get started."
                </div>
            }
        >
            <div class="space-y-2">
                <For
                    each=move || exports.get()
                    key=|e| e.id
                    children=move |export| {
                        view! {
                            <ExportCard export=export />
                        }
                    }
                />
            </div>
        </Show>
    }
}

/// Single export card
#[component]
fn ExportCard(export: ExportSummaryDto) -> impl IntoView {
    let status_class = match export.status {
        ExportStatus::Completed => "text-green-400",
        ExportStatus::InProgress => "text-yellow-400",
        ExportStatus::Failed => "text-red-400",
    };

    let status_text = match export.status {
        ExportStatus::Completed => "Completed",
        ExportStatus::InProgress => "In Progress",
        ExportStatus::Failed => "Failed",
    };

    let format_text = match export.format {
        ExportFormat::Cbor => "CBOR",
        ExportFormat::Sqlite => "SQLite",
        ExportFormat::Both => "CBOR + SQLite",
    };

    let size_mb = export.size_bytes as f64 / (1024.0 * 1024.0);
    let export_id = export.id;

    view! {
        <div class="bg-slate-800/50 border border-slate-700 rounded p-3">
            <div class="flex items-center justify-between">
                <div>
                    <div class="font-medium">{export.name}</div>
                    <div class="text-xs text-slate-400 mt-1">
                        <span class=status_class>{status_text}</span>
                        " · "
                        {format_text}
                        " · "
                        {format!("{:.2} MB", size_mb)}
                    </div>
                </div>
                <div class="flex items-center gap-2">
                    <Show when=move || export.status == ExportStatus::Completed>
                        <button
                            class="px-2 py-1 bg-slate-700 hover:bg-slate-600 text-xs rounded"
                            on:click=move |_| {
                                admin_ws::send_download_export(export_id);
                            }
                        >
                            "Download"
                        </button>
                    </Show>
                    <button
                        class="px-2 py-1 bg-red-900/50 hover:bg-red-900 text-red-400 text-xs rounded"
                        on:click=move |_| {
                            admin_ws::send_delete_export(export_id);
                        }
                    >
                        "Delete"
                    </button>
                </div>
            </div>
        </div>
    }
}
