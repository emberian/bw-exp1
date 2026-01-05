//! BLACKWING Frontend Entry Point

use bw_frontend::App;
use leptos::mount::mount_to_body;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
fn start() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

fn main() {
    // Native main for non-wasm builds (unused in browser)
}
