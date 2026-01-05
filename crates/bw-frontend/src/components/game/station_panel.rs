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

    let is_docked = move || game_state.ship_status.get().starts_with("Docked");
    let station_name = move || game_state.docked_station_name.get();
    let services = move || game_state.docked_station_services.get();

    let ws_undock = ws;
    let handle_undock = move |_| {
        ws_undock.undock();
    };

    view! {
        <Show when=is_docked>
            // Responsive positioning: above mobile nav on small screens, full-width on mobile
            // bottom-nav accounts for nav bar height + safe area on mobile
            <div data-testid="station-panel" class="absolute bottom-nav md:bottom-4 left-2 right-2 md:left-1/2 md:right-auto md:-translate-x-1/2 bg-slate-900/95 border border-amber-500/50 rounded-lg p-3 md:p-4 md:w-80 shadow-lg z-40">
                // Header
                <div class="flex justify-between items-center mb-3 md:mb-4">
                    <div>
                        <div class="text-xs text-slate-400">"Docked at"</div>
                        <h3 class="font-semibold text-amber-400">{station_name}</h3>
                    </div>
                    <button
                        data-testid="undock-button"
                        class="px-3 py-2 md:py-1 bg-red-900 hover:bg-red-800 rounded text-xs text-red-100 transition-colors min-h-[44px] md:min-h-0"
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
                        testid="service-refuel"
                        game_state=game_state
                        ws=ws
                    />
                    <StationServiceButton
                        name="Rearm"
                        description="Restore ammunition to 100%"
                        service="Rearm"
                        testid="service-rearm"
                        game_state=game_state
                        ws=ws
                    />
                    <StationServiceButton
                        name="Repair"
                        description="Restore hull integrity"
                        service="Repair"
                        testid="service-repair"
                        game_state=game_state
                        ws=ws
                    />
                    <StationServiceButton
                        name="Shore Leave"
                        description="Restore crew morale"
                        service="ShoreLeave"
                        testid="service-shore-leave"
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
    testid: &'static str,
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
                        data-testid=testid
                        class=move || {
                            // Touch-friendly size on mobile with min-h-[56px]
                            let base = "px-3 py-3 rounded text-center transition-colors flex flex-col items-center gap-1 min-h-[56px] md:min-h-0";
                            if is_enabled_class() {
                                format!("{} bg-slate-700 hover:bg-amber-600 cursor-pointer", base)
                            } else {
                                format!("{} bg-slate-800/50 cursor-not-allowed opacity-50", base)
                            }
                        }
                        disabled=move || !is_enabled_disabled()
                        on:click=handle_click_inner
                    >
                        <ServiceIcon service=service />
                        <div class="text-xs text-slate-200 font-medium">{name}</div>
                        <div class="text-[10px] text-slate-400 leading-tight">{description}</div>
                    </button>
                }.into_any()
            } else {
                view! {
                    <div class="px-3 py-3 bg-slate-800/50 rounded text-center flex flex-col items-center gap-1 min-h-[56px] md:min-h-0">
                        <ServiceIcon service=service disabled=true />
                        <div class="text-xs text-slate-600">{name}</div>
                        <div class="text-[10px] text-slate-700">"Not available"</div>
                    </div>
                }.into_any()
            }
        }}
    }
}

// =============================================================================
// Asset Placeholder Components
// =============================================================================

/// Service icon placeholder - displays icon for station services.
#[component]
fn ServiceIcon(service: &'static str, #[prop(optional)] disabled: bool) -> impl IntoView {
    let color_class = if disabled { "text-slate-600" } else { "text-amber-400" };

    let icon_content = match service {
        "Refuel" => view! {
            <svg width="24" height="24" viewBox="0 0 24 24" class=color_class>
                // Fuel pump icon placeholder
                <rect x="6" y="4" width="8" height="16" rx="1" fill="none" stroke="currentColor" stroke-width="1.5" />
                <path d="M14 8 L18 8 L18 14 L16 14" fill="none" stroke="currentColor" stroke-width="1.5" />
                <circle cx="18" cy="14" r="2" fill="none" stroke="currentColor" stroke-width="1.5" />
                <rect x="8" y="6" width="4" height="4" fill="currentColor" opacity="0.3" />
            </svg>
        }.into_any(),
        "Rearm" => view! {
            <svg width="24" height="24" viewBox="0 0 24 24" class=color_class>
                // Missile/ammo icon placeholder
                <path d="M12 2 L14 6 L14 18 L12 22 L10 18 L10 6 Z" fill="none" stroke="currentColor" stroke-width="1.5" />
                <line x1="10" y1="10" x2="14" y2="10" stroke="currentColor" stroke-width="1" />
                <line x1="10" y1="14" x2="14" y2="14" stroke="currentColor" stroke-width="1" />
                <circle cx="12" cy="5" r="1" fill="currentColor" />
            </svg>
        }.into_any(),
        "Repair" => view! {
            <svg width="24" height="24" viewBox="0 0 24 24" class=color_class>
                // Wrench icon placeholder
                <path d="M6 6 L10 10 L8 12 L4 8 C2 10 2 14 4 16 C6 18 10 18 12 16 L20 16 L20 20 L16 20 L16 16" fill="none" stroke="currentColor" stroke-width="1.5" />
                <circle cx="7" cy="7" r="3" fill="none" stroke="currentColor" stroke-width="1.5" />
                <rect x="14" y="14" width="6" height="6" rx="1" fill="currentColor" opacity="0.3" />
            </svg>
        }.into_any(),
        "ShoreLeave" => view! {
            <svg width="24" height="24" viewBox="0 0 24 24" class=color_class>
                // Recreation/drink icon placeholder
                <path d="M8 4 L16 4 L14 12 L14 20 L10 20 L10 12 Z" fill="none" stroke="currentColor" stroke-width="1.5" />
                <ellipse cx="12" cy="4" rx="4" ry="1" fill="none" stroke="currentColor" stroke-width="1.5" />
                <line x1="8" y1="20" x2="16" y2="20" stroke="currentColor" stroke-width="1.5" />
                <path d="M10 8 Q12 10 14 8" fill="none" stroke="currentColor" stroke-width="1" opacity="0.5" />
            </svg>
        }.into_any(),
        _ => view! {
            <svg width="24" height="24" viewBox="0 0 24 24" class=color_class>
                <rect x="4" y="4" width="16" height="16" rx="2" fill="none" stroke="currentColor" stroke-width="1.5" />
                <text x="12" y="15" text-anchor="middle" font-size="10" fill="currentColor">"?"</text>
            </svg>
        }.into_any(),
    };

    view! {
        <div class="w-6 h-6">
            {icon_content}
        </div>
    }
}
