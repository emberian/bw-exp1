//! Notifications component
//!
//! Displays pending squadron invitations and alliance proposals with accept/decline actions.

use leptos::prelude::*;

use crate::api::WsService;
use crate::state::GameState;

#[component]
pub fn NotificationsPanel() -> impl IntoView {
    let game_state = expect_context::<GameState>();
    let ws = expect_context::<WsService>();

    let squadron_invites = move || game_state.pending_squadron_invites.get();
    let alliance_proposals = move || game_state.pending_alliance_proposals.get();

    let has_notifications = move || !squadron_invites().is_empty() || !alliance_proposals().is_empty();

    view! {
        <Show when=has_notifications>
            // Responsive: full-width with margins on mobile, fixed width on desktop
            <div data-testid="notifications-panel" class="absolute top-16 md:top-20 right-2 left-2 md:left-auto md:right-4 md:w-72 max-h-80 md:max-h-96 overflow-y-auto bg-slate-900/95 border border-amber-500/50 rounded-lg shadow-lg z-40">
                // Header
                <div class="p-3 border-b border-slate-700 bg-slate-800/50">
                    <h3 class="font-semibold text-amber-400 text-sm">"Pending Requests"</h3>
                </div>

                <div class="p-2 space-y-2">
                    // Squadron invites
                    <For
                        each=squadron_invites
                        key=|i| i.invite_id
                        children=move |invite| {
                            let invite_id = invite.invite_id;
                            let ws_accept = ws;
                            let ws_decline = ws;
                            let game_state_accept = game_state;
                            let game_state_decline = game_state;

                            view! {
                                <div data-testid="notification" class="bg-slate-800 rounded-lg p-3 border border-slate-700">
                                    <div class="text-xs text-purple-400 mb-1">"Squadron Invite"</div>
                                    <div class="text-sm text-slate-200 font-medium">
                                        "["{invite.squadron_tag.clone()}"] "{invite.squadron_name.clone()}
                                    </div>
                                    <div class="text-xs text-slate-400 mt-1">
                                        "From: "{invite.inviter_name.clone()}
                                    </div>
                                    <div class="flex gap-2 mt-2">
                                        <button
                                            class="flex-1 px-2 py-2 md:py-1 min-h-[44px] md:min-h-0 bg-green-900/50 hover:bg-green-800 rounded text-xs text-green-200 transition-colors"
                                            on:click=move |_| {
                                                ws_accept.accept_squadron_invite(invite_id);
                                                game_state_accept.pending_squadron_invites.update(|list| {
                                                    list.retain(|i| i.invite_id != invite_id);
                                                });
                                            }
                                        >
                                            "Accept"
                                        </button>
                                        <button
                                            class="flex-1 px-2 py-2 md:py-1 min-h-[44px] md:min-h-0 bg-red-900/50 hover:bg-red-800 rounded text-xs text-red-200 transition-colors"
                                            on:click=move |_| {
                                                ws_decline.decline_squadron_invite(invite_id);
                                                game_state_decline.pending_squadron_invites.update(|list| {
                                                    list.retain(|i| i.invite_id != invite_id);
                                                });
                                            }
                                        >
                                            "Decline"
                                        </button>
                                    </div>
                                </div>
                            }
                        }
                    />

                    // Alliance proposals
                    <For
                        each=alliance_proposals
                        key=|p| p.proposal_id
                        children=move |proposal| {
                            let proposal_id = proposal.proposal_id;
                            let ws_accept = ws;
                            let ws_decline = ws;
                            let game_state_accept = game_state;
                            let game_state_decline = game_state;

                            view! {
                                <div data-testid="notification" class="bg-slate-800 rounded-lg p-3 border border-blue-500/30">
                                    <div class="text-xs text-blue-400 mb-1">"Alliance Proposal"</div>
                                    <div class="text-sm text-slate-200 font-medium">
                                        "["{proposal.from_squadron_tag.clone()}"] "{proposal.from_squadron_name.clone()}
                                    </div>
                                    <div class="text-xs text-slate-400 mt-1">
                                        "Proposes an alliance with your squadron"
                                    </div>
                                    <div class="flex gap-2 mt-2">
                                        <button
                                            class="flex-1 px-2 py-2 md:py-1 min-h-[44px] md:min-h-0 bg-green-900/50 hover:bg-green-800 rounded text-xs text-green-200 transition-colors"
                                            on:click=move |_| {
                                                ws_accept.accept_alliance(proposal_id);
                                                game_state_accept.pending_alliance_proposals.update(|list| {
                                                    list.retain(|p| p.proposal_id != proposal_id);
                                                });
                                            }
                                        >
                                            "Accept"
                                        </button>
                                        <button
                                            class="flex-1 px-2 py-2 md:py-1 min-h-[44px] md:min-h-0 bg-red-900/50 hover:bg-red-800 rounded text-xs text-red-200 transition-colors"
                                            on:click=move |_| {
                                                ws_decline.decline_alliance(proposal_id);
                                                game_state_decline.pending_alliance_proposals.update(|list| {
                                                    list.retain(|p| p.proposal_id != proposal_id);
                                                });
                                            }
                                        >
                                            "Decline"
                                        </button>
                                    </div>
                                </div>
                            }
                        }
                    />
                </div>
            </div>
        </Show>
    }
}
