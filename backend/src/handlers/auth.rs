use actix_web::{web, HttpRequest, HttpResponse};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::Utc;
use uuid::Uuid;

use crate::auth::{create_token, require_auth};
use crate::models::{AuthResponse, LoginRequest, RegisterRequest, User, UserResponse};
use crate::AppState;

pub async fn register(
    state: web::Data<AppState>,
    body: web::Json<RegisterRequest>,
) -> HttpResponse {
    // Validate input
    if body.email.is_empty() || body.password.is_empty() || body.username.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Email, password, and username are required"
        }));
    }

    if body.password.len() < 6 {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Password must be at least 6 characters"
        }));
    }

    // Check if email already exists
    let existing_email = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
        .bind(&body.email)
        .fetch_optional(&state.db)
        .await;

    if let Ok(Some(_)) = existing_email {
        return HttpResponse::Conflict().json(serde_json::json!({
            "error": "Email already registered"
        }));
    }

    // Check if username already exists
    let existing_username = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
        .bind(&body.username)
        .fetch_optional(&state.db)
        .await;

    if let Ok(Some(_)) = existing_username {
        return HttpResponse::Conflict().json(serde_json::json!({
            "error": "Username already taken"
        }));
    }

    // Hash password
    let password_hash = match hash(&body.password, DEFAULT_COST) {
        Ok(h) => h,
        Err(_) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to hash password"
            }))
        }
    };

    let user_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    // Insert user
    let result = sqlx::query(
        "INSERT INTO users (id, email, password_hash, username, display_name, created_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&user_id)
    .bind(&body.email)
    .bind(&password_hash)
    .bind(&body.username)
    .bind(&body.display_name)
    .bind(&now)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => {
            let token = create_token(&user_id, &state.jwt_secret).unwrap();
            
            HttpResponse::Created().json(AuthResponse {
                token,
                user: UserResponse {
                    id: user_id,
                    email: body.email.clone(),
                    username: body.username.clone(),
                    display_name: body.display_name.clone(),
                    avatar_url: None,
                    bio: None,
                },
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to create user: {}", e)
        })),
    }
}

pub async fn login(
    state: web::Data<AppState>,
    body: web::Json<LoginRequest>,
) -> HttpResponse {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
        .bind(&body.email)
        .fetch_optional(&state.db)
        .await;

    match user {
        Ok(Some(user)) => {
            if verify(&body.password, &user.password_hash).unwrap_or(false) {
                let token = create_token(&user.id, &state.jwt_secret).unwrap();
                
                HttpResponse::Ok().json(AuthResponse {
                    token,
                    user: user.into(),
                })
            } else {
                HttpResponse::Unauthorized().json(serde_json::json!({
                    "error": "Invalid credentials"
                }))
            }
        }
        Ok(None) => HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Invalid credentials"
        })),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Database error"
        })),
    }
}

pub async fn get_me(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> HttpResponse {
    match require_auth(&req, &state).await {
        Ok(user) => HttpResponse::Ok().json(UserResponse::from(user)),
        Err(e) => HttpResponse::Unauthorized().json(serde_json::json!({
            "error": e.to_string()
        })),
    }
}
