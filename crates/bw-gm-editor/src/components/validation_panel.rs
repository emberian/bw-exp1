//! Validation panel component for displaying script validation issues

use leptos::prelude::*;
use bw_shared::dto::{ValidationIssueDto, ValidationSeverity};

use crate::state::ValidationState;

/// Validation panel component for showing script issues
#[component]
pub fn ValidationPanel(
    /// Current script path being validated
    script_path: RwSignal<Option<String>>,
    /// Current script content
    script_content: RwSignal<String>,
    /// Callback when user clicks on an issue to jump to line
    #[prop(optional)]
    on_jump_to_line: Option<Callback<(usize, usize)>>,
) -> impl IntoView {
    // Get state from context
    let validation_state = expect_context::<ValidationState>();

    // Local tracking for debounce
    let last_validated_content = RwSignal::new(String::new());

    // Debounced validation effect
    Effect::new(move |_| {
        let path = script_path.get();
        let content = script_content.get();

        // Skip if no script or content hasn't changed
        if path.is_none() || content == last_validated_content.get() {
            return;
        }

        // Update last validated and trigger validation
        last_validated_content.set(content.clone());
        validation_state.validating.set(true);

        if let Some(p) = path {
            use crate::api::admin_ws;
            admin_ws::with_admin_ws(|ws| ws.validate_script(&p, &content));
        }
    });

    // Count issues by severity
    let error_count = move || {
        validation_state.errors.get().len()
    };
    let warning_count = move || {
        validation_state.warnings.get().len()
    };

    // Combined issues for display
    let all_issues = move || validation_state.all_issues();

    view! {
        <div class="border-t border-slate-700 bg-slate-900/50">
            // Header
            <div class="flex items-center justify-between px-3 py-2 bg-slate-800/50">
                <div class="flex items-center gap-3">
                    <span class="text-xs font-medium text-slate-400">"Problems"</span>
                    <Show when=move || validation_state.validating.get()>
                        <span class="text-xs text-slate-500">"Validating..."</span>
                    </Show>
                </div>
                <div class="flex items-center gap-2 text-xs">
                    <Show when=move || { error_count() > 0 }>
                        <span class="flex items-center gap-1 text-red-400">
                            <ErrorIcon />
                            {error_count}
                        </span>
                    </Show>
                    <Show when=move || { warning_count() > 0 }>
                        <span class="flex items-center gap-1 text-yellow-400">
                            <WarningIcon />
                            {warning_count}
                        </span>
                    </Show>
                    <Show when=move || validation_state.is_valid.get() && all_issues().is_empty() && !validation_state.validating.get()>
                        <span class="text-green-400">"No issues"</span>
                    </Show>
                </div>
            </div>

            // Issues list
            <div class="max-h-40 overflow-auto">
                <Show
                    when=move || !all_issues().is_empty()
                    fallback=|| view! {
                        <div class="px-3 py-4 text-center text-xs text-slate-500">
                            "No validation issues"
                        </div>
                    }
                >
                    <div class="divide-y divide-slate-800">
                        <For
                            each=all_issues
                            key=|issue| format!("{}:{}:{}", issue.line, issue.column, issue.message.clone())
                            children=move |issue| {
                                let line = issue.line;
                                let column = issue.column;
                                let on_jump = on_jump_to_line;
                                view! {
                                    <IssueRow
                                        issue=issue
                                        on_click=move || {
                                            if let Some(cb) = on_jump {
                                                cb.run((line, column));
                                            }
                                        }
                                    />
                                }
                            }
                        />
                    </div>
                </Show>
            </div>
        </div>
    }
}

/// Single validation issue row
#[component]
fn IssueRow<F>(
    issue: ValidationIssueDto,
    on_click: F,
) -> impl IntoView
where
    F: Fn() + 'static,
{
    let (icon, severity_class) = match issue.severity {
        ValidationSeverity::Error => (view! { <ErrorIcon /> }.into_any(), "text-red-400"),
        ValidationSeverity::Warning => (view! { <WarningIcon /> }.into_any(), "text-yellow-400"),
        ValidationSeverity::Info => (view! { <InfoIcon /> }.into_any(), "text-blue-400"),
        ValidationSeverity::Hint => (view! { <HintIcon /> }.into_any(), "text-slate-400"),
    };

    let location = format!(
        "Ln {}, Col {}",
        issue.line,
        issue.column
    );

    let suggestion = issue.suggestion.clone();
    let has_suggestion = suggestion.is_some();
    let suggestion_text = suggestion.unwrap_or_default();

    view! {
        <button
            class="w-full px-3 py-1.5 flex items-start gap-2 text-left hover:bg-slate-800/50 transition-colors"
            on:click=move |_| on_click()
        >
            <span class=format!("flex-shrink-0 mt-0.5 {}", severity_class)>
                {icon}
            </span>
            <div class="flex-1 min-w-0">
                <div class="text-xs text-slate-300 truncate">{issue.message}</div>
                <div class="flex items-center gap-2 mt-0.5">
                    <span class="text-xs text-slate-500">{location}</span>
                    <Show when=move || has_suggestion>
                        <span class="text-xs text-cyan-500">
                            {suggestion_text.clone()}
                        </span>
                    </Show>
                </div>
            </div>
        </button>
    }
}

/// Error icon (circle with X)
#[component]
fn ErrorIcon() -> impl IntoView {
    view! {
        <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10" />
            <path d="M15 9l-6 6M9 9l6 6" />
        </svg>
    }
}

/// Warning icon (triangle with !)
#[component]
fn WarningIcon() -> impl IntoView {
    view! {
        <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <path d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
    }
}

/// Info icon (circle with i)
#[component]
fn InfoIcon() -> impl IntoView {
    view! {
        <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10" />
            <path d="M12 16v-4M12 8h.01" />
        </svg>
    }
}

/// Hint icon (lightbulb)
#[component]
fn HintIcon() -> impl IntoView {
    view! {
        <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <path d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
        </svg>
    }
}
