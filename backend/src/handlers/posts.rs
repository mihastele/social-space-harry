use actix_web::{web, HttpRequest, HttpResponse};
use chrono::Utc;
use uuid::Uuid;

use crate::auth::require_auth;
use crate::models::{Comment, CommentResponse, CreateCommentRequest, CreatePostRequest, Like, Post, PostResponse, User, Friendship};
use crate::AppState;

pub async fn get_feed(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> HttpResponse {
    let current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    // Get friend IDs
    let friendships = sqlx::query_as::<_, Friendship>(
        "SELECT * FROM friendships WHERE (user_id = ? OR friend_id = ?) AND status = 'accepted'"
    )
    .bind(&current_user.id)
    .bind(&current_user.id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let friend_ids: Vec<String> = friendships
        .iter()
        .map(|f| {
            if f.user_id == current_user.id {
                f.friend_id.clone()
            } else {
                f.user_id.clone()
            }
        })
        .collect();

    // Get posts: own posts + friends' posts (friends_only or public) + public posts
    let posts = sqlx::query_as::<_, Post>(
        r#"
        SELECT * FROM posts 
        WHERE group_id IS NULL AND (
            user_id = ? 
            OR visibility = 'public'
            OR (visibility = 'friends_only' AND user_id IN (SELECT value FROM json_each(?)))
        )
        ORDER BY created_at DESC
        LIMIT 50
        "#
    )
    .bind(&current_user.id)
    .bind(serde_json::to_string(&friend_ids).unwrap_or_else(|_| "[]".to_string()))
    .fetch_all(&state.db)
    .await;

    match posts {
        Ok(posts) => {
            let mut post_responses: Vec<PostResponse> = Vec::new();
            
            for post in posts {
                let post_response = build_post_response(&state, &post, &current_user.id).await;
                post_responses.push(post_response);
            }
            
            HttpResponse::Ok().json(post_responses)
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to get feed: {}", e)
        })),
    }
}

pub async fn create_post(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<CreatePostRequest>,
) -> HttpResponse {
    let current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    if body.content.trim().is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Post content cannot be empty"
        }));
    }

    let post_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let visibility = body.visibility.clone().unwrap_or_else(|| "friends_only".to_string());

    let result = sqlx::query(
        "INSERT INTO posts (id, user_id, content, visibility, is_anonymous, created_at, updated_at) VALUES (?, ?, ?, ?, 0, ?, ?)"
    )
    .bind(&post_id)
    .bind(&current_user.id)
    .bind(&body.content)
    .bind(&visibility)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => {
            let post = Post {
                id: post_id.clone(),
                user_id: current_user.id.clone(),
                content: body.content.clone(),
                visibility,
                group_id: None,
                is_anonymous: false,
                created_at: now.clone(),
                updated_at: now,
            };
            
            let post_response = build_post_response(&state, &post, &current_user.id).await;
            HttpResponse::Created().json(post_response)
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to create post: {}", e)
        })),
    }
}

pub async fn get_post(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    let post_id = path.into_inner();

    let post = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = ?")
        .bind(&post_id)
        .fetch_optional(&state.db)
        .await;

    match post {
        Ok(Some(post)) => {
            // Check visibility permissions
            if !can_view_post(&state, &post, &current_user.id).await {
                return HttpResponse::Forbidden().json(serde_json::json!({
                    "error": "You don't have permission to view this post"
                }));
            }
            
            let post_response = build_post_response(&state, &post, &current_user.id).await;
            HttpResponse::Ok().json(post_response)
        }
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Post not found"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Database error: {}", e)
        })),
    }
}

pub async fn delete_post(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    let post_id = path.into_inner();

    let post = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = ?")
        .bind(&post_id)
        .fetch_optional(&state.db)
        .await;

    match post {
        Ok(Some(post)) => {
            if post.user_id != current_user.id {
                return HttpResponse::Forbidden().json(serde_json::json!({
                    "error": "You can only delete your own posts"
                }));
            }

            // Delete related comments and likes first
            let _ = sqlx::query("DELETE FROM comments WHERE post_id = ?")
                .bind(&post_id)
                .execute(&state.db)
                .await;
            let _ = sqlx::query("DELETE FROM likes WHERE post_id = ?")
                .bind(&post_id)
                .execute(&state.db)
                .await;

            let result = sqlx::query("DELETE FROM posts WHERE id = ?")
                .bind(&post_id)
                .execute(&state.db)
                .await;

            match result {
                Ok(_) => HttpResponse::Ok().json(serde_json::json!({
                    "message": "Post deleted"
                })),
                Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": format!("Failed to delete post: {}", e)
                })),
            }
        }
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Post not found"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Database error: {}", e)
        })),
    }
}

pub async fn like_post(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    let post_id = path.into_inner();

    // Check if already liked
    let existing_like = sqlx::query_as::<_, Like>(
        "SELECT * FROM likes WHERE post_id = ? AND user_id = ?"
    )
    .bind(&post_id)
    .bind(&current_user.id)
    .fetch_optional(&state.db)
    .await;

    match existing_like {
        Ok(Some(_)) => {
            // Unlike
            let _ = sqlx::query("DELETE FROM likes WHERE post_id = ? AND user_id = ?")
                .bind(&post_id)
                .bind(&current_user.id)
                .execute(&state.db)
                .await;

            HttpResponse::Ok().json(serde_json::json!({
                "message": "Post unliked",
                "liked": false
            }))
        }
        Ok(None) => {
            // Like
            let like_id = Uuid::new_v4().to_string();
            let now = Utc::now().to_rfc3339();

            let result = sqlx::query(
                "INSERT INTO likes (id, post_id, user_id, created_at) VALUES (?, ?, ?, ?)"
            )
            .bind(&like_id)
            .bind(&post_id)
            .bind(&current_user.id)
            .bind(&now)
            .execute(&state.db)
            .await;

            match result {
                Ok(_) => HttpResponse::Ok().json(serde_json::json!({
                    "message": "Post liked",
                    "liked": true
                })),
                Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": format!("Failed to like post: {}", e)
                })),
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Database error: {}", e)
        })),
    }
}

pub async fn add_comment(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<CreateCommentRequest>,
) -> HttpResponse {
    let current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    let post_id = path.into_inner();

    if body.content.trim().is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Comment content cannot be empty"
        }));
    }

    // Check if post exists
    let post = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = ?")
        .bind(&post_id)
        .fetch_optional(&state.db)
        .await;

    if let Ok(None) = post {
        return HttpResponse::NotFound().json(serde_json::json!({
            "error": "Post not found"
        }));
    }

    let comment_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let is_anonymous = body.is_anonymous.unwrap_or(false);

    let result = sqlx::query(
        "INSERT INTO comments (id, post_id, user_id, content, is_anonymous, created_at) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&comment_id)
    .bind(&post_id)
    .bind(&current_user.id)
    .bind(&body.content)
    .bind(is_anonymous)
    .bind(&now)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => {
            let comment_response = CommentResponse {
                id: comment_id,
                user: if is_anonymous { None } else { Some(current_user.into()) },
                content: body.content.clone(),
                is_anonymous,
                created_at: now,
            };
            HttpResponse::Created().json(comment_response)
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to add comment: {}", e)
        })),
    }
}

pub async fn get_comments(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let _current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    let post_id = path.into_inner();

    let comments = sqlx::query_as::<_, Comment>(
        "SELECT * FROM comments WHERE post_id = ? ORDER BY created_at ASC"
    )
    .bind(&post_id)
    .fetch_all(&state.db)
    .await;

    match comments {
        Ok(comments) => {
            let mut comment_responses: Vec<CommentResponse> = Vec::new();
            
            for comment in comments {
                let user = if comment.is_anonymous {
                    None
                } else {
                    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
                        .bind(&comment.user_id)
                        .fetch_optional(&state.db)
                        .await
                        .ok()
                        .flatten()
                        .map(|u| u.into())
                };

                comment_responses.push(CommentResponse {
                    id: comment.id,
                    user,
                    content: comment.content,
                    is_anonymous: comment.is_anonymous,
                    created_at: comment.created_at,
                });
            }
            
            HttpResponse::Ok().json(comment_responses)
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to get comments: {}", e)
        })),
    }
}

// Helper functions
async fn build_post_response(state: &web::Data<AppState>, post: &Post, current_user_id: &str) -> PostResponse {
    let user = if post.is_anonymous {
        None
    } else {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
            .bind(&post.user_id)
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten()
            .map(|u| u.into())
    };

    let likes_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM likes WHERE post_id = ?")
        .bind(&post.id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let comments_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM comments WHERE post_id = ?")
        .bind(&post.id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let is_liked = sqlx::query_as::<_, Like>(
        "SELECT * FROM likes WHERE post_id = ? AND user_id = ?"
    )
    .bind(&post.id)
    .bind(current_user_id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .is_some();

    PostResponse {
        id: post.id.clone(),
        user,
        content: post.content.clone(),
        visibility: post.visibility.clone(),
        is_anonymous: post.is_anonymous,
        likes_count,
        comments_count,
        is_liked,
        created_at: post.created_at.clone(),
    }
}

async fn can_view_post(state: &web::Data<AppState>, post: &Post, viewer_id: &str) -> bool {
    // Owner can always view
    if post.user_id == viewer_id {
        return true;
    }

    match post.visibility.as_str() {
        "public" => true,
        "private" => false,
        "friends_only" => {
            // Check if viewer is a friend
            let friendship = sqlx::query_as::<_, Friendship>(
                "SELECT * FROM friendships WHERE ((user_id = ? AND friend_id = ?) OR (user_id = ? AND friend_id = ?)) AND status = 'accepted'"
            )
            .bind(&post.user_id)
            .bind(viewer_id)
            .bind(viewer_id)
            .bind(&post.user_id)
            .fetch_optional(&state.db)
            .await;

            matches!(friendship, Ok(Some(_)))
        }
        _ => false,
    }
}
