use actix_web::{web, HttpRequest, HttpResponse, Error};
use actix_ws::Message;
use chrono::Utc;
use futures::StreamExt;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::auth::verify_token;
use crate::models::{MessageResponse, WsMessage};
use crate::AppState;

pub async fn chat_ws(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let (res, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    let state_clone = state.clone();
    
    // Create a channel for sending messages to this session
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    
    actix_rt::spawn(async move {
        let mut authenticated_user_id: Option<String> = None;

        loop {
            tokio::select! {
                // Handle incoming WebSocket messages
                msg_opt = msg_stream.next() => {
                    match msg_opt {
                        Some(Ok(msg)) => {
                            match msg {
                                Message::Text(text) => {
                                    let ws_msg: Result<WsMessage, _> = serde_json::from_str(&text);
                                    
                                    match ws_msg {
                                        Ok(WsMessage::Auth { token }) => {
                                            match verify_token(&token, &state_clone.jwt_secret) {
                                                Ok(claims) => {
                                                    authenticated_user_id = Some(claims.sub.clone());
                                                    
                                                    // Store connection channel
                                                    {
                                                        let mut connections = state_clone.ws_connections.write().await;
                                                        connections
                                                            .entry(claims.sub.clone())
                                                            .or_insert_with(Vec::new)
                                                            .push(tx.clone());
                                                    }

                                                    let response = WsMessage::Connected { user_id: claims.sub };
                                                    let _ = session.text(serde_json::to_string(&response).unwrap()).await;
                                                }
                                                Err(_) => {
                                                    let response = WsMessage::Error { message: "Invalid token".to_string() };
                                                    let _ = session.text(serde_json::to_string(&response).unwrap()).await;
                                                }
                                            }
                                        }
                                        Ok(WsMessage::ChatMessage { receiver_id, encrypted_content, iv }) => {
                                            if let Some(ref sender_id) = authenticated_user_id {
                                                // Store message in database
                                                let message_id = Uuid::new_v4().to_string();
                                                let now = Utc::now().to_rfc3339();

                                                let result = sqlx::query(
                                                    "INSERT INTO messages (id, sender_id, receiver_id, encrypted_content, iv, created_at, is_read) VALUES (?, ?, ?, ?, ?, ?, 0)"
                                                )
                                                .bind(&message_id)
                                                .bind(sender_id)
                                                .bind(&receiver_id)
                                                .bind(&encrypted_content)
                                                .bind(&iv)
                                                .bind(&now)
                                                .execute(&state_clone.db)
                                                .await;

                                                if result.is_ok() {
                                                    let message_response = MessageResponse {
                                                        id: message_id.clone(),
                                                        sender_id: sender_id.clone(),
                                                        receiver_id: receiver_id.clone(),
                                                        encrypted_content: encrypted_content.clone(),
                                                        iv: iv.clone(),
                                                        created_at: now.clone(),
                                                        is_read: false,
                                                    };

                                                    let ws_response = WsMessage::MessageReceived { 
                                                        message: message_response 
                                                    };
                                                    let response_json = serde_json::to_string(&ws_response).unwrap();

                                                    // Send to receiver if online
                                                    {
                                                        let connections = state_clone.ws_connections.read().await;
                                                        if let Some(receiver_channels) = connections.get(&receiver_id) {
                                                            for channel in receiver_channels {
                                                                let _ = channel.send(response_json.clone());
                                                            }
                                                        }
                                                    }

                                                    // Also send confirmation back to sender
                                                    let _ = session.text(response_json).await;
                                                } else {
                                                    let response = WsMessage::Error { message: "Failed to send message".to_string() };
                                                    let _ = session.text(serde_json::to_string(&response).unwrap()).await;
                                                }
                                            } else {
                                                let response = WsMessage::Error { message: "Not authenticated".to_string() };
                                                let _ = session.text(serde_json::to_string(&response).unwrap()).await;
                                            }
                                        }
                                        Ok(WsMessage::Typing { receiver_id }) => {
                                            if let Some(ref sender_id) = authenticated_user_id {
                                                let connections = state_clone.ws_connections.read().await;
                                                if let Some(receiver_channels) = connections.get(&receiver_id) {
                                                    let response = WsMessage::TypingIndicator { sender_id: sender_id.clone() };
                                                    let response_json = serde_json::to_string(&response).unwrap();
                                                    for channel in receiver_channels {
                                                        let _ = channel.send(response_json.clone());
                                                    }
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                Message::Ping(bytes) => {
                                    let _ = session.pong(&bytes).await;
                                }
                                Message::Close(_) => {
                                    break;
                                }
                                _ => {}
                            }
                        }
                        Some(Err(_)) | None => break,
                    }
                }
                // Handle outgoing messages from channel
                Some(msg) = rx.recv() => {
                    let _ = session.text(msg).await;
                }
            }
        }

        // Cleanup on disconnect - remove this connection's channel
        if let Some(ref user_id) = authenticated_user_id {
            let mut connections = state_clone.ws_connections.write().await;
            connections.remove(user_id);
        }
    });

    Ok(res)
}
