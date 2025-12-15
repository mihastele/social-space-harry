use actix_web::{web, HttpRequest, HttpResponse};
use chrono::Utc;

use crate::auth::require_auth;
use crate::models::{ConversationResponse, Message, MessageResponse, StorePublicKeyRequest, User, UserPublicKey};
use crate::AppState;

pub async fn get_conversations(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> HttpResponse {
    let current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    // Get unique conversation partners
    let conversations = sqlx::query_as::<_, Message>(
        r#"
        SELECT m1.* FROM messages m1
        INNER JOIN (
            SELECT 
                CASE WHEN sender_id = ? THEN receiver_id ELSE sender_id END as partner_id,
                MAX(created_at) as max_created
            FROM messages
            WHERE sender_id = ? OR receiver_id = ?
            GROUP BY partner_id
        ) m2 ON (
            (m1.sender_id = ? AND m1.receiver_id = m2.partner_id) OR
            (m1.receiver_id = ? AND m1.sender_id = m2.partner_id)
        ) AND m1.created_at = m2.max_created
        ORDER BY m1.created_at DESC
        "#
    )
    .bind(&current_user.id)
    .bind(&current_user.id)
    .bind(&current_user.id)
    .bind(&current_user.id)
    .bind(&current_user.id)
    .fetch_all(&state.db)
    .await;

    match conversations {
        Ok(messages) => {
            let mut conversation_responses: Vec<ConversationResponse> = Vec::new();
            
            for message in messages {
                let partner_id = if message.sender_id == current_user.id {
                    &message.receiver_id
                } else {
                    &message.sender_id
                };

                if let Ok(Some(partner)) = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
                    .bind(partner_id)
                    .fetch_optional(&state.db)
                    .await
                {
                    let unread_count: i32 = sqlx::query_scalar(
                        "SELECT COUNT(*) FROM messages WHERE sender_id = ? AND receiver_id = ? AND is_read = 0"
                    )
                    .bind(partner_id)
                    .bind(&current_user.id)
                    .fetch_one(&state.db)
                    .await
                    .unwrap_or(0);

                    conversation_responses.push(ConversationResponse {
                        user: partner.into(),
                        last_message: Some(MessageResponse {
                            id: message.id,
                            sender_id: message.sender_id,
                            receiver_id: message.receiver_id,
                            encrypted_content: message.encrypted_content,
                            iv: message.iv,
                            created_at: message.created_at,
                            is_read: message.is_read,
                        }),
                        unread_count,
                    });
                }
            }
            
            HttpResponse::Ok().json(conversation_responses)
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to get conversations: {}", e)
        })),
    }
}

pub async fn get_messages(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    let other_user_id = path.into_inner();

    // Mark messages as read
    let _ = sqlx::query(
        "UPDATE messages SET is_read = 1 WHERE sender_id = ? AND receiver_id = ?"
    )
    .bind(&other_user_id)
    .bind(&current_user.id)
    .execute(&state.db)
    .await;

    let messages = sqlx::query_as::<_, Message>(
        r#"
        SELECT * FROM messages 
        WHERE (sender_id = ? AND receiver_id = ?) OR (sender_id = ? AND receiver_id = ?)
        ORDER BY created_at ASC
        LIMIT 100
        "#
    )
    .bind(&current_user.id)
    .bind(&other_user_id)
    .bind(&other_user_id)
    .bind(&current_user.id)
    .fetch_all(&state.db)
    .await;

    match messages {
        Ok(messages) => {
            let message_responses: Vec<MessageResponse> = messages
                .into_iter()
                .map(|m| MessageResponse {
                    id: m.id,
                    sender_id: m.sender_id,
                    receiver_id: m.receiver_id,
                    encrypted_content: m.encrypted_content,
                    iv: m.iv,
                    created_at: m.created_at,
                    is_read: m.is_read,
                })
                .collect();
            
            HttpResponse::Ok().json(message_responses)
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to get messages: {}", e)
        })),
    }
}

pub async fn get_public_key(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let _current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    let user_id = path.into_inner();

    let key = sqlx::query_as::<_, UserPublicKey>(
        "SELECT * FROM user_public_keys WHERE user_id = ?"
    )
    .bind(&user_id)
    .fetch_optional(&state.db)
    .await;

    match key {
        Ok(Some(key)) => HttpResponse::Ok().json(serde_json::json!({
            "public_key": key.public_key
        })),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Public key not found for user"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Database error: {}", e)
        })),
    }
}

pub async fn store_public_key(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<StorePublicKeyRequest>,
) -> HttpResponse {
    let current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    let now = Utc::now().to_rfc3339();

    // Upsert public key
    let result = sqlx::query(
        r#"
        INSERT INTO user_public_keys (user_id, public_key, created_at) 
        VALUES (?, ?, ?)
        ON CONFLICT(user_id) DO UPDATE SET public_key = ?, created_at = ?
        "#
    )
    .bind(&current_user.id)
    .bind(&body.public_key)
    .bind(&now)
    .bind(&body.public_key)
    .bind(&now)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Public key stored successfully"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to store public key: {}", e)
        })),
    }
}
