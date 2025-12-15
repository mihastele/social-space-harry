mod models;
mod handlers;
mod db;
mod auth;
mod websocket;

use actix_cors::Cors;
use actix_web::{web, App, HttpServer, middleware};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use std::collections::HashMap;

pub struct AppState {
    pub db: SqlitePool,
    pub jwt_secret: String,
    pub ws_connections: Arc<RwLock<HashMap<String, Vec<mpsc::UnboundedSender<String>>>>>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting Social Space Backend...");
    
    // Initialize database
    let db = db::init_db().await.expect("Failed to initialize database");
    
    let app_state = web::Data::new(AppState {
        db,
        jwt_secret: std::env::var("JWT_SECRET").unwrap_or_else(|_| "super_secret_key_change_in_production".to_string()),
        ws_connections: Arc::new(RwLock::new(HashMap::new())),
    });

    println!("Server running at http://localhost:8080");
    
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .supports_credentials();

        App::new()
            .app_data(app_state.clone())
            .wrap(cors)
            .wrap(middleware::Logger::default())
            // Auth routes
            .route("/api/auth/register", web::post().to(handlers::auth::register))
            .route("/api/auth/login", web::post().to(handlers::auth::login))
            .route("/api/auth/me", web::get().to(handlers::auth::get_me))
            // User routes
            .route("/api/users", web::get().to(handlers::users::search_users))
            .route("/api/users/{id}", web::get().to(handlers::users::get_user))
            // Friend routes
            .route("/api/friends", web::get().to(handlers::friends::get_friends))
            .route("/api/friends/requests", web::get().to(handlers::friends::get_friend_requests))
            .route("/api/friends/request/{user_id}", web::post().to(handlers::friends::send_friend_request))
            .route("/api/friends/accept/{user_id}", web::post().to(handlers::friends::accept_friend_request))
            .route("/api/friends/reject/{user_id}", web::post().to(handlers::friends::reject_friend_request))
            // Post routes
            .route("/api/posts", web::get().to(handlers::posts::get_feed))
            .route("/api/posts", web::post().to(handlers::posts::create_post))
            .route("/api/posts/{id}", web::get().to(handlers::posts::get_post))
            .route("/api/posts/{id}", web::delete().to(handlers::posts::delete_post))
            .route("/api/posts/{id}/like", web::post().to(handlers::posts::like_post))
            .route("/api/posts/{id}/comment", web::post().to(handlers::posts::add_comment))
            .route("/api/posts/{id}/comments", web::get().to(handlers::posts::get_comments))
            // Group routes
            .route("/api/groups", web::get().to(handlers::groups::get_groups))
            .route("/api/groups", web::post().to(handlers::groups::create_group))
            .route("/api/groups/{id}", web::get().to(handlers::groups::get_group))
            .route("/api/groups/{id}/join", web::post().to(handlers::groups::join_group))
            .route("/api/groups/{id}/leave", web::post().to(handlers::groups::leave_group))
            .route("/api/groups/{id}/posts", web::get().to(handlers::groups::get_group_posts))
            .route("/api/groups/{id}/posts", web::post().to(handlers::groups::create_group_post))
            // Chat routes
            .route("/api/chat/conversations", web::get().to(handlers::chat::get_conversations))
            .route("/api/chat/messages/{user_id}", web::get().to(handlers::chat::get_messages))
            .route("/api/chat/keys/{user_id}", web::get().to(handlers::chat::get_public_key))
            .route("/api/chat/keys", web::post().to(handlers::chat::store_public_key))
            .route("/ws/chat", web::get().to(websocket::chat_ws))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
