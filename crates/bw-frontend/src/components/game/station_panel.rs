//! Station services panel component
//!
//! Displays available station services when docked and allows using them.

use leptos::prelude::*;

use crate::api::WsService;
use crate::state::GameState;

#[component]
pub fn StationPanel() -> impl IntoView {
    let game_state = expect_context::<GameState>();
    let ws = expect_context::<WsService>();

    let is_docked = move || game_state.ship_status.get() == "Docked";
    let station_name = move || game_state.docked_station_name.get();
    let services = move || game_state.docked_station_services.get();

    let ws_undock = ws;
    let handle_undock = move |_| {
        ws_undock.undock();
    };

    view! {
        <Show when=is_docked>
            <div class="absolute bottom-4 left-1/2 -translate-x-1/2 bg-slate-900/95 border border-amber-500/50 rounded-lg p-4 w-80 shadow-lg z-50">
                // Header
                <div class="flex justify-between items-center mb-4">
                    <div>
                        <div class="text-xs text-slate-400">"Docked at"</div>
                        <h3 class="font-semibold text-amber-400">{station_name}</h3>
                    </div>
                    <button
                        class="px-3 py-1 bg-red-900 hover:bg-red-800 rounded text-xs text-red-100 transition-colors"
                        on:click=handle_undock
                    >
                        "Undock"
                    </button>
                </div>

                // Services grid
                <div class="grid grid-cols-2 gap-2">
                    <StationServiceButton
                        name="Refuel"
                        description="Restore fuel to 100%"
                        service="Refuel"
                        game_state=game_state
                        ws=ws
                    />
                    <StationServiceButton
                        name="Rearm"
                        description="Restore ammunition to 100%"
                        service="Rearm"
                        game_state=game_state
                        ws=ws
                    />
                    <StationServiceButton
                        name="Repair"
                        description="Restore hull integrity"
                        service="Repair"
                        game_state=game_state
                        ws=ws
                    />
                    <StationServiceButton
                        name="Shore Leave"
                        description="Restore crew morale"
                        service="ShoreLeave"
                        game_state=game_state
                        ws=ws
                    />
                </div>

                // Show message if no services available
                <Show when=move || services().is_empty()>
                    <div class="mt-2 text-xs text-slate-400 italic text-center">
                        "No services available at this station."
                    </div>
                </Show>

                // Station tips
                <div class="mt-4 pt-3 border-t border-slate-700 text-xs text-slate-500">
                    "Services require reputation. Hostile stations may refuse service."
                </div>
            </div>
        </Show>
    }
}

/// Station service button that checks game state for availability and enabled status.
#[component]
fn StationServiceButton(
    name: &'static str,
    description: &'static str,
    service: &'static str,
    game_state: GameState,
    ws: WsService,
) -> impl IntoView {
    let service_name = service.to_string();
    let service_for_check = service.to_string();
    let service_for_enabled = service.to_string();

    // Check if this service is available at this station
    let is_available = move || {
        game_state.docked_station_services.get().contains(&service_for_check)
    };

    // Check if service can be used (resource not already full)
    let is_enabled = move || {
        let services = game_state.docked_station_services.get();
        if !services.contains(&service_for_enabled) {
            return false;
        }
        match service {
            "Refuel" => game_state.fuel.get() < 100.0,
            "Rearm" => game_state.ammunition.get() < 100.0,
            "Repair" => game_state.ship_hull.get() < game_state.ship_max_hull.get(),
            "ShoreLeave" => game_state.morale.get() < 100.0,
            _ => true,
        }
    };

    let handle_click = move |_| {
        ws.use_service(service_name.clone());
    };

    view! {
        {move || {
            // Clone closures inside the outer closure so they can be used multiple times
            let is_available_check = is_available.clone();
            let is_enabled_class = is_enabled.clone();
            let is_enabled_disabled = is_enabled.clone();
            let handle_click_inner = handle_click.clone();

            if is_available_check() {
                view! {
                    <button
                        class=move || {
                            let base = "px-3 py-2 rounded text-center transition-colors";
                            if is_enabled_class() {
                                format!("{} bg-slate-700 hover:bg-amber-600 cursor-pointer", base)
                            } else {
                                format!("{} bg-slate-800/50 cursor-not-allowed opacity-50", base)
                            }
                        }
                        disabled=move || !is_enabled_disabled()
                        on:click=handle_click_inner
                    >
                        <div class="text-xs text-slate-200">{name}</div>
                        <div class="text-[10px] text-slate-400">{description}</div>
                    </button>
                }.into_any()
            } else {
                view! {
                    <div class="px-3 py-2 bg-slate-800/50 rounded text-center">
                        <div class="text-xs text-slate-600">{name}</div>
                        <div class="text-[10px] text-slate-700">"Not available"</div>
                    </div>
                }.into_any()
            }
        }}
    }
}
