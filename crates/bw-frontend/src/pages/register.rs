//! Registration page

use leptos::prelude::*;
use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;

#[derive(Serialize)]
struct RegisterRequest {
    username: String,
    password: String,
    faction: String,
}

#[derive(Deserialize)]
struct AuthResponse {
    success: bool,
    token: Option<String>,
    error: Option<String>,
}

#[component]
pub fn RegisterPage() -> impl IntoView {
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let password_confirm = RwSignal::new(String::new());
    let faction = RwSignal::new("COMPACT".to_string());
    let error = RwSignal::new(Option::<String>::None);
    let loading = RwSignal::new(false);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();

        let username_val = username.get();
        let password_val = password.get();
        let password_confirm_val = password_confirm.get();
        let faction_val = faction.get();

        if username_val.len() < 3 {
            error.set(Some("Username must be at least 3 characters".to_string()));
            return;
        }

        if password_val.len() < 8 {
            error.set(Some("Password must be at least 8 characters".to_string()));
            return;
        }

        if password_val != password_confirm_val {
            error.set(Some("Passwords do not match".to_string()));
            return;
        }

        loading.set(true);
        error.set(None);

        // Call registration API asynchronously
        spawn_local(async move {
            let result = register_player(username_val, password_val, faction_val).await;

            match result {
                Ok(response) => {
                    if response.success {
                        if let Some(token) = response.token {
                            // Store token in localStorage
                            if let Some(storage) = web_sys::window()
                                .and_then(|w| w.local_storage().ok())
                                .flatten()
                            {
                                let _ = storage.set_item("auth_token", &token);
                            }

                            // Redirect to game
                            if let Some(window) = web_sys::window() {
                                let _ = window.location().set_href("/game");
                            }
                        }
                    } else {
                        error.set(Some(response.error.unwrap_or_else(|| "Registration failed".to_string())));
                        loading.set(false);
                    }
                }
                Err(e) => {
                    error.set(Some(format!("Network error: {}", e)));
                    loading.set(false);
                }
            }
        });
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

                // Password field
                <div class="mb-4">
                    <label class="block text-sm font-medium text-slate-300 mb-2">
                        "Password"
                    </label>
                    <input
                        type="password"
                        prop:value=move || password.get()
                        on:input=move |ev| password.set(event_target_value(&ev))
                        class="w-full px-4 py-2 bg-slate-700 border border-slate-600 rounded-lg
                               text-white focus:outline-none focus:border-amber-500"
                        placeholder="Enter password (min 8 characters)..."
                    />
                </div>

                // Password confirmation field
                <div class="mb-6">
                    <label class="block text-sm font-medium text-slate-300 mb-2">
                        "Confirm Password"
                    </label>
                    <input
                        type="password"
                        prop:value=move || password_confirm.get()
                        on:input=move |ev| password_confirm.set(event_target_value(&ev))
                        class="w-full px-4 py-2 bg-slate-700 border border-slate-600 rounded-lg
                               text-white focus:outline-none focus:border-amber-500"
                        placeholder="Confirm your password..."
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

            <div class="mt-6 flex flex-col items-center gap-2">
                <a href="/login" class="text-amber-400 hover:text-amber-300">
                    "Already have an account? Log in"
                </a>
                <a href="/" class="text-slate-400 hover:text-slate-300">
                    "← Back to Home"
                </a>
            </div>
        </div>
    }
}

/// Call the registration API.
async fn register_player(username: String, password: String, faction: String) -> Result<AuthResponse, String> {
    let request = RegisterRequest { username, password, faction };

    let response = Request::post("/api/auth/register")
        .header("Content-Type", "application/json")
        .json(&request)
        .map_err(|e| format!("Failed to create request: {}", e))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    response
        .json::<AuthResponse>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
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
