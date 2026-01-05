//! CodeMirror 6 JavaScript bindings.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use wasm_bindgen::prelude::*;
use web_sys::HtmlElement;

#[wasm_bindgen]
extern "C" {
    /// Initialize CodeMirror on the given element.
    #[wasm_bindgen(js_namespace = window, js_name = initCodeMirror)]
    fn js_init_codemirror(element: &HtmlElement, content: &str, callback: &Closure<dyn Fn(String)>);

    /// Update the content of a CodeMirror instance.
    #[wasm_bindgen(js_namespace = window, js_name = updateCodeMirrorContent)]
    fn js_update_content(element: &HtmlElement, content: &str);

    /// Destroy a CodeMirror instance.
    #[wasm_bindgen(js_namespace = window, js_name = destroyCodeMirror)]
    fn js_destroy_codemirror(element: &HtmlElement);

    /// Get the current content of a CodeMirror instance.
    #[wasm_bindgen(js_namespace = window, js_name = getCodeMirrorContent)]
    fn js_get_content(element: &HtmlElement) -> Option<String>;
}

/// Monotonically increasing ID for CodeMirror instances.
/// Using atomic for thread-safety even though WASM is single-threaded.
static NEXT_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);

// Track closures by instance ID so we can clean them up on destroy.
// Using stable IDs instead of pointer addresses to avoid issues with DOM element recycling.
thread_local! {
    static CLOSURES: RefCell<HashMap<u64, Closure<dyn Fn(String)>>> = RefCell::new(HashMap::new());
}

/// Handle to a CodeMirror instance for cleanup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CodeMirrorHandle {
    instance_id: u64,
}

impl CodeMirrorHandle {
    /// Destroy this CodeMirror instance and clean up resources.
    pub fn destroy(&self, element: &HtmlElement) {
        js_destroy_codemirror(element);

        // Clean up the stored closure to prevent memory leak
        CLOSURES.with(|closures| {
            closures.borrow_mut().remove(&self.instance_id);
        });

        tracing::debug!("[codemirror] Destroyed instance {}", self.instance_id);
    }
}

/// Initialize CodeMirror on a DOM element.
///
/// The `on_change` callback is called whenever the content changes.
/// Returns a handle that must be used to properly destroy the instance.
pub fn init_codemirror<F>(element: &HtmlElement, initial_content: &str, on_change: F) -> CodeMirrorHandle
where
    F: Fn(String) + 'static,
{
    let instance_id = NEXT_INSTANCE_ID.fetch_add(1, Ordering::Relaxed);

    // Create a closure that can be called from JS
    let closure = Closure::new(move |content: String| {
        on_change(content);
    });

    js_init_codemirror(element, initial_content, &closure);

    // Store the closure so it stays alive and can be cleaned up later
    CLOSURES.with(|closures| {
        closures.borrow_mut().insert(instance_id, closure);
    });

    tracing::debug!("[codemirror] Initialized instance {}", instance_id);

    CodeMirrorHandle { instance_id }
}

/// Update the content of an existing CodeMirror instance.
///
/// This is called when content changes externally (e.g., file opened).
pub fn update_codemirror_content(element: &HtmlElement, content: &str) {
    js_update_content(element, content);
}

/// Get the current content from a CodeMirror instance.
#[allow(dead_code)]
pub fn get_codemirror_content(element: &HtmlElement) -> Option<String> {
    js_get_content(element)
}
