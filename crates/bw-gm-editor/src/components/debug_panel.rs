//! Debug panel component for script debugging
//!
//! Provides UI for:
//! - Starting/stopping debug sessions
//! - Execution control (continue, step, pause)
//! - Call stack visualization
//! - Variable inspection
//! - Breakpoint management
//! - Script error tracking

use leptos::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

use bw_shared::dto::{DebugTargetDto, PauseReasonDto};

use crate::api::with_admin_ws;
use crate::components::icons::{CloseIcon, PauseIcon, PlayIcon, StepIntoIcon, StepOutIcon, StepOverIcon};
use crate::state::{DebugPanelState, GMEditorState, ScriptErrorEntry};

/// Main debug panel component
#[component]
pub fn DebugPanel() -> impl IntoView {
    let debug_state = expect_context::<DebugPanelState>();
    let gm_state = use_context::<GMEditorState>();

    view! {
        <div class="debug-panel h-full flex flex-col bg-slate-800">
            // Header with session controls
            <DebugHeader />

            // Main content area when session is active
            <Show
                when=move || debug_state.session_id.get().is_some()
                fallback=move || view! {
                    <div class="flex-1 flex flex-col">
                        // Show script errors even when no session
                        {move || gm_state.map(|state| view! {
                            <ScriptErrorsView errors=state.script_errors />
                        })}

                        // Placeholder when no session
                        <div class="flex-1 flex items-center justify-center text-slate-500">
                            <div class="text-center">
                                <div class="text-lg mb-2">"No debug session active"</div>
                                <div class="text-sm">"Start a debug session to begin debugging scripts"</div>
                            </div>
                        </div>
                    </div>
                }
            >
                // Execution controls
                <ExecutionControls />

                // Pause indicator
                <Show when=move || debug_state.is_paused.get()>
                    <PauseIndicator />
                </Show>

                // Scrollable content
                <div class="flex-1 overflow-y-auto">
                    // Entity context (when paused)
                    <Show when=move || debug_state.entity_context.get().is_some()>
                        <EntityContextView />
                    </Show>

                    // Call stack
                    <CallStackView />

                    // Variables
                    <VariablesView />

                    // Breakpoints
                    <BreakpointListView />

                    // Script errors
                    {move || gm_state.map(|state| view! {
                        <ScriptErrorsView errors=state.script_errors />
                    })}
                </div>
            </Show>

            // Error display
            <Show when=move || debug_state.error.get().is_some()>
                <div class="px-3 py-2 bg-red-900/50 border-t border-red-700 text-red-300 text-sm">
                    {move || debug_state.error.get().unwrap_or_default()}
                </div>
            </Show>
        </div>
    }
}

/// Header with session controls
#[component]
fn DebugHeader() -> impl IntoView {
    let debug_state = expect_context::<DebugPanelState>();

    let is_live = move || {
        matches!(debug_state.target.get(), Some(DebugTargetDto::Live))
    };

    let start_live_session = move |_| {
        debug_state.connecting.set(true);
        debug_state.target.set(Some(DebugTargetDto::Live));
        with_admin_ws(|ws| ws.start_debug_session(DebugTargetDto::Live));
    };

    let end_session = move |_| {
        with_admin_ws(|ws| ws.end_debug_session());
    };

    view! {
        <div class="p-3 border-b border-slate-700">
            <div class="flex items-center gap-2">
                <div class="flex-1">
                    <div class="text-sm font-medium text-slate-200">"Script Debugger"</div>
                    <div class="text-xs text-slate-400">
                        {move || {
                            if debug_state.connecting.get() {
                                "Connecting...".to_string()
                            } else if let Some(session_id) = debug_state.session_id.get() {
                                format!("Session: {}", &session_id.to_string()[..8])
                            } else {
                                "Not connected".to_string()
                            }
                        }}
                    </div>
                </div>

                <Show
                    when=move || debug_state.session_id.get().is_none()
                    fallback=move || view! {
                        <button
                            class="px-3 py-1 bg-red-600 hover:bg-red-500 rounded text-sm"
                            on:click=end_session
                        >
                            "End Session"
                        </button>
                    }
                >
                    <button
                        class="px-3 py-1 bg-amber-600 hover:bg-amber-500 rounded text-sm disabled:opacity-50"
                        disabled=move || debug_state.connecting.get()
                        on:click=start_live_session
                    >
                        "Start Debug"
                    </button>
                </Show>
            </div>

            // Live server warning
            <Show when=is_live>
                <div class="mt-2 px-2 py-1.5 bg-red-900/40 border border-red-700/50 rounded text-xs text-red-300 flex items-center gap-2">
                    <span class="text-red-400 font-bold">"!"</span>
                    <span>"Debugging LIVE server - paused scripts affect all players"</span>
                </div>
            </Show>
        </div>
    }
}

/// Execution controls (continue, step, pause)
#[component]
fn ExecutionControls() -> impl IntoView {
    let debug_state = expect_context::<DebugPanelState>();

    let is_paused = move || debug_state.is_paused.get();

    let on_continue = move |_| {
        with_admin_ws(|ws| ws.debug_continue());
    };

    let on_pause = move |_| {
        with_admin_ws(|ws| ws.debug_pause());
    };

    let on_step_into = move |_| {
        with_admin_ws(|ws| ws.debug_step_into());
    };

    let on_step_over = move |_| {
        with_admin_ws(|ws| ws.debug_step_over());
    };

    let on_step_out = move |_| {
        with_admin_ws(|ws| ws.debug_step_out());
    };

    view! {
        <div class="px-3 py-2 border-b border-slate-700 flex items-center gap-1">
            // Continue/Pause toggle
            <Show
                when=is_paused
                fallback=move || view! {
                    <button
                        class="p-2 hover:bg-slate-700 rounded text-slate-300 hover:text-slate-100"
                        title="Pause"
                        on:click=on_pause
                    >
                        <PauseIcon />
                    </button>
                }
            >
                <button
                    class="p-2 hover:bg-slate-700 rounded text-green-400 hover:text-green-300"
                    title="Continue (F5)"
                    on:click=on_continue
                >
                    <PlayIcon />
                </button>
            </Show>

            <div class="w-px h-6 bg-slate-600 mx-1"></div>

            // Step controls (only enabled when paused)
            <button
                class="p-2 hover:bg-slate-700 rounded disabled:opacity-50 disabled:cursor-not-allowed text-slate-300 hover:text-slate-100"
                title="Step Into (F11)"
                disabled=move || !is_paused()
                on:click=on_step_into
            >
                <StepIntoIcon />
            </button>

            <button
                class="p-2 hover:bg-slate-700 rounded disabled:opacity-50 disabled:cursor-not-allowed text-slate-300 hover:text-slate-100"
                title="Step Over (F10)"
                disabled=move || !is_paused()
                on:click=on_step_over
            >
                <StepOverIcon />
            </button>

            <button
                class="p-2 hover:bg-slate-700 rounded disabled:opacity-50 disabled:cursor-not-allowed text-slate-300 hover:text-slate-100"
                title="Step Out (Shift+F11)"
                disabled=move || !is_paused()
                on:click=on_step_out
            >
                <StepOutIcon />
            </button>
        </div>
    }
}

/// Pause indicator showing why execution stopped
#[component]
fn PauseIndicator() -> impl IntoView {
    let debug_state = expect_context::<DebugPanelState>();

    view! {
        <div class="px-3 py-2 bg-amber-900/30 border-b border-amber-700/50">
            <div class="flex items-center gap-2">
                <div class="w-2 h-2 rounded-full bg-amber-500 animate-pulse"></div>
                <span class="text-amber-300 text-sm font-medium">"Paused"</span>
            </div>
            <div class="mt-1 text-xs text-slate-400">
                {move || {
                    let script = debug_state.paused_script.get().unwrap_or_default();
                    let line = debug_state.paused_line.get().unwrap_or(0);
                    format!("{}:{}", script, line)
                }}
            </div>
            <div class="mt-1 text-xs text-slate-500">
                {move || format_pause_reason(debug_state.pause_reason.get())}
            </div>
        </div>
    }
}

/// Format pause reason for display
fn format_pause_reason(reason: Option<PauseReasonDto>) -> String {
    match reason {
        Some(PauseReasonDto::Breakpoint { .. }) => "Hit breakpoint".to_string(),
        Some(PauseReasonDto::FunctionEntry { function_name }) => format!("Entered function: {}", function_name),
        Some(PauseReasonDto::FunctionExit { function_name }) => format!("Exited function: {}", function_name),
        Some(PauseReasonDto::Step) => "Step completed".to_string(),
        Some(PauseReasonDto::Pause) => "Manual pause".to_string(),
        Some(PauseReasonDto::Exception { message }) => format!("Exception: {}", message),
        None => "Unknown".to_string(),
    }
}

/// Entity context view (what entity is being debugged)
#[component]
fn EntityContextView() -> impl IntoView {
    let debug_state = expect_context::<DebugPanelState>();

    view! {
        <div class="px-3 py-2 bg-slate-750 border-b border-slate-700">
            <div class="text-xs font-medium text-slate-400 mb-1">"Entity Context"</div>
            {move || {
                if let Some(ctx) = debug_state.entity_context.get() {
                    view! {
                        <div class="text-sm">
                            <div class="flex items-center gap-2">
                                <span class="text-slate-300">{ctx.entity_name}</span>
                                <span class="text-xs px-1.5 py-0.5 bg-slate-700 rounded text-slate-400">
                                    {ctx.entity_type}
                                </span>
                            </div>
                            <div class="text-xs text-slate-500 mt-0.5">
                                {format!("ID: {}", &ctx.entity_id.to_string()[..8])}
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <div class="text-sm text-slate-500">"No entity context"</div> }.into_any()
                }
            }}
        </div>
    }
}

/// Call stack view
#[component]
fn CallStackView() -> impl IntoView {
    let debug_state = expect_context::<DebugPanelState>();

    view! {
        <div class="border-b border-slate-700">
            <CollapsibleSection title="Call Stack" default_open=true>
                <Show
                    when=move || !debug_state.call_stack.get().is_empty()
                    fallback=|| view! {
                        <div class="px-3 py-2 text-slate-500 text-sm">"No call stack"</div>
                    }
                >
                    <div class="divide-y divide-slate-700/50">
                        <For
                            each=move || debug_state.call_stack.get()
                            key=|frame| frame.index
                            children=move |frame| {
                                let index = frame.index;
                                let is_selected = Memo::new(move |_| {
                                    debug_state.selected_frame.get() == index
                                });

                                view! {
                                    <button
                                        class="w-full text-left px-3 py-1.5 hover:bg-slate-700/50"
                                        class:bg-slate-700=move || is_selected.get()
                                        on:click=move |_| {
                                            debug_state.select_frame(index);
                                            with_admin_ws(|ws| ws.get_variables(index));
                                        }
                                    >
                                        <div class="flex items-center gap-2">
                                            <span class="text-slate-500 text-xs w-4">{frame.index}</span>
                                            <span class="text-amber-400 text-sm">{frame.function_name.clone()}</span>
                                        </div>
                                        {frame.source.clone().map(|src| {
                                            let line = frame.line.unwrap_or(0);
                                            view! {
                                                <div class="ml-6 text-xs text-slate-500">
                                                    {format!("{}:{}", src, line)}
                                                </div>
                                            }
                                        })}
                                    </button>
                                }
                            }
                        />
                    </div>
                </Show>
            </CollapsibleSection>
        </div>
    }
}

/// Variables view
#[component]
fn VariablesView() -> impl IntoView {
    let debug_state = expect_context::<DebugPanelState>();

    view! {
        <div class="border-b border-slate-700">
            <CollapsibleSection title="Variables" default_open=true>
                <Show
                    when=move || !debug_state.variables.get().is_empty()
                    fallback=|| view! {
                        <div class="px-3 py-2 text-slate-500 text-sm">"No variables"</div>
                    }
                >
                    <div class="divide-y divide-slate-700/50">
                        <For
                            each=move || debug_state.variables.get()
                            key=|var| var.path.clone()
                            children=move |var| {
                                view! {
                                    <VariableRow variable=var />
                                }
                            }
                        />
                    </div>
                </Show>
            </CollapsibleSection>
        </div>
    }
}

/// Single variable row with expansion support
#[component]
fn VariableRow(variable: bw_shared::dto::VariableDto) -> impl IntoView {
    let expanded = RwSignal::new(false);
    let path = variable.path.clone();

    let toggle_expand = move |_| {
        if variable.expandable {
            let new_val = !expanded.get();
            expanded.set(new_val);
            if new_val {
                let path = path.clone();
                with_admin_ws(|ws| ws.expand_variable(&path));
            }
        }
    };

    view! {
        <div class="px-3 py-1.5">
            <div
                class="flex items-center gap-2 cursor-pointer hover:bg-slate-700/30 -mx-2 px-2 py-0.5 rounded"
                on:click=toggle_expand
            >
                // Expand indicator
                <span class="w-4 text-slate-500 text-xs">
                    {if variable.expandable {
                        if expanded.get() { "v" } else { ">" }
                    } else {
                        ""
                    }}
                </span>

                // Variable name
                <span class="text-blue-400 text-sm">{variable.name.clone()}</span>

                // Type badge
                <span class="text-xs text-slate-500">{format!("({})", variable.type_name)}</span>

                // Value
                <span class="flex-1 text-right text-sm text-slate-300 truncate">
                    {variable.value.clone()}
                </span>
            </div>

            // TODO: Show children when expanded
            // This would require tracking expanded children in state
        </div>
    }
}

/// Breakpoint list view
#[component]
fn BreakpointListView() -> impl IntoView {
    let debug_state = expect_context::<DebugPanelState>();

    view! {
        <div class="border-b border-slate-700">
            <CollapsibleSection title="Breakpoints" default_open=true>
                <Show
                    when=move || !debug_state.breakpoints.get().is_empty() || !debug_state.function_breakpoints.get().is_empty()
                    fallback=|| view! {
                        <div class="px-3 py-2 text-slate-500 text-sm">"No breakpoints set"</div>
                    }
                >
                    <div class="divide-y divide-slate-700/50">
                        // Line breakpoints
                        <For
                            each=move || debug_state.breakpoints.get()
                            key=|bp| bp.id
                            children=move |bp| {
                                let bp_id = bp.id;
                                let enabled = bp.enabled;

                                view! {
                                    <BreakpointRow
                                        id=bp_id
                                        enabled=enabled
                                        label=format!("{}:{}", bp.script, bp.line)
                                        sublabel=bp.condition.clone()
                                        hit_count=bp.hit_count
                                    />
                                }
                            }
                        />

                        // Function breakpoints
                        <For
                            each=move || debug_state.function_breakpoints.get()
                            key=|fbp| fbp.id
                            children=move |fbp| {
                                let bp_id = fbp.id;
                                let enabled = fbp.enabled;

                                view! {
                                    <BreakpointRow
                                        id=bp_id
                                        enabled=enabled
                                        label=format!("fn {}", fbp.function_name)
                                        sublabel=None
                                        hit_count=0
                                    />
                                }
                            }
                        />
                    </div>
                </Show>
            </CollapsibleSection>
        </div>
    }
}

/// Single breakpoint row
#[component]
fn BreakpointRow(
    id: Uuid,
    enabled: bool,
    label: String,
    sublabel: Option<String>,
    hit_count: u64,
) -> impl IntoView {
    let toggle_enabled = move |_| {
        with_admin_ws(|ws| ws.toggle_breakpoint(id, !enabled));
    };

    let remove = move |_| {
        with_admin_ws(|ws| ws.remove_breakpoint(id));
    };

    view! {
        <div class="px-3 py-1.5 flex items-center gap-2 group">
            // Enable/disable checkbox
            <input
                type="checkbox"
                checked=enabled
                class="w-4 h-4 rounded border-slate-600 bg-slate-700 text-amber-500 focus:ring-amber-500 focus:ring-offset-slate-800"
                on:change=toggle_enabled
            />

            // Breakpoint label
            <div class="flex-1 min-w-0">
                <div class="text-sm text-slate-300 truncate">{label}</div>
                {sublabel.map(|s| view! {
                    <div class="text-xs text-slate-500 truncate">"if "{s}</div>
                })}
            </div>

            // Hit count
            {if hit_count > 0 {
                Some(view! {
                    <span class="text-xs text-slate-500">{format!("x{}", hit_count)}</span>
                })
            } else {
                None
            }}

            // Remove button
            <button
                class="p-1 hover:bg-slate-700 rounded opacity-0 group-hover:opacity-100 text-slate-400 hover:text-red-400"
                title="Remove breakpoint"
                on:click=remove
            >
                <CloseIcon />
            </button>
        </div>
    }
}

/// Collapsible section wrapper
#[component]
fn CollapsibleSection(
    title: &'static str,
    #[prop(default = false)] default_open: bool,
    children: ChildrenFn,
) -> impl IntoView {
    let is_open = RwSignal::new(default_open);

    view! {
        <div>
            <button
                class="w-full px-3 py-2 flex items-center gap-2 hover:bg-slate-700/50 text-left"
                on:click=move |_| is_open.update(|v| *v = !*v)
            >
                <span class="text-slate-500 text-xs transition-transform"
                    class:rotate-90=move || is_open.get()
                >
                    ">"
                </span>
                <span class="text-sm font-medium text-slate-300">{title}</span>
            </button>
            <Show when=move || is_open.get()>
                {children()}
            </Show>
        </div>
    }
}

// =============================================================================
// Script Errors View
// =============================================================================

/// Script errors grouped by script file
#[component]
fn ScriptErrorsView(errors: RwSignal<Vec<ScriptErrorEntry>>) -> impl IntoView {
    // Group errors by script
    let grouped_errors = Memo::new(move |_| {
        let mut groups: HashMap<String, Vec<ScriptErrorEntry>> = HashMap::new();
        for error in errors.get() {
            groups.entry(error.script.clone()).or_default().push(error);
        }
        // Sort by script name
        let mut sorted: Vec<_> = groups.into_iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        sorted
    });

    let total_count = move || errors.get().len();

    view! {
        <div class="border-b border-slate-700">
            <CollapsibleSection title="Script Errors" default_open=true>
                <Show
                    when=move || { total_count() > 0 }
                    fallback=|| view! {
                        <div class="px-3 py-2 text-slate-500 text-sm">"No script errors"</div>
                    }
                >
                    <div class="divide-y divide-slate-700/50">
                        <For
                            each=move || grouped_errors.get()
                            key=|(script, _): &(String, Vec<ScriptErrorEntry>)| script.clone()
                            children=move |(script, errors)| {
                                view! {
                                    <ScriptErrorGroup script=script errors=errors />
                                }
                            }
                        />
                    </div>
                </Show>
            </CollapsibleSection>
        </div>
    }
}

/// Error group for a single script file
#[component]
fn ScriptErrorGroup(script: String, errors: Vec<ScriptErrorEntry>) -> impl IntoView {
    let is_expanded = RwSignal::new(true);
    let error_count = errors.len();
    let script_name = script.clone();

    view! {
        <div class="bg-slate-900/30">
            <button
                class="w-full px-3 py-2 flex items-center gap-2 hover:bg-slate-700/30 text-left"
                on:click=move |_| is_expanded.update(|v| *v = !*v)
            >
                <span class="text-slate-500 text-xs transition-transform"
                    class:rotate-90=move || is_expanded.get()
                >
                    ">"
                </span>
                <span class="flex-1 text-sm text-amber-400 font-mono truncate">{script_name}</span>
                <span class="px-1.5 py-0.5 bg-red-900/50 text-red-400 text-xs rounded">
                    {error_count}
                </span>
            </button>

            <Show when=move || is_expanded.get()>
                <div class="divide-y divide-slate-700/30">
                    {errors.clone().into_iter().map(|error| {
                        view! {
                            <ScriptErrorRow error=error />
                        }
                    }).collect_view()}
                </div>
            </Show>
        </div>
    }
}

/// Single error row
#[component]
fn ScriptErrorRow(error: ScriptErrorEntry) -> impl IntoView {
    let gm_state = use_context::<GMEditorState>();

    let script_path = error.script.clone();

    // Open script in editor at the error line
    let open_in_editor = move |_| {
        if let Some(state) = gm_state {
            state.selected_script.set(Some(script_path.clone()));
            // Request script content - the editor will scroll to line when loaded
            with_admin_ws(|ws| ws.read_script(&script_path));
        }
    };

    // Determine if this is a runtime error or validation error
    let (badge_class, badge_text) = if error.function == "validate" {
        ("bg-yellow-900/50 text-yellow-400", "validation")
    } else {
        ("bg-red-900/50 text-red-400", "runtime")
    };

    view! {
        <div class="px-3 py-2 hover:bg-slate-700/30 group">
            <div class="flex items-start gap-2">
                // Error indicator
                <div class="mt-0.5">
                    <ErrorDotIcon />
                </div>

                <div class="flex-1 min-w-0">
                    // Error message
                    <div class="text-sm text-slate-300">{error.message.clone()}</div>

                    // Location and type
                    <div class="flex items-center gap-2 mt-1 text-xs">
                        <span class="text-slate-500 font-mono">
                            {format!("{}:{}:{}", error.function, error.line, error.column)}
                        </span>
                        <span class=format!("px-1 py-0.5 rounded {}", badge_class)>
                            {badge_text}
                        </span>
                        <span class="text-slate-600">
                            "tick "{error.tick}
                        </span>
                    </div>
                </div>

                // Open in editor button
                <button
                    class="p-1 hover:bg-slate-700 rounded opacity-0 group-hover:opacity-100 text-slate-400 hover:text-amber-400"
                    title="Open in Editor"
                    on:click=open_in_editor
                >
                    <OpenFileIcon />
                </button>
            </div>
        </div>
    }
}

#[component]
fn ErrorDotIcon() -> impl IntoView {
    view! {
        <div class="w-2 h-2 rounded-full bg-red-500"></div>
    }
}

#[component]
fn OpenFileIcon() -> impl IntoView {
    view! {
        <svg class="w-4 h-4" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 16 16">
            <path d="M2 4v9h9M5 2h9v9M5 11L14 2"/>
        </svg>
    }
}
