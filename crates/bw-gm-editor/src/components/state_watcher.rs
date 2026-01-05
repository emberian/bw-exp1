//! State watcher component for tracking expressions over time
#![allow(dead_code)] // Leptos component props appear unused to clippy

use leptos::prelude::*;
use bw_shared::dto::WatchDto;

use crate::api::admin_ws;
use crate::state::InspectorState;

/// State watcher component for creating and monitoring watch expressions
#[component]
pub fn StateWatcher() -> impl IntoView {
    // Get state from context
    let inspector_state = expect_context::<InspectorState>();

    // New watch form state (local)
    let new_expression = RwSignal::new(String::new());
    let new_name = RwSignal::new(String::new());
    let creating = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);

    // Load watches on mount
    Effect::new(move |_| {
        admin_ws::with_admin_ws(|ws| ws.list_watches());
    });

    let on_create_watch = move |_| {
        let expr = new_expression.get();
        if expr.is_empty() {
            error.set(Some("Expression is required".to_string()));
            return;
        }

        creating.set(true);
        error.set(None);

        let name = if new_name.get().is_empty() {
            None
        } else {
            Some(new_name.get())
        };

        admin_ws::with_admin_ws(|ws| ws.create_watch(&expr, name));

        // Clear form
        new_expression.set(String::new());
        new_name.set(String::new());
        creating.set(false);
    };

    view! {
        <div class="h-full flex flex-col p-4">
            // Header
            <div class="flex items-center justify-between mb-4">
                <h2 class="text-lg font-semibold text-amber-500">"Watch Expressions"</h2>
                <button
                    class="px-3 py-1 bg-slate-700 hover:bg-slate-600 text-sm rounded"
                    on:click=move |_| {
                        admin_ws::with_admin_ws(|ws| ws.list_watches());
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
                        "x"
                    </button>
                </div>
            </Show>

            <div class="flex gap-4 flex-1 overflow-hidden">
                // Create watch form
                <div class="w-72 flex-shrink-0 bg-slate-800/50 rounded p-4">
                    <h3 class="text-sm font-medium text-slate-300 mb-4">"New Watch"</h3>

                    <div class="space-y-4">
                        // Expression input
                        <div>
                            <label class="block text-xs text-slate-400 mb-1">"Expression"</label>
                            <input
                                type="text"
                                class="w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded text-sm font-mono"
                                placeholder="ships[uuid].hull"
                                prop:value=move || new_expression.get()
                                on:input=move |ev| new_expression.set(event_target_value(&ev))
                            />
                            <p class="text-xs text-slate-500 mt-1">
                                "Rhai expression to evaluate each tick"
                            </p>
                        </div>

                        // Name input (optional)
                        <div>
                            <label class="block text-xs text-slate-400 mb-1">"Name (optional)"</label>
                            <input
                                type="text"
                                class="w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded text-sm"
                                placeholder="Player Hull"
                                prop:value=move || new_name.get()
                                on:input=move |ev| new_name.set(event_target_value(&ev))
                            />
                        </div>

                        // Create button
                        <button
                            class="w-full px-4 py-2 bg-amber-600 hover:bg-amber-500 text-white rounded disabled:opacity-50"
                            disabled=move || creating.get() || new_expression.get().is_empty()
                            on:click=on_create_watch
                        >
                            {move || if creating.get() { "Creating..." } else { "Add Watch" }}
                        </button>
                    </div>

                    // Example expressions
                    <div class="mt-6">
                        <h4 class="text-xs font-medium text-slate-400 mb-2">"Example Expressions"</h4>
                        <div class="space-y-1 text-xs font-mono text-slate-500">
                            <div>"ships.len()"</div>
                            <div>"players[id].credits"</div>
                            <div>"sectors[\"sol\"].ship_ids.len()"</div>
                            <div>"missions.filter(|m| m.status == \"active\").len()"</div>
                        </div>
                    </div>
                </div>

                // Watch list
                <div class="flex-1 overflow-auto">
                    <h3 class="text-sm font-medium text-slate-300 mb-4">"Active Watches"</h3>
                    <WatchList watches=inspector_state.watches />
                </div>
            </div>
        </div>
    }
}

/// Watch list display
#[allow(unused)] // Clippy false positive - props used via macro expansion
#[component]
fn WatchList(watches: RwSignal<Vec<WatchDto>>) -> impl IntoView {
    view! {
        <Show
            when=move || !watches.get().is_empty()
            fallback=|| view! {
                <div class="text-center text-slate-400 py-8">
                    "No watches yet. Create one to monitor game state."
                </div>
            }
        >
            <div class="space-y-2">
                <For
                    each=move || watches.get()
                    key=|w| w.id
                    children=move |watch| {
                        view! {
                            <WatchCard watch=watch />
                        }
                    }
                />
            </div>
        </Show>
    }
}

/// Single watch card
#[allow(unused)] // Clippy false positive - props used via macro expansion
#[component]
fn WatchCard(watch: WatchDto) -> impl IntoView {
    let watch_id = watch.id;
    let expanded = RwSignal::new(false);

    // Determine value display
    let (value_display, value_class) = if let Some(val) = &watch.last_value {
        // Convert serde_json::Value to string representation
        let display = match val {
            serde_json::Value::String(s) => format!("\"{}\"", s),
            serde_json::Value::Null => "null".to_string(),
            other => other.to_string(),
        };
        (display, "text-green-400")
    } else {
        ("No value".to_string(), "text-slate-500")
    };

    let has_name = watch.name.is_some();
    let display_name = watch.name.clone().unwrap_or_else(|| watch.expression.clone());
    let expression = watch.expression.clone();

    view! {
        <div class="bg-slate-800/50 border border-slate-700 rounded overflow-hidden">
            <div class="px-3 py-2 flex items-center justify-between">
                <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-2">
                        <span class="font-medium truncate">
                            {display_name}
                        </span>
                        <Show when=move || has_name>
                            <span class="text-xs text-slate-500 font-mono truncate">
                                {expression.clone()}
                            </span>
                        </Show>
                    </div>
                    <div class=format!("text-sm font-mono mt-1 {}", value_class)>
                        {value_display}
                    </div>
                </div>
                <div class="flex items-center gap-2 ml-4">
                    <button
                        class="px-2 py-1 text-xs text-slate-400 hover:text-slate-300"
                        on:click=move |_| expanded.set(!expanded.get())
                    >
                        {move || if expanded.get() { "Hide" } else { "History" }}
                    </button>
                    <button
                        class="px-2 py-1 bg-red-900/50 hover:bg-red-900 text-red-400 text-xs rounded"
                        on:click=move |_| {
                            admin_ws::with_admin_ws(|ws| ws.remove_watch(watch_id));
                        }
                    >
                        "Remove"
                    </button>
                </div>
            </div>

            // Expanded history view
            <Show when=move || expanded.get()>
                <div class="px-3 py-2 border-t border-slate-700 bg-slate-900/50">
                    <div class="text-xs text-slate-400 mb-2">
                        "History: "{watch.history_length}" samples"
                    </div>
                    // Simple sparkline placeholder - could be enhanced with actual chart
                    <div class="h-12 bg-slate-800 rounded flex items-center justify-center text-xs text-slate-500">
                        "Value history visualization (TODO)"
                    </div>
                </div>
            </Show>
        </div>
    }
}
