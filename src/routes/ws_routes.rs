use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use tracing::{error, info, warn};

use crate::models::{ChatRequest, WsChatRequest, WsEvent};
use crate::service::chat_service::ChatService;

/// GET `/ws/chat` — upgrades to a WebSocket for streaming chat.
pub async fn ws_chat_handler(
    ws: WebSocketUpgrade,
    State(svc): State<ChatService>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, svc))
}

/// Handles a single WebSocket connection.
///
/// Protocol:
/// - Client sends JSON `{ "conversation_id": "...|null", "message": "..." }`
/// - Server streams back:
///   1. `{ "type": "stream_start", "conversation_id": "..." }`
///   2. `{ "type": "stream_chunk", "content": "..." }` (repeated)
///   3. `{ "type": "stream_end",   "message_id": "..." }`
///   or `{ "type": "error", "message": "..." }` on failure.
async fn handle_socket(mut socket: WebSocket, svc: ChatService) {
    info!("WebSocket client connected");

    while let Some(msg) = socket.recv().await {
        let msg = match msg {
            Ok(m) => m,
            Err(e) => {
                warn!("WebSocket receive error: {e}");
                break;
            }
        };

        // Only handle text messages
        let text = match &msg {
            Message::Text(t) => t.to_string(),
            Message::Close(_) => break,
            _ => continue,
        };

        // Parse the incoming request
        let ws_req: WsChatRequest = match serde_json::from_str(&text) {
            Ok(r) => r,
            Err(e) => {
                send_event(&mut socket, &WsEvent::Error {
                    message: format!("Invalid request: {e}"),
                }).await;
                continue;
            }
        };

        // Build a ChatRequest for the service layer
        let chat_request = ChatRequest {
            conversation_id: ws_req.conversation_id,
            message: ws_req.message,
        };

        // ── Prepare: validate, resolve conversation, save user message ────
        let ctx = match svc.prepare_chat(chat_request).await {
            Ok(ctx) => ctx,
            Err(e) => {
                send_event(&mut socket, &WsEvent::Error {
                    message: e.to_string(),
                }).await;
                continue;
            }
        };

        // ── Notify client: streaming is starting ─────────────────────────
        send_event(&mut socket, &WsEvent::StreamStart {
            conversation_id: ctx.conversation_id.clone(),
        }).await;

        // ── Stream tokens from Ollama via a channel ──────────────────────
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(64);
        let agent = svc.agent().clone();
        let conv_id = ctx.conversation_id.clone();
        let history = ctx.history.clone();
        let user_msg = ctx.user_message.clone();

        let stream_handle = tokio::spawn(async move {
            agent.stream_chat(&conv_id, &history, &user_msg, tx).await
        });

        // Forward each chunk to the WebSocket client
        let mut full_content = String::new();
        while let Some(chunk) = rx.recv().await {
            full_content.push_str(&chunk);
            send_event(&mut socket, &WsEvent::StreamChunk {
                content: chunk,
            }).await;
        }

        // Wait for the agent task to finish
        match stream_handle.await {
            Ok(Ok(())) => {
                // Persist the complete assistant message
                match svc.save_assistant_message(&ctx.conversation_id, &full_content).await {
                    Ok(msg) => {
                        send_event(&mut socket, &WsEvent::StreamEnd {
                            message_id: msg.id,
                            full_content: full_content.clone(),
                        }).await;
                    }
                    Err(e) => {
                        error!("Failed to save assistant message: {e}");
                        send_event(&mut socket, &WsEvent::Error {
                            message: format!("Failed to save response: {e}"),
                        }).await;
                    }
                }
            }
            Ok(Err(e)) => {
                error!("Agent streaming failed: {e}");
                send_event(&mut socket, &WsEvent::Error {
                    message: e.to_string(),
                }).await;
            }
            Err(e) => {
                error!("Agent task panicked: {e}");
                send_event(&mut socket, &WsEvent::Error {
                    message: "Internal error during streaming".to_string(),
                }).await;
            }
        }
    }

    info!("WebSocket client disconnected");
}

/// Helper: serialize a `WsEvent` and send it over the socket.
async fn send_event(socket: &mut WebSocket, event: &WsEvent) {
    if let Ok(json) = serde_json::to_string(event) {
        let _ = socket.send(Message::Text(json.into())).await;
    }
}
