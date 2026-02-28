use leptos::prelude::*;
use leptos::ev;

use crate::state::AppState;

/// Main chat area with message history, streaming display, and input.
#[component]
pub fn ChatArea() -> impl IntoView {
    let state = expect_context::<AppState>();

    view! {
        <main class="chat-area">
            // Error banner
            {move || {
                state.error.get().map(|err| {
                    view! {
                        <div class="error-banner">{err}</div>
                    }
                })
            }}

            // Chat header
            <div class="chat-header">
                {move || {
                    match state.active_conversation.get() {
                        Some(id) => format!("Conversation: {}", &id[..8.min(id.len())]),
                        None => "New conversation".to_string(),
                    }
                }}
            </div>

            // Messages
            <div class="messages-container">
                {move || {
                    let msgs = state.messages.get();
                    if msgs.is_empty() && state.streaming_text.get().is_none() {
                        view! {
                            <div class="empty-state">
                                "Send a message to start chatting"
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <For
                                each=move || state.messages.get()
                                key=|m| m.id.clone()
                                let:msg
                            >
                                <MessageBubble role=msg.role.clone() content=msg.content.clone() />
                            </For>
                            // Streaming message (assistant typing)
                            {move || {
                                state.streaming_text.get().map(|text| {
                                    view! {
                                        <div class="message assistant">
                                            <div class="role-label">"assistant"</div>
                                            <div class="streaming-cursor">{text}</div>
                                        </div>
                                    }
                                })
                            }}
                        }.into_any()
                    }
                }}
            </div>

            // Input area
            <ChatInput />
        </main>
    }
}

/// A single chat message bubble.
#[component]
fn MessageBubble(role: String, content: String) -> impl IntoView {
    let css_class = if role == "user" {
        "message user"
    } else {
        "message assistant"
    };
    let label = role.clone();

    view! {
        <div class=css_class>
            <div class="role-label">{label}</div>
            <div>{content}</div>
        </div>
    }
}

/// Chat input form with textarea and send button.
#[component]
fn ChatInput() -> impl IntoView {
    let state = expect_context::<AppState>();
    let (input, set_input) = signal(String::new());

    let is_sending = move || state.is_streaming.get();

    let send = move || {
        let text = input.get().trim().to_string();
        if text.is_empty() || is_sending() {
            return;
        }
        set_input.set(String::new());
        state.send_message(text);
    };

    let send_clone = send.clone();
    let on_keydown = move |ev: ev::KeyboardEvent| {
        if ev.key() == "Enter" && !ev.shift_key() {
            ev.prevent_default();
            send_clone();
        }
    };

    let on_submit = move |_| {
        send();
    };

    view! {
        <div class="input-area">
            <div class="input-row">
                <textarea
                    rows="1"
                    placeholder="Type a message… (Enter to send, Shift+Enter for newline)"
                    prop:value=input
                    on:input=move |ev| {
                        set_input.set(event_target_value(&ev));
                    }
                    on:keydown=on_keydown
                    disabled=is_sending
                />
                <button
                    class="send-btn"
                    on:click=on_submit
                    disabled=move || is_sending() || input.get().trim().is_empty()
                >
                    {move || if is_sending() { "Sending…" } else { "Send" }}
                </button>
            </div>
        </div>
    }
}
