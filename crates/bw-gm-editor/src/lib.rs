//! BW GM Editor - Game Master interface for Blackwing
//!
//! A lazily-loaded Leptos/WASM application that provides admin/GM functionality
//! for inspecting and editing the game simulation.

mod api;
mod app;
mod bindings;
mod components;
mod state;

use leptos::prelude::*;
use wasm_bindgen::prelude::*;

/// Mount the GM Editor into a container element.
///
/// Called from the main frontend via dynamic import.
#[wasm_bindgen]
pub fn mount_gm_editor(container_id: &str, ws_url: &str, auth_token: &str) -> Result<(), JsValue> {
    // Set up panic hook for better error messages
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    // Initialize tracing
    tracing_wasm::set_as_global_default();

    tracing::info!("[gm-editor] Mounting to container: {}", container_id);

    // Get the container element
    let window = web_sys::window().ok_or("No window")?;
    let document = window.document().ok_or("No document")?;
    let container = document
        .get_element_by_id(container_id)
        .ok_or_else(|| format!("Container '{}' not found", container_id))?;

    // Show the container
    if let Some(html_el) = container.dyn_ref::<web_sys::HtmlElement>() {
        html_el.class_list().remove_1("hidden").ok();
    }

    // Store connection info in context
    let ws_url = ws_url.to_string();
    let auth_token = auth_token.to_string();

    // Mount Leptos app (forget the handle to keep mounted until unmount_gm_editor is called)
    leptos::mount::mount_to(container.unchecked_into(), move || {
        view! {
            <app::GMEditorApp ws_url=ws_url.clone() auth_token=auth_token.clone() />
        }
    })
    .forget();

    tracing::info!("[gm-editor] Mounted successfully");
    Ok(())
}

/// Unmount and cleanup the GM Editor.
#[wasm_bindgen]
pub fn unmount_gm_editor(container_id: &str) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("No window")?;
    let document = window.document().ok_or("No document")?;

    if let Some(container) = document.get_element_by_id(container_id) {
        // Hide and clear the container
        if let Some(html_el) = container.dyn_ref::<web_sys::HtmlElement>() {
            html_el.class_list().add_1("hidden").ok();
            html_el.set_inner_html("");
        }
    }

    tracing::info!("[gm-editor] Unmounted");
    Ok(())
}
