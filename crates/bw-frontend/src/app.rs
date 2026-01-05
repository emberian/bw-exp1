//! Root application component

use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;

use crate::pages::*;
use crate::state::*;

/// Root application component.
#[component]
pub fn App() -> impl IntoView {
    // Provide global game state
    provide_context(GameState::new());

    view! {
        <Router>
            <main class="min-h-screen bg-slate-900 text-slate-100">
                <Routes fallback=|| "Page not found">
                    <Route path=path!("/") view=HomePage />
                    <Route path=path!("/game") view=GamePage />
                    <Route path=path!("/register") view=RegisterPage />
                </Routes>
            </main>
        </Router>
    }
}
