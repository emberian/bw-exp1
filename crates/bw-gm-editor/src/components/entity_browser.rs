//! Entity browser component

use leptos::prelude::*;

use bw_shared::EntityType;

use crate::api::with_admin_ws;
use crate::state::GMEditorState;

/// Entity browser with search and filtering
#[component]
pub fn EntityBrowser() -> impl IntoView {
    let gm_state = expect_context::<GMEditorState>();

    // Selected entity type
    let entity_type = RwSignal::new(EntityType::Ship);

    // Search query
    let search = RwSignal::new(String::new());

    // Load entities when type changes
    Effect::new(move |_| {
        let et = entity_type.get();
        gm_state.loading_entities.set(true);
        with_admin_ws(|ws| ws.query_entities(et, vec![], 100, 0));
    });

    view! {
        <div class="entity-browser flex h-full">
            // Left: List
            <div class="w-80 border-r border-slate-700 flex flex-col">
                // Filters
                <div class="p-2 border-b border-slate-700 space-y-2">
                    <select
                        class="w-full bg-slate-800 border border-slate-600 rounded px-2 py-1 text-sm text-slate-200"
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            let et = match value.as_str() {
                                "Ship" => EntityType::Ship,
                                "Player" => EntityType::Player,
                                "Mission" => EntityType::Mission,
                                "Station" => EntityType::Station,
                                "Sector" => EntityType::Sector,
                                _ => EntityType::Ship,
                            };
                            entity_type.set(et);
                        }
                    >
                        <option value="Ship">"Ships"</option>
                        <option value="Player">"Players"</option>
                        <option value="Mission">"Missions"</option>
                        <option value="Station">"Stations"</option>
                        <option value="Sector">"Sectors"</option>
                    </select>

                    <input
                        type="text"
                        class="w-full bg-slate-800 border border-slate-600 rounded px-2 py-1 text-sm text-slate-200 placeholder-slate-500"
                        placeholder="Search..."
                        prop:value=move || search.get()
                        on:input=move |ev| search.set(event_target_value(&ev))
                    />
                </div>

                // Entity list
                <div class="flex-1 overflow-y-auto">
                    <Show
                        when=move || !gm_state.loading_entities.get()
                        fallback=|| view! { <div class="p-2 text-slate-400 text-sm">"Loading..."</div> }
                    >
                        <For
                            each=move || {
                                let search_lower = search.get().to_lowercase();
                                gm_state.entities.get()
                                    .into_iter()
                                    .filter(|e| search_lower.is_empty() || e.name.to_lowercase().contains(&search_lower))
                                    .collect::<Vec<_>>()
                            }
                            key=|e| e.id
                            children=move |entity| {
                                let id = entity.id;
                                let is_selected = move || gm_state.selected_entity.get() == Some(id);
                                view! {
                                    <button
                                        class="w-full text-left px-3 py-2 border-b border-slate-700 hover:bg-slate-700"
                                        class:bg-slate-700=is_selected
                                        on:click=move |_| {
                                            gm_state.selected_entity.set(Some(id));
                                            let et = entity_type.get();
                                            with_admin_ws(|ws| ws.get_entity(et, id));
                                        }
                                    >
                                        <div class="font-medium text-slate-200 truncate">{entity.name.clone()}</div>
                                        <div class="text-xs text-slate-400">{entity.status.clone()}</div>
                                    </button>
                                }
                            }
                        />
                    </Show>
                </div>
            </div>

            // Right: Details
            <div class="flex-1 p-4 overflow-y-auto">
                {move || {
                    match gm_state.entity_details.get() {
                        Some(details) => view! {
                            <div>
                                <h3 class="text-lg font-semibold text-slate-200 mb-4">"Entity Details"</h3>
                                <pre class="bg-slate-800 p-3 rounded text-sm font-mono text-slate-300 overflow-x-auto">
                                    {serde_json::to_string_pretty(&details).unwrap_or_else(|_| "Error".to_string())}
                                </pre>
                            </div>
                        }.into_any(),
                        None => view! {
                            <div class="text-slate-400">"Select an entity to view details"</div>
                        }.into_any(),
                    }
                }}
            </div>
        </div>
    }
}
