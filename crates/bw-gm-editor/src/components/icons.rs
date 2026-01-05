//! Shared icon components for the GM Editor

use leptos::prelude::*;

// =============================================================================
// Media Control Icons
// =============================================================================

#[component]
pub fn PlayIcon() -> impl IntoView {
    view! {
        <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 16 16">
            <path d="M4 2l10 6-10 6V2z"/>
        </svg>
    }
}

#[component]
pub fn PauseIcon() -> impl IntoView {
    view! {
        <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 16 16">
            <path d="M3 2h4v12H3V2zm6 0h4v12H9V2z"/>
        </svg>
    }
}

// =============================================================================
// Debug Stepping Icons
// =============================================================================

#[component]
pub fn StepIntoIcon() -> impl IntoView {
    view! {
        <svg class="w-4 h-4" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 16 16">
            <path d="M8 2v8M5 7l3 3 3-3M8 14h0"/>
            <circle cx="8" cy="14" r="1" fill="currentColor"/>
        </svg>
    }
}

#[component]
pub fn StepOverIcon() -> impl IntoView {
    view! {
        <svg class="w-4 h-4" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 16 16">
            <path d="M2 8h8M7 5l3 3-3 3"/>
            <circle cx="13" cy="8" r="1.5" fill="currentColor"/>
        </svg>
    }
}

#[component]
pub fn StepOutIcon() -> impl IntoView {
    view! {
        <svg class="w-4 h-4" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 16 16">
            <path d="M8 14V6M5 9l3-3 3 3M8 2h0"/>
            <circle cx="8" cy="2" r="1" fill="currentColor"/>
        </svg>
    }
}

// =============================================================================
// UI Icons
// =============================================================================

#[component]
pub fn CloseIcon() -> impl IntoView {
    view! {
        <svg class="w-4 h-4" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 16 16">
            <path d="M4 4l8 8M12 4l-8 8"/>
        </svg>
    }
}

// =============================================================================
// Validation Severity Icons
// =============================================================================

/// Error icon (circle with X)
#[component]
pub fn ErrorIcon() -> impl IntoView {
    view! {
        <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10" />
            <path d="M15 9l-6 6M9 9l6 6" />
        </svg>
    }
}

/// Warning icon (triangle with !)
#[component]
pub fn WarningIcon() -> impl IntoView {
    view! {
        <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <path d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
    }
}

/// Info icon (circle with i)
#[component]
pub fn InfoIcon() -> impl IntoView {
    view! {
        <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10" />
            <path d="M12 16v-4M12 8h.01" />
        </svg>
    }
}

/// Hint icon (lightbulb)
#[component]
pub fn HintIcon() -> impl IntoView {
    view! {
        <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
            <path d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
        </svg>
    }
}
