//! CodeMirror 6 JavaScript bindings.

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

/// Initialize CodeMirror on a DOM element.
///
/// The `on_change` callback is called whenever the content changes.
pub fn init_codemirror<F>(element: &HtmlElement, initial_content: &str, on_change: F)
where
    F: Fn(String) + 'static,
{
    // Create a closure that can be called from JS
    let closure = Closure::new(move |content: String| {
        on_change(content);
    });

    js_init_codemirror(element, initial_content, &closure);

    // Leak the closure so it lives forever (it's tied to the editor lifetime)
    closure.forget();
}

/// Update the content of an existing CodeMirror instance.
///
/// This is called when content changes externally (e.g., file opened).
pub fn update_codemirror_content(element: &HtmlElement, content: &str) {
    js_update_content(element, content);
}

/// Destroy a CodeMirror instance and clean up.
#[allow(dead_code)]
pub fn destroy_codemirror(element: &HtmlElement) {
    js_destroy_codemirror(element);
}

/// Get the current content from a CodeMirror instance.
#[allow(dead_code)]
pub fn get_codemirror_content(element: &HtmlElement) -> Option<String> {
    js_get_content(element)
}
