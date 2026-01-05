//! BLACKWING Frontend Entry Point

use bw_frontend::App;
use leptos::mount::mount_to_body;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
