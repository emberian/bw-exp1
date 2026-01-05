//! Main GM Editor application component

use leptos::prelude::*;

use crate::api::{init_admin_ws, shutdown_admin_ws};
use crate::components::{
    ConfigEditor, DebugPanel, EntityBrowser, ScriptEditor, StagedPreview, TabBar, Tab,
};
use crate::state::{DebugPanelState, GMEditorState, StagedChangesState};

/// Root component for the GM Editor
#[component]
pub fn GMEditorApp(
    ws_url: String,
    auth_token: String,
) -> impl IntoView {
    // Create state
    let gm_state = GMEditorState::new();
    let staged = StagedChangesState::new();
    let debug_state = DebugPanelState::new();

    // Initialize WebSocket client (thread_local storage)
    init_admin_ws(&ws_url, &auth_token);

    // Cleanup on unmount
    on_cleanup(|| {
        shutdown_admin_ws();
    });

    // Provide contexts (state only, not WS client)
    provide_context(gm_state);
    provide_context(staged);
    provide_context(debug_state);

    // Active tab
    let active_tab = RwSignal::new(Tab::Scripts);

    // Panel visibility
    let is_visible = RwSignal::new(true);

    view! {
        <div
            class="gm-editor fixed inset-4 z-[200] bg-slate-900/95 backdrop-blur-sm border border-amber-500/30 rounded-lg shadow-2xl flex flex-col"
            class:hidden=move || !is_visible.get()
        >
            // Header
            <div class="flex items-center justify-between px-4 py-2 border-b border-slate-700 bg-slate-800/50">
                <div class="flex items-center gap-3">
                    <span class="text-amber-500 font-bold text-lg">"GM Editor"</span>
                    <span class="text-slate-400 text-sm">"Game Master Tools"</span>
                </div>
                <div class="flex items-center gap-2">
                    <StagedChangesIndicator />
                    <button
                        class="px-2 py-1 text-slate-400 hover:text-white hover:bg-slate-700 rounded"
                        on:click=move |_| is_visible.set(false)
                    >
                        "Minimize"
                    </button>
                    <button
                        class="px-2 py-1 text-red-400 hover:text-red-300 hover:bg-slate-700 rounded"
                        on:click=move |_| {
                            if let Err(e) = crate::unmount_gm_editor("gm-editor-container") {
                                tracing::error!("Failed to unmount: {:?}", e);
                            }
                        }
                    >
                        "Close"
                    </button>
                </div>
            </div>

            // Tab bar
            <TabBar active_tab=active_tab />

            // Content area
            <div class="flex-1 overflow-hidden flex">
                // Main content
                <div class="flex-1 overflow-auto">
                    {move || match active_tab.get() {
                        Tab::Scripts => view! { <ScriptEditor /> }.into_any(),
                        Tab::Config => view! { <ConfigEditor /> }.into_any(),
                        Tab::Entities => view! { <EntityBrowser /> }.into_any(),
                        Tab::Debug => view! { <DebugPanel /> }.into_any(),
                        Tab::Staged => view! { <StagedPreview /> }.into_any(),
                    }}
                </div>
            </div>
        </div>

        // Minimized indicator
        <Show when=move || !is_visible.get()>
            <button
                class="fixed bottom-4 right-4 z-[200] px-4 py-2 bg-amber-600 hover:bg-amber-500 text-white rounded-lg shadow-lg flex items-center gap-2"
                on:click=move |_| is_visible.set(true)
            >
                <span>"GM Editor"</span>
                <StagedChangesIndicator />
            </button>
        </Show>
    }
}

/// Shows count of staged changes
#[component]
fn StagedChangesIndicator() -> impl IntoView {
    let staged = expect_context::<StagedChangesState>();
    let count = move || staged.changes.get().len();
    let has_changes = move || !staged.changes.get().is_empty();

    view! {
        <Show when=has_changes>
            <span class="px-2 py-0.5 bg-amber-500 text-slate-900 text-xs font-bold rounded-full">
                {count}
            </span>
        </Show>
    }
}
