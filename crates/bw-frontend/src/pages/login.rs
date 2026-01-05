//! Login page

use leptos::prelude::*;
use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;

#[derive(Serialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct AuthResponse {
    success: bool,
    token: Option<String>,
    error: Option<String>,
}

#[component]
pub fn LoginPage() -> impl IntoView {
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(Option::<String>::None);
    let loading = RwSignal::new(false);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();

        let username_val = username.get();
        let password_val = password.get();

        if username_val.is_empty() {
            error.set(Some("Username is required".to_string()));
            return;
        }

        if password_val.is_empty() {
            error.set(Some("Password is required".to_string()));
            return;
        }

        loading.set(true);
        error.set(None);

        // Call login API asynchronously
        spawn_local(async move {
            let result = login_player(username_val, password_val).await;

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
                        error.set(Some(response.error.unwrap_or_else(|| "Login failed".to_string())));
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
            <h1 class="text-4xl font-bold text-amber-500 mb-8">"Welcome Back, Officer"</h1>

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
                <div class="mb-6">
                    <label class="block text-sm font-medium text-slate-300 mb-2">
                        "Password"
                    </label>
                    <input
                        type="password"
                        prop:value=move || password.get()
                        on:input=move |ev| password.set(event_target_value(&ev))
                        class="w-full px-4 py-2 bg-slate-700 border border-slate-600 rounded-lg
                               text-white focus:outline-none focus:border-amber-500"
                        placeholder="Enter your password..."
                    />
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
                    {move || if loading.get() { "Logging in..." } else { "Log In" }}
                </button>
            </form>

            <div class="mt-6 flex flex-col items-center gap-2">
                <a href="/register" class="text-amber-400 hover:text-amber-300">
                    "Need an account? Register"
                </a>
                <a href="/" class="text-slate-400 hover:text-slate-300">
                    "← Back to Home"
                </a>
            </div>
        </div>
    }
}

/// Call the login API.
async fn login_player(username: String, password: String) -> Result<AuthResponse, String> {
    let request = LoginRequest { username, password };

    let response = Request::post("/api/auth/login")
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
