//! Root application component

use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;

use crate::api::WsService;
use crate::pages::*;
use crate::state::*;

/// Root application component.
#[component]
pub fn App() -> impl IntoView {
    // Create and provide global game state
    let game_state = GameState::new();
    provide_context(game_state);

    // Create and provide WebSocket service (signals only, so Send+Sync)
    let ws_service = WsService::new();
    provide_context(ws_service);

    view! {
        <Router>
            <main class="min-h-screen bg-slate-900 text-slate-100">
                <Routes fallback=|| "Page not found">
                    <Route path=path!("/") view=HomePage />
                    <Route path=path!("/game") view=GamePage />
                    <Route path=path!("/login") view=LoginPage />
                    <Route path=path!("/register") view=RegisterPage />
                </Routes>
            </main>
        </Router>
    }
}
