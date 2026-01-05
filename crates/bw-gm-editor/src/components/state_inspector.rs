//! State inspector component for viewing live game state

use leptos::prelude::*;
use bw_shared::EntityType;

use crate::api::admin_ws;

/// State inspector component
#[component]
pub fn StateInspector() -> impl IntoView {
    // State
    let selected_type = RwSignal::new(EntityType::Ship);
    let entities = RwSignal::new(Vec::<serde_json::Value>::new());
    let total_count = RwSignal::new(0usize);
    let current_tick = RwSignal::new(0u64);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);

    // Pagination
    let limit = 50usize;
    let offset = RwSignal::new(0usize);

    // Load state on type change
    let fetch_state = move || {
        loading.set(true);
        error.set(None);
        admin_ws::send_get_state_snapshot(selected_type.get(), limit, offset.get());
    };

    Effect::new(move |_| {
        let _ = selected_type.get();
        let _ = offset.get();
        fetch_state();
    });

    // Handle incoming messages
    Effect::new(move |_| {
        if let Some(msg) = admin_ws::poll_message() {
            use bw_shared::AdminServerMessage;
            match msg {
                AdminServerMessage::StateSnapshot { tick, entity_type: _, entities: e, total_count: tc } => {
                    entities.set(e);
                    total_count.set(tc);
                    current_tick.set(tick);
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

    view! {
        <div class="h-full flex flex-col p-4">
            // Header
            <div class="flex items-center justify-between mb-4">
                <div class="flex items-center gap-4">
                    <h2 class="text-lg font-semibold text-amber-500">"State Inspector"</h2>
                    <span class="text-xs text-slate-400">
                        "Tick: "{move || current_tick.get()}
                    </span>
                </div>
                <div class="flex items-center gap-2">
                    // Entity type selector
                    <select
                        class="px-2 py-1 bg-slate-700 border border-slate-600 rounded text-sm"
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            let entity_type = match value.as_str() {
                                "Ship" => EntityType::Ship,
                                "Player" => EntityType::Player,
                                "Sector" => EntityType::Sector,
                                "Mission" => EntityType::Mission,
                                "Station" => EntityType::Station,
                                _ => EntityType::Ship,
                            };
                            selected_type.set(entity_type);
                            offset.set(0);
                        }
                    >
                        <option value="Ship">"Ships"</option>
                        <option value="Player">"Players"</option>
                        <option value="Sector">"Sectors"</option>
                        <option value="Mission">"Missions"</option>
                        <option value="Station">"Stations"</option>
                    </select>
                    <button
                        class="px-3 py-1 bg-slate-700 hover:bg-slate-600 text-sm rounded"
                        on:click=move |_| fetch_state()
                    >
                        "Refresh"
                    </button>
                </div>
            </div>

            // Error display
            <Show when=move || error.get().is_some()>
                <div class="mb-4 p-2 bg-red-900/50 border border-red-500 rounded text-red-300 text-sm">
                    {move || error.get().unwrap_or_default()}
                </div>
            </Show>

            // Stats bar
            <div class="mb-4 flex items-center gap-4 text-sm text-slate-400">
                <span>"Total: "{move || total_count.get()}</span>
                <span>"Showing: "{move || entities.get().len()}</span>
            </div>

            // Entity list
            <div class="flex-1 overflow-auto">
                <Show
                    when=move || loading.get()
                    fallback=move || view! {
                        <EntityList entities=entities />
                    }
                >
                    <div class="flex items-center justify-center h-32 text-slate-400">
                        "Loading..."
                    </div>
                </Show>
            </div>

            // Pagination
            <div class="mt-4 flex items-center justify-between">
                <button
                    class="px-3 py-1 bg-slate-700 hover:bg-slate-600 text-sm rounded disabled:opacity-50"
                    disabled=move || offset.get() == 0
                    on:click=move |_| {
                        let new_offset = offset.get().saturating_sub(limit);
                        offset.set(new_offset);
                    }
                >
                    "Previous"
                </button>
                <span class="text-sm text-slate-400">
                    {move || format!("{}-{} of {}", offset.get() + 1, (offset.get() + entities.get().len()).min(total_count.get()), total_count.get())}
                </span>
                <button
                    class="px-3 py-1 bg-slate-700 hover:bg-slate-600 text-sm rounded disabled:opacity-50"
                    disabled=move || offset.get() + limit >= total_count.get()
                    on:click=move |_| {
                        offset.set(offset.get() + limit);
                    }
                >
                    "Next"
                </button>
            </div>
        </div>
    }
}

/// Entity list display
#[component]
fn EntityList(entities: RwSignal<Vec<serde_json::Value>>) -> impl IntoView {
    view! {
        <div class="space-y-2">
            <For
                each=move || entities.get().into_iter().enumerate()
                key=|(i, _)| *i
                children=move |(_, entity)| {
                    view! {
                        <EntityCard entity=entity />
                    }
                }
            />
        </div>
    }
}

/// Single entity card
#[component]
fn EntityCard(entity: serde_json::Value) -> impl IntoView {
    let expanded = RwSignal::new(false);

    // Extract common fields
    let id = entity.get("id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
    let name = entity.get("name")
        .or_else(|| entity.get("username"))
        .or_else(|| entity.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unnamed")
        .to_string();

    let entity_json = serde_json::to_string_pretty(&entity).unwrap_or_default();

    view! {
        <div class="bg-slate-800/50 border border-slate-700 rounded overflow-hidden">
            <button
                class="w-full px-3 py-2 flex items-center justify-between hover:bg-slate-700/50 transition-colors"
                on:click=move |_| expanded.set(!expanded.get())
            >
                <div class="flex items-center gap-3">
                    <span class="text-amber-400 font-mono text-xs">{id.chars().take(8).collect::<String>()}"..."</span>
                    <span class="font-medium">{name}</span>
                </div>
                <span class="text-slate-400">
                    {move || if expanded.get() { "▼" } else { "▶" }}
                </span>
            </button>
            <Show when=move || expanded.get()>
                <div class="px-3 py-2 border-t border-slate-700 bg-slate-900/50">
                    <pre class="text-xs font-mono text-slate-300 overflow-x-auto whitespace-pre-wrap">
                        {entity_json.clone()}
                    </pre>
                </div>
            </Show>
        </div>
    }
}
