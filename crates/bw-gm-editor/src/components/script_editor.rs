//! Script editor component with CodeMirror integration

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use leptos::prelude::*;
use wasm_bindgen::JsCast;

use bw_shared::StagedChange;

use crate::api::with_admin_ws;
use crate::bindings::{init_codemirror, update_codemirror_content};
use crate::state::{GMEditorState, StagedChangesState};

/// Script editor with file browser and CodeMirror
#[component]
pub fn ScriptEditor() -> impl IntoView {
    let gm_state = expect_context::<GMEditorState>();
    let staged = expect_context::<StagedChangesState>();

    // Load scripts on mount
    Effect::new(move |_| {
        gm_state.loading_scripts.set(true);
        with_admin_ws(|ws| ws.list_scripts());
    });

    let is_modified = move || gm_state.script_is_modified();

    // Reference to the CodeMirror container
    let editor_ref = NodeRef::<leptos::html::Div>::new();

    // Track if CodeMirror has been initialized
    let cm_initialized = Arc::new(AtomicBool::new(false));

    // Initialize CodeMirror when content changes from server
    Effect::new({
        let cm_initialized = cm_initialized.clone();
        move |_| {
            // Trigger on script_original changes (set when file is loaded)
            let content = gm_state.script_original.get();

            if let Some(el) = editor_ref.get() {
                if let Some(html_el) = el.dyn_ref::<web_sys::HtmlElement>() {
                    if !cm_initialized.load(Ordering::SeqCst) && !content.is_empty() {
                        // First initialization
                        let on_change = move |new_content: String| {
                            gm_state.script_content.set(new_content);
                        };
                        init_codemirror(html_el, &content, on_change);
                        cm_initialized.store(true, Ordering::SeqCst);
                    } else if cm_initialized.load(Ordering::SeqCst) {
                        // Update existing editor
                        update_codemirror_content(html_el, &content);
                    }
                }
            }
        }
    });

    view! {
        <div class="script-editor flex h-full">
            // File browser sidebar
            <div class="w-64 border-r border-slate-700 flex flex-col">
                <div class="p-2 border-b border-slate-700 text-sm font-medium text-slate-300">
                    "Scripts"
                </div>
                <div class="flex-1 overflow-y-auto">
                    <Show
                        when=move || !gm_state.loading_scripts.get()
                        fallback=|| view! { <div class="p-2 text-slate-400 text-sm">"Loading..."</div> }
                    >
                        <For
                            each=move || gm_state.scripts.get()
                            key=|f| f.path.clone()
                            children=move |file| {
                                let path = file.path.clone();
                                let path_click = path.clone();
                                let is_selected = Memo::new(move |_| {
                                    gm_state.selected_script.get().as_ref() == Some(&path)
                                });
                                view! {
                                    <button
                                        class="w-full text-left px-3 py-1.5 text-sm truncate hover:bg-slate-700"
                                        class:bg-slate-700=move || is_selected.get()
                                        class:text-amber-500=move || is_selected.get()
                                        class:text-slate-300=move || !is_selected.get()
                                        on:click=move |_| {
                                            gm_state.loading_scripts.set(true);
                                            let path = path_click.clone();
                                            with_admin_ws(|ws| ws.read_script(&path));
                                        }
                                    >
                                        {file.name}
                                    </button>
                                }
                            }
                        />
                    </Show>
                </div>
            </div>

            // Editor area
            <div class="flex-1 flex flex-col">
                // Toolbar
                <div class="h-10 border-b border-slate-700 flex items-center px-4 gap-2">
                    <span class="text-sm text-slate-400 flex-1 truncate">
                        {move || gm_state.selected_script.get().unwrap_or_else(|| "No file selected".to_string())}
                    </span>
                    <Show when=is_modified>
                        <span class="text-amber-400 text-xs">"(modified)"</span>
                    </Show>
                    <button
                        class="px-3 py-1 bg-amber-600 hover:bg-amber-500 rounded text-sm disabled:opacity-50 disabled:cursor-not-allowed"
                        disabled=move || !is_modified()
                        on:click=move |_| {
                            if let Some(path) = gm_state.selected_script.get() {
                                staged.add(StagedChange::ScriptUpdate {
                                    path,
                                    content: gm_state.script_content.get(),
                                });
                                // Mark as staged (update original)
                                gm_state.script_original.set(gm_state.script_content.get());
                            }
                        }
                    >
                        "Stage Change"
                    </button>
                </div>

                // CodeMirror container
                <div class="flex-1 overflow-hidden">
                    <Show
                        when=move || gm_state.selected_script.get().is_some()
                        fallback=|| view! {
                            <div class="h-full flex items-center justify-center text-slate-500">
                                "Select a script file to edit"
                            </div>
                        }
                    >
                        <div
                            node_ref=editor_ref
                            class="h-full overflow-auto codemirror-container"
                        />
                    </Show>
                </div>
            </div>
        </div>
    }
}
