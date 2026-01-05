//! Registration page

use leptos::prelude::*;
use crate::state::GameState;

#[component]
pub fn RegisterPage() -> impl IntoView {
    let game_state = expect_context::<GameState>();

    let username = RwSignal::new(String::new());
    let faction = RwSignal::new("COMPACT".to_string());
    let error = RwSignal::new(Option::<String>::None);
    let loading = RwSignal::new(false);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();

        let username_val = username.get();
        let faction_val = faction.get();

        if username_val.len() < 3 {
            error.set(Some("Username must be at least 3 characters".to_string()));
            return;
        }

        loading.set(true);
        error.set(None);

        // TODO: Call registration API
        // For now, just redirect to game
        let window = web_sys::window().unwrap();
        let _ = window.location().set_href("/game");
    };

    view! {
        <div class="flex flex-col items-center justify-center min-h-screen p-8">
            <h1 class="text-4xl font-bold text-amber-500 mb-8">"Create Your Officer"</h1>

            <form
                on:submit=on_submit
                class="w-full max-w-md bg-slate-800 rounded-lg p-8"
            >
                // Username field
                <div class="mb-6">
                    <label class="block text-sm font-medium text-slate-300 mb-2">
                        "Callsign"
                    </label>
                    <input
                        type="text"
                        prop:value=move || username.get()
                        on:input=move |ev| username.set(event_target_value(&ev))
                        class="w-full px-4 py-2 bg-slate-700 border border-slate-600 rounded-lg
                               text-white focus:outline-none focus:border-amber-500"
                        placeholder="Enter your callsign..."
                    />
                </div>

                // Faction selection
                <div class="mb-6">
                    <label class="block text-sm font-medium text-slate-300 mb-2">
                        "Faction"
                    </label>
                    <div class="grid grid-cols-1 gap-3">
                        <FactionOption
                            value="COMPACT"
                            name="Continuity Compact"
                            description="The legitimate government. Stability through cooperation."
                            selected=faction
                        />
                        <FactionOption
                            value="FLOTILLA"
                            name="Argent Flotilla"
                            description="Military peacekeepers. Vigilance is purpose."
                            selected=faction
                        />
                        <FactionOption
                            value="FORGE"
                            name="Forgeborn"
                            description="Industrial builders. Purpose through creation."
                            selected=faction
                        />
                    </div>
                </div>

                // Error display
                {move || error.get().map(|e| view! {
                    <div class="mb-4 p-3 bg-red-900/50 border border-red-700 rounded text-red-200 text-sm">
                        {e}
                    </div>
                })}

                // Submit button
                <button
                    type="submit"
                    disabled=move || loading.get()
                    class="w-full py-3 bg-amber-600 hover:bg-amber-500 disabled:bg-slate-600
                           text-white font-semibold rounded-lg transition-colors"
                >
                    {move || if loading.get() { "Creating..." } else { "Begin Service" }}
                </button>
            </form>

            <a href="/" class="mt-6 text-slate-400 hover:text-slate-300">
                "← Back to Home"
            </a>
        </div>
    }
}

#[component]
fn FactionOption(
    value: &'static str,
    name: &'static str,
    description: &'static str,
    selected: RwSignal<String>,
) -> impl IntoView {
    let is_selected = move || selected.get() == value;

    view! {
        <button
            type="button"
            on:click=move |_| selected.set(value.to_string())
            class=move || format!(
                "p-4 rounded-lg border-2 text-left transition-colors {}",
                if is_selected() {
                    "border-amber-500 bg-amber-900/20"
                } else {
                    "border-slate-600 bg-slate-700 hover:border-slate-500"
                }
            )
        >
            <div class="font-semibold text-slate-200">{name}</div>
            <div class="text-sm text-slate-400">{description}</div>
        </button>
    }
}
