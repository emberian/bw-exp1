//! BLACKWING Frontend
//!
//! Leptos 0.8 frontend for the Blackwing game.


use leptos::mount::mount_to_body;
use wasm_bindgen::prelude::*;

pub mod app;
pub mod api;
pub mod components;
pub mod pages;
pub mod state;
pub mod utils;

pub use app::App;

#[wasm_bindgen(start)]
fn start() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}