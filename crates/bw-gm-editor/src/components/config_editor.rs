//! Config editor component

use leptos::prelude::*;

use crate::api::with_admin_ws;
use crate::state::GMEditorState;

/// Simulation config editor
#[component]
pub fn ConfigEditor() -> impl IntoView {
    let gm_state = expect_context::<GMEditorState>();

    // Load config on mount
    Effect::new(move |_| {
        gm_state.loading_config.set(true);
        with_admin_ws(|ws| ws.get_sim_config());
    });

    view! {
        <div class="config-editor p-4 overflow-y-auto">
            <h2 class="text-lg font-semibold text-slate-200 mb-4">"Simulation Configuration"</h2>

            <Show
                when=move || !gm_state.loading_config.get()
                fallback=|| view! { <div class="text-slate-400">"Loading configuration..."</div> }
            >
                {move || {
                    match gm_state.sim_config.get() {
                        Some(config) => view! {
                            <div class="space-y-6">
                                <ConfigSection title="Raw Config (JSON)">
                                    <pre class="bg-slate-800 p-3 rounded text-sm font-mono text-slate-300 overflow-x-auto">
                                        {serde_json::to_string_pretty(&config).unwrap_or_else(|_| "Error".to_string())}
                                    </pre>
                                </ConfigSection>
                            </div>
                        }.into_any(),
                        None => view! {
                            <div class="text-slate-400">"No configuration loaded"</div>
                        }.into_any(),
                    }
                }}
            </Show>
        </div>
    }
}

/// A collapsible config section
#[component]
fn ConfigSection(
    title: &'static str,
    children: Children,
) -> impl IntoView {
    let expanded = RwSignal::new(true);
    // Render children once, then show/hide with CSS
    let children_view = children();

    view! {
        <div class="border border-slate-700 rounded">
            <button
                class="w-full flex items-center justify-between px-4 py-2 bg-slate-800/50 hover:bg-slate-700/50"
                on:click=move |_| expanded.update(|e| *e = !*e)
            >
                <span class="font-medium text-slate-200">{title}</span>
                <span class="text-slate-400">
                    {move || if expanded.get() { "▼" } else { "▶" }}
                </span>
            </button>
            <div class="p-4" class:hidden=move || !expanded.get()>
                {children_view}
            </div>
        </div>
    }
}
