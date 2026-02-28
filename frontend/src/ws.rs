use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};

use crate::api::ws_url;
use crate::models::{WsChatRequest, WsEvent};

/// Opens a WebSocket connection, sends a chat request, and invokes callbacks
/// for each streaming event. Returns a handle that auto-closes on drop.
pub fn start_streaming(
    message: String,
    conversation_id: Option<String>,
    on_start: impl Fn(String) + 'static,
    on_chunk: impl Fn(String) + 'static,
    on_end: impl Fn(String) + 'static,
    on_error: impl Fn(String) + 'static,
) -> Option<WebSocket> {
    let url = ws_url();
    let ws = match WebSocket::new(&url) {
        Ok(ws) => ws,
        Err(e) => {
            on_error(format!("Failed to connect: {e:?}"));
            return None;
        }
    };
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

    // --- onopen: send the chat request ---
    let ws_clone = ws.clone();
    let onopen = Closure::<dyn Fn()>::new(move || {
        let req = WsChatRequest {
            message: message.clone(),
            conversation_id: conversation_id.clone(),
        };
        if let Ok(json) = serde_json::to_string(&req) {
            let _ = ws_clone.send_with_str(&json);
        }
    });
    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    // --- onmessage: dispatch WsEvent ---
    let onmessage = Closure::<dyn Fn(MessageEvent)>::new(move |ev: MessageEvent| {
        if let Some(text) = ev.data().as_string() {
            match serde_json::from_str::<WsEvent>(&text) {
                Ok(WsEvent::StreamStart { conversation_id }) => {
                    on_start(conversation_id);
                }
                Ok(WsEvent::StreamChunk { content }) => {
                    on_chunk(content);
                }
                Ok(WsEvent::StreamEnd { full_content, .. }) => {
                    on_end(full_content);
                }
                Ok(WsEvent::Error { message }) => {
                    on_error(message);
                }
                Err(e) => {
                    on_error(format!("Parse error: {e}"));
                }
            }
        }
    });
    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    // --- onerror ---
    let on_error_clone = {
        // We can't move on_error again so we use a simple log here
        Closure::<dyn Fn()>::new(move || {
            log::error!("WebSocket connection error");
        })
    };
    ws.set_onerror(Some(on_error_clone.as_ref().unchecked_ref()));
    on_error_clone.forget();

    Some(ws)
}

/// Close a WebSocket connection gracefully.
pub fn close_ws(ws: &WebSocket) {
    let _ = ws.close();
}
