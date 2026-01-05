//! Communications panel component

use leptos::prelude::*;

#[component]
pub fn CommsPanel() -> impl IntoView {
    let messages = RwSignal::new(vec![
        ChatMessage {
            id: 0,
            sender: "SYSTEM".to_string(),
            content: "Welcome to Thornwick Sector.".to_string(),
            channel: "system".to_string(),
        },
        ChatMessage {
            id: 1,
            sender: "Harbormaster Kell".to_string(),
            content: "Docking services available. Stay safe out there.".to_string(),
            channel: "local".to_string(),
        },
    ]);

    let input = RwSignal::new(String::new());
    let next_id = RwSignal::new(2u64);

    let send_message = move |_: ()| {
        let msg = input.get();
        if msg.is_empty() {
            return;
        }

        let id = next_id.get();
        next_id.set(id + 1);

        messages.update(|msgs| {
            msgs.push(ChatMessage {
                id,
                sender: "You".to_string(),
                content: msg,
                channel: "local".to_string(),
            });
        });

        input.set(String::new());
    };

    view! {
        <div class="h-full flex flex-col">
            <div class="p-2 border-b border-slate-700">
                <h3 class="text-sm font-semibold text-slate-300">"Comms"</h3>
            </div>

            <div class="flex-1 overflow-y-auto p-2 space-y-2 text-xs">
                {move || {
                    messages.get().into_iter().map(|msg| {
                        let color = match msg.channel.as_str() {
                            "system" => "text-amber-400",
                            "local" => "text-blue-400",
                            _ => "text-slate-400",
                        };
                        view! {
                            <div class="flex gap-2">
                                <span class={color}>
                                    "["{msg.sender}"]"
                                </span>
                                <span class="text-slate-300">{msg.content}</span>
                            </div>
                        }
                    }).collect_view()
                }}
            </div>

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
                        class="flex-1 px-2 py-1 bg-slate-700 border border-slate-600 rounded text-sm text-slate-100 focus:outline-none focus:border-amber-500"
                        placeholder="Type message..."
                    />
                    <button
                        on:click=move |_| send_message(())
                        class="px-3 py-1 bg-amber-600 hover:bg-amber-500 rounded text-sm"
                    >
                        "Send"
                    </button>
                </div>
            </div>
        </div>
    }
}

#[derive(Clone)]
struct ChatMessage {
    id: u64,
    sender: String,
    content: String,
    channel: String,
}
