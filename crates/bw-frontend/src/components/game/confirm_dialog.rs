//! Reusable confirmation dialog component

use leptos::prelude::*;

/// State for managing a confirmation dialog.
#[derive(Clone, Copy)]
pub struct ConfirmDialogState {
    /// Whether the dialog is visible
    pub visible: RwSignal<bool>,
    /// Title of the dialog
    pub title: RwSignal<String>,
    /// Message body of the dialog
    pub message: RwSignal<String>,
    /// Callback to run on confirm (stored as an ID to look up)
    callback_id: RwSignal<Option<u32>>,
}

impl ConfirmDialogState {
    pub fn new() -> Self {
        Self {
            visible: RwSignal::new(false),
            title: RwSignal::new(String::new()),
            message: RwSignal::new(String::new()),
            callback_id: RwSignal::new(None),
        }
    }

    /// Show the confirmation dialog with the given title and message.
    /// Returns the callback ID that should be checked in on_confirm.
    pub fn show(&self, title: impl Into<String>, message: impl Into<String>, callback_id: u32) {
        self.title.set(title.into());
        self.message.set(message.into());
        self.callback_id.set(Some(callback_id));
        self.visible.set(true);
    }

    /// Hide the dialog.
    pub fn hide(&self) {
        self.visible.set(false);
        self.callback_id.set(None);
    }

    /// Get the current callback ID (if dialog was confirmed).
    pub fn get_callback_id(&self) -> Option<u32> {
        self.callback_id.get()
    }
}

impl Default for ConfirmDialogState {
    fn default() -> Self {
        Self::new()
    }
}

/// Confirmation dialog component.
///
/// Usage:
/// ```rust
/// let confirm_state = ConfirmDialogState::new();
///
/// // In your view:
/// <ConfirmDialog
///     state=confirm_state
///     on_confirm=move |id| {
///         match id {
///             1 => do_thing_one(),
///             2 => do_thing_two(),
///             _ => {}
///         }
///     }
/// />
///
/// // To show:
/// confirm_state.show("Title", "Are you sure?", 1);
/// ```
#[component]
pub fn ConfirmDialog<F>(
    state: ConfirmDialogState,
    on_confirm: F,
) -> impl IntoView
where
    F: Fn(u32) + 'static + Clone + Send + Sync,
{
    let on_confirm_clone = on_confirm.clone();

    let handle_confirm = move |_| {
        if let Some(id) = state.get_callback_id() {
            on_confirm_clone(id);
        }
        state.hide();
    };

    let handle_cancel = move |_| {
        state.hide();
    };

    view! {
        <Show when=move || state.visible.get()>
            <div class="fixed inset-0 bg-black/70 flex items-center justify-center z-50">
                <div class="bg-slate-800 rounded-lg border border-slate-600 shadow-xl max-w-md w-full mx-4">
                    // Header
                    <div class="p-4 border-b border-slate-700">
                        <h2 class="text-lg font-semibold text-amber-400">{move || state.title.get()}</h2>
                    </div>

                    // Message
                    <div class="p-4 text-slate-300">
                        {move || state.message.get()}
                    </div>

                    // Buttons
                    <div class="p-4 pt-0 flex gap-3 justify-end">
                        <button
                            class="px-4 py-2 bg-slate-700 hover:bg-slate-600 rounded text-sm text-slate-200 transition-colors"
                            on:click=handle_cancel
                        >
                            "Cancel"
                        </button>
                        <button
                            class="px-4 py-2 bg-red-900 hover:bg-red-800 rounded text-sm text-red-100 transition-colors"
                            on:click=handle_confirm
                        >
                            "Confirm"
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
