//! Communications panel component
//!
//! Displays chat messages and allows sending to different channels.
//!
//! Design note: Chat intentionally does NOT auto-scroll on new messages.
//! Users control their scroll position; a "New messages" button appears
//! when not at bottom. This prevents jarring scroll jumps while reading.

use leptos::prelude::*;
use leptos::html::Div;

use bw_shared::ChatChannel;

use crate::api::WsService;
use crate::state::GameState;

#[component]
pub fn CommsPanel() -> impl IntoView {
    let game_state = expect_context::<GameState>();
    let ws = expect_context::<WsService>();

    let input = RwSignal::new(String::new());
    let selected_channel = RwSignal::new(ChatChannel::Sector);

    // Track if user is at bottom of messages
    let is_at_bottom = RwSignal::new(true);
    let messages_ref = NodeRef::<Div>::new();

    // Get messages from game state
    let messages = move || game_state.chat_messages.get();

    // Check if player is in a squadron (to enable squadron chat)
    let in_squadron = move || game_state.in_squadron();

    // Scroll handler to track if user is at bottom
    let handle_scroll = move |_| {
        if let Some(el) = messages_ref.get() {
            let scroll_top = el.scroll_top();
            let scroll_height = el.scroll_height();
            let client_height = el.client_height();
            // Consider "at bottom" if within 50px of bottom
            let at_bottom = scroll_height - scroll_top - client_height < 50;
            is_at_bottom.set(at_bottom);
        }
    };

    // Scroll to bottom function
    let scroll_to_bottom = move |_| {
        if let Some(el) = messages_ref.get() {
            el.set_scroll_top(el.scroll_height());
            is_at_bottom.set(true);
        }
    };

    // Send message handler
    let send_message = move |_| {
        let msg = input.get();
        if msg.is_empty() {
            return;
        }

        let channel = selected_channel.get();
        ws.send_chat(msg, channel);
        input.set(String::new());
    };

    view! {
        <div class="h-full flex flex-col">
            // Header with channel selector
            <div class="p-2 border-b border-slate-700 flex items-center justify-between">
                <h3 class="text-sm font-semibold text-slate-300">"Comms"</h3>
                <div class="flex gap-1">
                    <ChannelButton
                        channel=ChatChannel::Sector
                        selected=selected_channel
                        label="Sector"
                        enabled=|| true
                    />
                    <ChannelButton
                        channel=ChatChannel::Squadron
                        selected=selected_channel
                        label="Squad"
                        enabled=in_squadron
                    />
                </div>
            </div>

            // Message list container
            <div class="flex-1 relative overflow-hidden">
                // Message list
                <div
                    node_ref=messages_ref
                    on:scroll=handle_scroll
                    class="h-full overflow-y-auto p-2 space-y-1 text-xs"
                >
                    <For
                        each=messages
                        key=|msg| msg.id
                        children=move |msg| {
                            // Pre-compute channel display info (computed once per message)
                            let (channel_color, channel_prefix) = match msg.channel {
                                ChatChannel::System => ("text-amber-400", "[SYS]"),
                                ChatChannel::Sector => ("text-blue-400", "[SEC]"),
                                ChatChannel::Squadron => ("text-purple-400", "[SQD]"),
                                ChatChannel::Direct => ("text-green-400", "[DM]"),
                            };

                            // Pre-compute class string to avoid format! on each render
                            let prefix_class = format!("flex-shrink-0 {}", channel_color);

                            let is_system = msg.is_system;
                            let sender_name = msg.sender_name.clone();
                            let message = msg.message.clone();

                            // Format timestamp as HH:MM in user's local timezone
                            let time_str = {
                                let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(msg.timestamp as f64));
                                let hours = date.get_hours();
                                let mins = date.get_minutes();
                                format!("{:02}:{:02}", hours, mins)
                            };

                            // Use if/else to avoid Show closure issues
                            if is_system {
                                view! {
                                    <div class="flex gap-1 leading-relaxed">
                                        <span class="text-slate-600 text-[10px] flex-shrink-0">{time_str}</span>
                                        <span class=prefix_class.clone()>{channel_prefix}</span>
                                        <span class="text-amber-400 italic">{message}</span>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="flex gap-1 leading-relaxed">
                                        <span class="text-slate-600 text-[10px] flex-shrink-0">{time_str}</span>
                                        <span class=prefix_class>{channel_prefix}</span>
                                        <span class="text-slate-400">{sender_name}":"</span>
                                        <span class="text-slate-200">{message}</span>
                                    </div>
                                }.into_any()
                            }
                        }
                    />

                    // Show empty state if no messages
                    <Show when=move || messages().is_empty()>
                        <div class="text-slate-500 italic text-center py-4">
                            "No messages yet."
                        </div>
                    </Show>
                </div>

                // Jump to bottom button
                <Show when=move || !is_at_bottom.get()>
                    <button
                        on:click=scroll_to_bottom
                        class="absolute bottom-2 left-1/2 -translate-x-1/2 px-3 py-1 bg-amber-600 hover:bg-amber-500
                               rounded-full text-xs text-white shadow-lg flex items-center gap-1 transition-all"
                    >
                        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M12 5v14m-7-7l7 7 7-7" />
                        </svg>
                        "New messages"
                    </button>
                </Show>
            </div>

            // Input area
            <div class="p-2 border-t border-slate-700">
                <div class="flex gap-2">
                    <input
                        type="text"
                        prop:value=move || input.get()
                        on:input=move |ev| input.set(event_target_value(&ev))
                        on:keypress=move |ev: web_sys::KeyboardEvent| {
                            if ev.key() == "Enter" {
                                send_message(());
                            }
                        }
                        class="flex-1 px-3 py-2 md:py-1 min-h-[44px] md:min-h-0 bg-slate-700 border border-slate-600 rounded text-sm
                               text-slate-100 focus:outline-none focus:border-amber-500"
                        placeholder=move || {
                            match selected_channel.get() {
                                ChatChannel::Sector => "Message sector...",
                                ChatChannel::Squadron => "Message squadron...",
                                ChatChannel::System => "System channel (read only)",
                                ChatChannel::Direct => "Direct message...",
                            }
                        }
                    />
                    <button
                        on:click=move |_| send_message(())
                        class="px-4 py-2 md:py-1 min-h-[44px] md:min-h-0 bg-amber-600 hover:bg-amber-500 rounded text-sm
                               disabled:opacity-50 disabled:cursor-not-allowed"
                        disabled=move || input.get().is_empty()
                    >
                        "Send"
                    </button>
                </div>

                // Channel indicator
                <div class="mt-1 text-[10px] text-slate-500">
                    "Sending to: "
                    <span class=move || {
                        match selected_channel.get() {
                            ChatChannel::Sector => "text-blue-400",
                            ChatChannel::Squadron => "text-purple-400",
                            ChatChannel::System => "text-amber-400",
                            ChatChannel::Direct => "text-green-400",
                        }
                    }>
                        {move || match selected_channel.get() {
                            ChatChannel::Sector => "Sector",
                            ChatChannel::Squadron => "Squadron",
                            ChatChannel::System => "System",
                            ChatChannel::Direct => "Direct",
                        }}
                    </span>
                </div>
            </div>
        </div>
    }
}

#[component]
fn ChannelButton<E>(
    channel: ChatChannel,
    selected: RwSignal<ChatChannel>,
    label: &'static str,
    enabled: E,
) -> impl IntoView
where
    E: Fn() -> bool + 'static + Clone + Send,
{
    let is_selected = move || selected.get() == channel;
    let enabled_click = enabled.clone();
    let enabled_class = enabled.clone();
    let enabled_disabled = enabled.clone();

    view! {
        <button
            on:click=move |_| {
                if enabled_click() {
                    selected.set(channel);
                }
            }
            class=move || {
                let base = "px-2 py-0.5 text-[10px] rounded transition-colors";
                let state = if is_selected() {
                    "bg-amber-600 text-amber-100"
                } else if enabled_class() {
                    "bg-slate-700 text-slate-400 hover:bg-slate-600"
                } else {
                    "bg-slate-800 text-slate-600 cursor-not-allowed"
                };
                format!("{} {}", base, state)
            }
            disabled=move || !enabled_disabled()
        >
            {label}
        </button>
    }
}
