use leptos::prelude::*;

use crate::state::AppState;

/// Sidebar showing conversation list and "New Chat" button.
#[component]
pub fn Sidebar() -> impl IntoView {
    let state = expect_context::<AppState>();

    let on_new = move |_| {
        state.set_active_conversation.set(None);
        state.set_messages.set(Vec::new());
        state.set_streaming_text.set(None);
    };

    view! {
        <aside class="sidebar">
            <div class="sidebar-header">
                <h2>"Rust AI Chat"</h2>
                <button class="new-chat-btn" on:click=on_new>
                    "+ New Chat"
                </button>
            </div>
            <div class="conversation-list">
                {move || {
                    let convos = state.conversations.get();
                    if convos.is_empty() {
                        view! {
                            <div style="padding:1rem;color:var(--text-secondary);font-size:0.85rem">
                                "No conversations yet"
                            </div>
                        }.into_any()
                    } else {
                        let state = state.clone();
                        view! {
                            <For
                                each=move || state.conversations.get()
                                key=|c| c.id.clone()
                                let:conv
                            >
                                {
                                    let state = state.clone();
                                    let id = conv.id.clone();
                                    let title = conv.title.clone()
                                        .unwrap_or_else(|| "Untitled chat".to_string());
                                    let id_click = id.clone();
                                    let id_active = id.clone();
                                    view! {
                                        <div
                                            class="conversation-item"
                                            class:active=move || {
                                                state.active_conversation.get().as_deref() == Some(id_active.as_str())
                                            }
                                            on:click=move |_| {
                                                state.select_conversation(id_click.clone());
                                            }
                                        >
                                            {title}
                                        </div>
                                    }
                                }
                            </For>
                        }.into_any()
                    }
                }}
            </div>
        </aside>
    }
}

