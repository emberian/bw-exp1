//! Registration page

use leptos::prelude::*;
use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;
use uuid::Uuid;

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

/// Faction info from API.
#[derive(Clone, Debug, Deserialize)]
pub struct FactionInfo {
    pub id: Uuid,
    pub name: String,
    pub tag: String,
    pub description: String,
    pub philosophy: String,
    pub color: String,
}

#[derive(Deserialize)]
struct FactionsResponse {
    factions: Vec<FactionInfo>,
}

#[component]
pub fn RegisterPage() -> impl IntoView {
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let password_confirm = RwSignal::new(String::new());
    let faction = RwSignal::new(String::new());
    let error = RwSignal::new(Option::<String>::None);
    let loading = RwSignal::new(false);

    // Factions state
    let factions = RwSignal::new(Vec::<FactionInfo>::new());
    let factions_loading = RwSignal::new(true);
    let factions_error = RwSignal::new(Option::<String>::None);

    // Fetch factions on mount
    Effect::new(move |_| {
        spawn_local(async move {
            match fetch_factions().await {
                Ok(response) => {
                    // Set default faction to first one's tag
                    if let Some(first) = response.factions.first() {
                        faction.set(first.tag.clone());
                    }
                    factions.set(response.factions);
                    factions_loading.set(false);
                }
                Err(e) => {
                    factions_error.set(Some(e));
                    factions_loading.set(false);
                }
            }
        });
    });

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

        if faction_val.is_empty() {
            error.set(Some("Please select a faction".to_string()));
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
                                let _ = window.location().set_href("/play/game");
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
                    // Inline validation hint
                    {move || {
                        let len = username.get().len();
                        if len == 0 {
                            view! { <div class="mt-1 text-xs text-slate-500">"3-32 characters"</div> }.into_any()
                        } else if len < 3 {
                            view! { <div class="mt-1 text-xs text-red-400">{format!("{} more character{} needed", 3 - len, if len == 2 { "" } else { "s" })}</div> }.into_any()
                        } else {
                            view! { <div class="mt-1 text-xs text-green-400">"Valid callsign"</div> }.into_any()
                        }
                    }}
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
                    // Inline validation hint
                    {move || {
                        let len = password.get().len();
                        if len == 0 {
                            view! { <div class="mt-1 text-xs text-slate-500">"Minimum 8 characters"</div> }.into_any()
                        } else if len < 8 {
                            view! { <div class="mt-1 text-xs text-red-400">{format!("{} more character{} needed", 8 - len, if 8 - len == 1 { "" } else { "s" })}</div> }.into_any()
                        } else {
                            view! { <div class="mt-1 text-xs text-green-400">"Password length OK"</div> }.into_any()
                        }
                    }}
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
                    // Inline validation hint
                    {move || {
                        let confirm = password_confirm.get();
                        let pass = password.get();
                        if confirm.is_empty() {
                            view! { <div class="mt-1 text-xs text-slate-500">"Re-enter your password"</div> }.into_any()
                        } else if confirm != pass {
                            view! { <div class="mt-1 text-xs text-red-400">"Passwords don't match"</div> }.into_any()
                        } else {
                            view! { <div class="mt-1 text-xs text-green-400">"Passwords match"</div> }.into_any()
                        }
                    }}
                </div>

                // Faction selection
                <div class="mb-6">
                    <label class="block text-sm font-medium text-slate-300 mb-2">
                        "Faction"
                    </label>

                    // Loading state
                    <Show when=move || factions_loading.get()>
                        <div class="animate-pulse space-y-3">
                            <div class="h-24 bg-slate-700 rounded-lg" />
                            <div class="h-24 bg-slate-700 rounded-lg" />
                            <div class="h-24 bg-slate-700 rounded-lg" />
                        </div>
                    </Show>

                    // Error state
                    {move || factions_error.get().map(|e| view! {
                        <div class="p-3 bg-red-900/50 border border-red-700 rounded text-red-200 text-sm">
                            "Failed to load factions: "{e}
                        </div>
                    })}

                    // Factions list
                    <Show when=move || !factions_loading.get() && factions_error.get().is_none()>
                        <div class="grid grid-cols-1 gap-3">
                            <For
                                each=move || factions.get()
                                key=|f| f.id
                                children=move |f| {
                                    let tag = f.tag.clone();
                                    let tag_for_click = f.tag.clone();
                                    let tag_for_selected = f.tag.clone();
                                    let name = f.name.clone();
                                    let description = f.description.clone();
                                    let philosophy = f.philosophy.clone();
                                    let color = f.color.clone();

                                    view! {
                                        <FactionOption
                                            tag=tag
                                            name=name
                                            description=description
                                            philosophy=philosophy
                                            color=color
                                            on_select=move |_| faction.set(tag_for_click.clone())
                                            is_selected=move || faction.get() == tag_for_selected
                                        />
                                    }
                                }
                            />
                        </div>
                    </Show>
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
                    disabled=move || loading.get() || factions_loading.get()
                    class="w-full py-3 bg-amber-600 hover:bg-amber-500 disabled:bg-slate-600
                           text-white font-semibold rounded-lg transition-colors"
                >
                    {move || if loading.get() { "Creating..." } else { "Begin Service" }}
                </button>
            </form>

            <div class="mt-6 flex flex-col items-center gap-2">
                <a href="/play/login" class="text-amber-400 hover:text-amber-300">
                    "Already have an account? Log in"
                </a>
                <a href="/" class="text-slate-400 hover:text-slate-300">
                    "← Back to Home"
                </a>
            </div>
        </div>
    }
}

/// Fetch factions from API.
async fn fetch_factions() -> Result<FactionsResponse, String> {
    let response = Request::get("/api/factions")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    response
        .json::<FactionsResponse>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
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
fn FactionOption<S, C>(
    tag: String,
    name: String,
    description: String,
    philosophy: String,
    color: String,
    on_select: C,
    is_selected: S,
) -> impl IntoView
where
    S: Fn() -> bool + 'static + Clone + Send + Sync,
    C: Fn(()) + 'static + Clone + Send + Sync,
{
    let is_selected_class = is_selected.clone();
    let is_selected_style = is_selected.clone();
    let on_select_click = on_select.clone();

    // Clone color for reactive style closure
    let color_for_style = color.clone();

    // Format philosophy with quotes
    let philosophy_display = format!("\"{}\"", philosophy);

    view! {
        <button
            type="button"
            on:click=move |_| on_select_click(())
            class=move || format!(
                "p-4 rounded-lg border-2 text-left transition-all {}",
                if is_selected_class() {
                    "border-amber-500 bg-amber-900/20"
                } else {
                    "border-slate-600 bg-slate-700 hover:border-slate-500"
                }
            )
            style=move || {
                if is_selected_style() {
                    format!("border-color: {}; background-color: {}20", color_for_style, color_for_style)
                } else {
                    String::new()
                }
            }
        >
            <div class="flex items-center gap-2 mb-1">
                <span
                    class="w-3 h-3 rounded-full"
                    style=format!("background-color: {}", color)
                />
                <span class="font-semibold text-slate-200">{name}</span>
                <span class="text-xs text-slate-500">"["{tag}"]"</span>
            </div>
            <div class="text-sm text-slate-400 mb-2">{description}</div>
            <div class="text-xs text-slate-500 italic">{philosophy_display}</div>
        </button>
    }
}
