use actix_web::{web, HttpRequest, HttpResponse};
use chrono::Utc;
use uuid::Uuid;

use crate::auth::require_auth;
use crate::models::{CreateGroupPostRequest, CreateGroupRequest, Group, GroupMember, GroupResponse, Post, PostResponse, User, UserResponse};
use crate::AppState;

pub async fn get_groups(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> HttpResponse {
    let current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    // Get groups the user is a member of
    let groups = sqlx::query_as::<_, Group>(
        r#"
        SELECT g.* FROM groups g
        INNER JOIN group_members gm ON g.id = gm.group_id
        WHERE gm.user_id = ?
        ORDER BY g.created_at DESC
        "#
    )
    .bind(&current_user.id)
    .fetch_all(&state.db)
    .await;

    match groups {
        Ok(groups) => {
            let mut group_responses: Vec<GroupResponse> = Vec::new();
            
            for group in groups {
                let group_response = build_group_response(&state, &group, &current_user.id).await;
                group_responses.push(group_response);
            }
            
            HttpResponse::Ok().json(group_responses)
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to get groups: {}", e)
        })),
    }
}

pub async fn create_group(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<CreateGroupRequest>,
) -> HttpResponse {
    let current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    if body.name.trim().is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Group name cannot be empty"
        }));
    }

    let group_id = Uuid::new_v4().to_string();
    let member_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let is_private = body.is_private.unwrap_or(false);

    // Create group
    let result = sqlx::query(
        "INSERT INTO groups (id, name, description, creator_id, is_private, created_at) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&group_id)
    .bind(&body.name)
    .bind(&body.description)
    .bind(&current_user.id)
    .bind(is_private)
    .bind(&now)
    .execute(&state.db)
    .await;

    if let Err(e) = result {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to create group: {}", e)
        }));
    }

    // Add creator as admin member
    let member_result = sqlx::query(
        "INSERT INTO group_members (id, group_id, user_id, role, joined_at) VALUES (?, ?, ?, 'admin', ?)"
    )
    .bind(&member_id)
    .bind(&group_id)
    .bind(&current_user.id)
    .bind(&now)
    .execute(&state.db)
    .await;

    match member_result {
        Ok(_) => {
            let group = Group {
                id: group_id,
                name: body.name.clone(),
                description: body.description.clone(),
                cover_image: None,
                creator_id: current_user.id.clone(),
                is_private,
                created_at: now,
            };
            
            let group_response = build_group_response(&state, &group, &current_user.id).await;
            HttpResponse::Created().json(group_response)
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to add creator as member: {}", e)
        })),
    }
}

pub async fn get_group(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    let group_id = path.into_inner();

    let group = sqlx::query_as::<_, Group>("SELECT * FROM groups WHERE id = ?")
        .bind(&group_id)
        .fetch_optional(&state.db)
        .await;

    match group {
        Ok(Some(group)) => {
            let group_response = build_group_response(&state, &group, &current_user.id).await;
            HttpResponse::Ok().json(group_response)
        }
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Group not found"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Database error: {}", e)
        })),
    }
}

pub async fn join_group(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    let group_id = path.into_inner();

    // Check if group exists
    let group = sqlx::query_as::<_, Group>("SELECT * FROM groups WHERE id = ?")
        .bind(&group_id)
        .fetch_optional(&state.db)
        .await;

    if let Ok(None) = group {
        return HttpResponse::NotFound().json(serde_json::json!({
            "error": "Group not found"
        }));
    }

    // Check if already a member
    let existing = sqlx::query_as::<_, GroupMember>(
        "SELECT * FROM group_members WHERE group_id = ? AND user_id = ?"
    )
    .bind(&group_id)
    .bind(&current_user.id)
    .fetch_optional(&state.db)
    .await;

    if let Ok(Some(_)) = existing {
        return HttpResponse::Conflict().json(serde_json::json!({
            "error": "Already a member of this group"
        }));
    }

    let member_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let result = sqlx::query(
        "INSERT INTO group_members (id, group_id, user_id, role, joined_at) VALUES (?, ?, ?, 'member', ?)"
    )
    .bind(&member_id)
    .bind(&group_id)
    .bind(&current_user.id)
    .bind(&now)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Joined group successfully"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to join group: {}", e)
        })),
    }
}

pub async fn leave_group(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    let group_id = path.into_inner();

    let result = sqlx::query(
        "DELETE FROM group_members WHERE group_id = ? AND user_id = ?"
    )
    .bind(&group_id)
    .bind(&current_user.id)
    .execute(&state.db)
    .await;

    match result {
        Ok(r) => {
            if r.rows_affected() > 0 {
                HttpResponse::Ok().json(serde_json::json!({
                    "message": "Left group successfully"
                }))
            } else {
                HttpResponse::NotFound().json(serde_json::json!({
                    "error": "Not a member of this group"
                }))
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to leave group: {}", e)
        })),
    }
}

pub async fn get_group_posts(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    let group_id = path.into_inner();

    // Check if user is a member
    let is_member = sqlx::query_as::<_, GroupMember>(
        "SELECT * FROM group_members WHERE group_id = ? AND user_id = ?"
    )
    .bind(&group_id)
    .bind(&current_user.id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .is_some();

    if !is_member {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "You must be a member to view group posts"
        }));
    }

    let posts = sqlx::query_as::<_, Post>(
        "SELECT * FROM posts WHERE group_id = ? ORDER BY created_at DESC LIMIT 50"
    )
    .bind(&group_id)
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
            "error": format!("Failed to get group posts: {}", e)
        })),
    }
}

pub async fn create_group_post(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<CreateGroupPostRequest>,
) -> HttpResponse {
    let current_user = match require_auth(&req, &state).await {
        Ok(user) => user,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": e.to_string()})),
    };

    let group_id = path.into_inner();

    if body.content.trim().is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Post content cannot be empty"
        }));
    }

    // Check if user is a member
    let is_member = sqlx::query_as::<_, GroupMember>(
        "SELECT * FROM group_members WHERE group_id = ? AND user_id = ?"
    )
    .bind(&group_id)
    .bind(&current_user.id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .is_some();

    if !is_member {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "You must be a member to post in this group"
        }));
    }

    let post_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let is_anonymous = body.is_anonymous.unwrap_or(false);

    let result = sqlx::query(
        "INSERT INTO posts (id, user_id, content, visibility, group_id, is_anonymous, created_at, updated_at) VALUES (?, ?, ?, 'group', ?, ?, ?, ?)"
    )
    .bind(&post_id)
    .bind(&current_user.id)
    .bind(&body.content)
    .bind(&group_id)
    .bind(is_anonymous)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => {
            let post = Post {
                id: post_id,
                user_id: current_user.id.clone(),
                content: body.content.clone(),
                visibility: "group".to_string(),
                group_id: Some(group_id),
                is_anonymous,
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

// Helper functions
async fn build_group_response(state: &web::Data<AppState>, group: &Group, current_user_id: &str) -> GroupResponse {
    let creator = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(&group.creator_id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten()
        .map(|u| UserResponse::from(u))
        .unwrap_or_else(|| UserResponse {
            id: group.creator_id.clone(),
            email: String::new(),
            username: "Unknown".to_string(),
            display_name: "Unknown User".to_string(),
            avatar_url: None,
            bio: None,
        });

    let members_count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM group_members WHERE group_id = ?")
        .bind(&group.id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let is_member = sqlx::query_as::<_, GroupMember>(
        "SELECT * FROM group_members WHERE group_id = ? AND user_id = ?"
    )
    .bind(&group.id)
    .bind(current_user_id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .is_some();

    GroupResponse {
        id: group.id.clone(),
        name: group.name.clone(),
        description: group.description.clone(),
        cover_image: group.cover_image.clone(),
        creator,
        is_private: group.is_private,
        members_count,
        is_member,
        created_at: group.created_at.clone(),
    }
}

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

    let is_liked = sqlx::query_scalar::<_, i32>(
        "SELECT COUNT(*) FROM likes WHERE post_id = ? AND user_id = ?"
    )
    .bind(&post.id)
    .bind(current_user_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0) > 0;

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
