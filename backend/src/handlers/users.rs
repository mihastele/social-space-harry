use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;

use crate::auth::require_auth;
use crate::models::{User, UserResponse};
use crate::AppState;

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

pub async fn search_users(
    req: HttpRequest,
    state: web::Data<AppState>,
    query: web::Query<SearchQuery>,
) -> HttpResponse {
    let current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    let search_term = query.q.clone().unwrap_or_default();
    
    let users = if search_term.is_empty() {
        sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE id != ? LIMIT 50"
        )
        .bind(&current_user.id)
        .fetch_all(&state.db)
        .await
    } else {
        let search_pattern = format!("%{}%", search_term);
        sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE id != ? AND (username LIKE ? OR display_name LIKE ?) LIMIT 50"
        )
        .bind(&current_user.id)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .fetch_all(&state.db)
        .await
    };

    match users {
        Ok(users) => {
            let user_responses: Vec<UserResponse> = users.into_iter().map(|u| u.into()).collect();
            HttpResponse::Ok().json(user_responses)
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to search users: {}", e)
        })),
    }
}

pub async fn get_user(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let _current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    let user_id = path.into_inner();

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_optional(&state.db)
        .await;

    match user {
        Ok(Some(user)) => HttpResponse::Ok().json(UserResponse::from(user)),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "User not found"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Database error: {}", e)
        })),
    }
}
