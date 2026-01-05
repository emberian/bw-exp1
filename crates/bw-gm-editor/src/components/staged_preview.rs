//! Staged changes preview component

use leptos::prelude::*;

use crate::api::with_admin_ws;
use crate::state::StagedChangesState;

/// Preview and commit staged changes
#[component]
pub fn StagedPreview() -> impl IntoView {
    let staged = expect_context::<StagedChangesState>();

    let has_changes = move || !staged.changes.get().is_empty();
    let has_errors = move || staged.has_errors();

    view! {
        <div class="staged-preview p-4">
            // Header with actions
            <div class="flex items-center justify-between mb-4">
                <h2 class="text-lg font-semibold text-slate-200">
                    "Staged Changes "
                    <span class="text-slate-400">
                        "("{move || staged.changes.get().len()}")"
                    </span>
                </h2>
                <div class="flex gap-2">
                    <button
                        class="px-3 py-1 bg-slate-600 hover:bg-slate-500 rounded text-sm disabled:opacity-50 disabled:cursor-not-allowed"
                        disabled=move || !has_changes() || staged.preview_loading.get()
                        on:click=move |_| {
                            staged.preview_loading.set(true);
                            let changes = staged.get_changes();
                            with_admin_ws(|ws| ws.preview_staged(changes));
                        }
                    >
                        {move || if staged.preview_loading.get() { "Loading..." } else { "Refresh Preview" }}
                    </button>
                    <button
                        class="px-3 py-1 bg-red-600 hover:bg-red-500 rounded text-sm disabled:opacity-50 disabled:cursor-not-allowed"
                        disabled=move || !has_changes()
                        on:click=move |_| staged.clear()
                    >
                        "Discard All"
                    </button>
                    <button
                        class="px-3 py-1 bg-green-600 hover:bg-green-500 rounded text-sm disabled:opacity-50 disabled:cursor-not-allowed"
                        disabled=move || !has_changes() || has_errors() || staged.commit_loading.get()
                        on:click=move |_| {
                            staged.commit_loading.set(true);
                            let changes = staged.get_changes();
                            with_admin_ws(|ws| ws.commit_staged(changes));
                        }
                    >
                        {move || if staged.commit_loading.get() { "Committing..." } else { "Commit All" }}
                    </button>
                </div>
            </div>

            // Last result message
            <Show when=move || staged.last_result.get().is_some()>
                <div class="mb-4 p-3 bg-slate-800 border border-slate-600 rounded text-sm">
                    {move || staged.last_result.get().unwrap_or_default()}
                </div>
            </Show>

            // Change list
            <Show
                when=has_changes
                fallback=|| view! {
                    <div class="text-slate-400 text-center py-8">
                        "No staged changes. Make edits in the Scripts, Config, or Entities tabs."
                    </div>
                }
            >
                <div class="space-y-2">
                    <For
                        each=move || staged.changes.get()
                        key=|c| c.id
                        children=move |entry| {
                            let id = entry.id;
                            let border_color = if entry.has_error {
                                "border-red-500"
                            } else {
                                "border-slate-600"
                            };

                            view! {
                                <div class=format!("p-3 border rounded bg-slate-800 {}", border_color)>
                                    <div class="flex items-center justify-between">
                                        <span class="font-medium text-slate-200">
                                            {entry.description()}
                                        </span>
                                        <button
                                            class="text-red-400 hover:text-red-300 text-sm"
                                            on:click=move |_| staged.remove(id)
                                        >
                                            "Remove"
                                        </button>
                                    </div>

                                    // Preview diff
                                    {entry.preview.as_ref().map(|p| view! {
                                        <div class="mt-2 text-xs font-mono">
                                            {p.before.as_ref().map(|before| view! {
                                                <div class="text-red-400 truncate">
                                                    "- "{serde_json::to_string(before).unwrap_or_default()}
                                                </div>
                                            })}
                                            <div class="text-green-400 truncate">
                                                "+ "{serde_json::to_string(&p.after).unwrap_or_default()}
                                            </div>
                                        </div>
                                    })}

                                    // Error message
                                    {entry.error_message.as_ref().map(|err| view! {
                                        <div class="mt-2 text-red-400 text-sm">
                                            {err.clone()}
                                        </div>
                                    })}
                                </div>
                            }
                        }
                    />
                </div>
            </Show>
        </div>
    }
}
