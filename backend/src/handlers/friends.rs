use actix_web::{web, HttpRequest, HttpResponse};
use chrono::Utc;
use uuid::Uuid;

use crate::auth::require_auth;
use crate::models::{Friendship, FriendWithUser, User};
use crate::AppState;

pub async fn get_friends(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> HttpResponse {
    let current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    // Get accepted friendships where user is either user_id or friend_id
    let friendships = sqlx::query_as::<_, Friendship>(
        r#"
        SELECT * FROM friendships 
        WHERE (user_id = ? OR friend_id = ?) AND status = 'accepted'
        "#
    )
    .bind(&current_user.id)
    .bind(&current_user.id)
    .fetch_all(&state.db)
    .await;

    match friendships {
        Ok(friendships) => {
            let mut friends: Vec<FriendWithUser> = Vec::new();
            
            for friendship in friendships {
                let friend_id = if friendship.user_id == current_user.id {
                    &friendship.friend_id
                } else {
                    &friendship.user_id
                };

                if let Ok(Some(friend)) = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
                    .bind(friend_id)
                    .fetch_optional(&state.db)
                    .await
                {
                    friends.push(FriendWithUser {
                        friendship_id: friendship.id,
                        user: friend.into(),
                        status: friendship.status,
                        created_at: friendship.created_at,
                    });
                }
            }
            
            HttpResponse::Ok().json(friends)
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to get friends: {}", e)
        })),
    }
}

pub async fn get_friend_requests(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> HttpResponse {
    let current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    // Get pending friendships where current user is the friend (receiving the request)
    let friendships = sqlx::query_as::<_, Friendship>(
        "SELECT * FROM friendships WHERE friend_id = ? AND status = 'pending'"
    )
    .bind(&current_user.id)
    .fetch_all(&state.db)
    .await;

    match friendships {
        Ok(friendships) => {
            let mut requests: Vec<FriendWithUser> = Vec::new();
            
            for friendship in friendships {
                if let Ok(Some(user)) = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
                    .bind(&friendship.user_id)
                    .fetch_optional(&state.db)
                    .await
                {
                    requests.push(FriendWithUser {
                        friendship_id: friendship.id,
                        user: user.into(),
                        status: friendship.status,
                        created_at: friendship.created_at,
                    });
                }
            }
            
            HttpResponse::Ok().json(requests)
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to get friend requests: {}", e)
        })),
    }
}

pub async fn send_friend_request(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    let friend_id = path.into_inner();

    if current_user.id == friend_id {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Cannot send friend request to yourself"
        }));
    }

    // Check if friend exists
    let friend_exists = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(&friend_id)
        .fetch_optional(&state.db)
        .await;

    if let Ok(None) = friend_exists {
        return HttpResponse::NotFound().json(serde_json::json!({
            "error": "User not found"
        }));
    }

    // Check if friendship already exists
    let existing = sqlx::query_as::<_, Friendship>(
        "SELECT * FROM friendships WHERE (user_id = ? AND friend_id = ?) OR (user_id = ? AND friend_id = ?)"
    )
    .bind(&current_user.id)
    .bind(&friend_id)
    .bind(&friend_id)
    .bind(&current_user.id)
    .fetch_optional(&state.db)
    .await;

    if let Ok(Some(friendship)) = existing {
        return HttpResponse::Conflict().json(serde_json::json!({
            "error": "Friendship already exists",
            "status": friendship.status
        }));
    }

    let friendship_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let result = sqlx::query(
        "INSERT INTO friendships (id, user_id, friend_id, status, created_at) VALUES (?, ?, ?, 'pending', ?)"
    )
    .bind(&friendship_id)
    .bind(&current_user.id)
    .bind(&friend_id)
    .bind(&now)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => HttpResponse::Created().json(serde_json::json!({
            "message": "Friend request sent",
            "friendship_id": friendship_id
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to send friend request: {}", e)
        })),
    }
}

pub async fn accept_friend_request(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    let user_id = path.into_inner();

    // Find the pending friend request
    let friendship = sqlx::query_as::<_, Friendship>(
        "SELECT * FROM friendships WHERE user_id = ? AND friend_id = ? AND status = 'pending'"
    )
    .bind(&user_id)
    .bind(&current_user.id)
    .fetch_optional(&state.db)
    .await;

    match friendship {
        Ok(Some(friendship)) => {
            let result = sqlx::query("UPDATE friendships SET status = 'accepted' WHERE id = ?")
                .bind(&friendship.id)
                .execute(&state.db)
                .await;

            match result {
                Ok(_) => HttpResponse::Ok().json(serde_json::json!({
                    "message": "Friend request accepted"
                })),
                Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": format!("Failed to accept friend request: {}", e)
                })),
            }
        }
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Friend request not found"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Database error: {}", e)
        })),
    }
}

pub async fn reject_friend_request(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    let user_id = path.into_inner();

    let result = sqlx::query(
        "DELETE FROM friendships WHERE user_id = ? AND friend_id = ? AND status = 'pending'"
    )
    .bind(&user_id)
    .bind(&current_user.id)
    .execute(&state.db)
    .await;

    match result {
        Ok(r) => {
            if r.rows_affected() > 0 {
                HttpResponse::Ok().json(serde_json::json!({
                    "message": "Friend request rejected"
                }))
            } else {
                HttpResponse::NotFound().json(serde_json::json!({
                    "error": "Friend request not found"
                }))
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to reject friend request: {}", e)
        })),
    }
}
