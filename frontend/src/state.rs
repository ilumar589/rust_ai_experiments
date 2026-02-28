use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::models::{Conversation, Message};
use crate::ws;

/// Shared application state, provided via Leptos context.
#[derive(Clone)]
pub struct AppState {
    // --- Read signals (for components to subscribe to) ---
    pub conversations: ReadSignal<Vec<Conversation>>,
    pub active_conversation: ReadSignal<Option<String>>,
    pub messages: ReadSignal<Vec<Message>>,
    pub streaming_text: ReadSignal<Option<String>>,
    pub is_streaming: ReadSignal<bool>,
    pub error: ReadSignal<Option<String>>,

    // --- Write signals (for mutating state) ---
    pub set_conversations: WriteSignal<Vec<Conversation>>,
    pub set_active_conversation: WriteSignal<Option<String>>,
    pub set_messages: WriteSignal<Vec<Message>>,
    pub set_streaming_text: WriteSignal<Option<String>>,
    pub set_is_streaming: WriteSignal<bool>,
    pub set_error: WriteSignal<Option<String>>,
}

impl AppState {
    /// Create a new `AppState` and provide it in the current Leptos context.
    pub fn provide() -> Self {
        let (conversations, set_conversations) = signal(Vec::<Conversation>::new());
        let (active_conversation, set_active_conversation) = signal(None::<String>);
        let (messages, set_messages) = signal(Vec::<Message>::new());
        let (streaming_text, set_streaming_text) = signal(None::<String>);
        let (is_streaming, set_is_streaming) = signal(false);
        let (error, set_error) = signal(None::<String>);

        let state = Self {
            conversations,
            active_conversation,
            messages,
            streaming_text,
            is_streaming,
            error,
            set_conversations,
            set_active_conversation,
            set_messages,
            set_streaming_text,
            set_is_streaming,
            set_error,
        };

        provide_context(state.clone());
        state
    }

    /// Load conversations from the backend.
    pub fn load_conversations(&self) {
        let state = self.clone();
        spawn_local(async move {
            match api::fetch_conversations().await {
                Ok(convos) => state.set_conversations.set(convos),
                Err(e) => {
                    log::error!("Failed to fetch conversations: {e}");
                    state.set_error.set(Some(e));
                }
            }
        });
    }

    /// Select a conversation and load its messages.
    pub fn select_conversation(&self, id: String) {
        let state = self.clone();
        self.set_active_conversation.set(Some(id.clone()));
        self.set_streaming_text.set(None);
        self.set_error.set(None);

        spawn_local(async move {
            match api::fetch_messages(&id).await {
                Ok(msgs) => state.set_messages.set(msgs),
                Err(e) => {
                    log::error!("Failed to fetch messages: {e}");
                    state.set_error.set(Some(e));
                }
            }
        });
    }

    /// Send a message via WebSocket streaming.
    pub fn send_message(&self, text: String) {
        let state = self.clone();
        let conv_id = self.active_conversation.get_untracked();

        // Optimistically add the user message to the display
        let temp_user_msg = Message {
            id: format!("temp-{}", js_sys::Date::now() as u64),
            conversation_id: conv_id.clone().unwrap_or_default(),
            role: "user".to_string(),
            content: text.clone(),
            created_at: String::new(),
        };
        self.set_messages.update(|msgs| msgs.push(temp_user_msg));
        self.set_is_streaming.set(true);
        self.set_streaming_text.set(Some(String::new()));
        self.set_error.set(None);

        let set_active = self.set_active_conversation;
        let set_streaming = self.set_streaming_text;
        let set_is_streaming = self.set_is_streaming;
        let set_messages = self.set_messages;
        let set_error = self.set_error;

        // Callbacks to update state from WebSocket events
        let on_start = move |new_conv_id: String| {
            set_active.set(Some(new_conv_id.clone()));
            // Update the temp user message's conversation_id
            set_messages.update(|msgs| {
                for m in msgs.iter_mut() {
                    if m.conversation_id.is_empty() {
                        m.conversation_id = new_conv_id.clone();
                    }
                }
            });
        };

        let on_chunk = move |chunk: String| {
            set_streaming.update(|current| {
                if let Some(text) = current {
                    text.push_str(&chunk);
                }
            });
        };

        let st2 = state.clone();
        let on_end = move |full_content: String| {
            // Convert streaming text into a proper assistant message
            let conv = state.active_conversation.get_untracked().unwrap_or_default();
            let assistant_msg = Message {
                id: format!("msg-{}", js_sys::Date::now() as u64),
                conversation_id: conv,
                role: "assistant".to_string(),
                content: full_content,
                created_at: String::new(),
            };
            set_messages.update(|msgs| msgs.push(assistant_msg));
            set_streaming.set(None);
            set_is_streaming.set(false);

            // Refresh conversations list to pick up any new/updated ones
            st2.load_conversations();
        };

        let on_error = move |err: String| {
            log::error!("WebSocket error: {err}");
            set_error.set(Some(err));
            set_streaming.set(None);
            set_is_streaming.set(false);
        };

        ws::start_streaming(text, conv_id, on_start, on_chunk, on_end, on_error);
    }
}
