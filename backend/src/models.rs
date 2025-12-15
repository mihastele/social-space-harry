use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// User models
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub username: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        UserResponse {
            id: user.id,
            email: user.email,
            username: user.username,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
            bio: user.bio,
        }
    }
}

// Friend models
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Friendship {
    pub id: String,
    pub user_id: String,
    pub friend_id: String,
    pub status: String, // pending, accepted, rejected
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct FriendWithUser {
    pub friendship_id: String,
    pub user: UserResponse,
    pub status: String,
    pub created_at: String,
}

// Post models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PostVisibility {
    Public,
    FriendsOnly,
    Private,
}

impl From<String> for PostVisibility {
    fn from(s: String) -> Self {
        match s.as_str() {
            "public" => PostVisibility::Public,
            "private" => PostVisibility::Private,
            _ => PostVisibility::FriendsOnly,
        }
    }
}

impl ToString for PostVisibility {
    fn to_string(&self) -> String {
        match self {
            PostVisibility::Public => "public".to_string(),
            PostVisibility::FriendsOnly => "friends_only".to_string(),
            PostVisibility::Private => "private".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Post {
    pub id: String,
    pub user_id: String,
    pub content: String,
    pub visibility: String,
    pub group_id: Option<String>,
    pub is_anonymous: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreatePostRequest {
    pub content: String,
    pub visibility: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PostResponse {
    pub id: String,
    pub user: Option<UserResponse>,
    pub content: String,
    pub visibility: String,
    pub is_anonymous: bool,
    pub likes_count: i32,
    pub comments_count: i32,
    pub is_liked: bool,
    pub created_at: String,
}

// Comment models
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Comment {
    pub id: String,
    pub post_id: String,
    pub user_id: String,
    pub content: String,
    pub is_anonymous: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
    pub is_anonymous: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CommentResponse {
    pub id: String,
    pub user: Option<UserResponse>,
    pub content: String,
    pub is_anonymous: bool,
    pub created_at: String,
}

// Like model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Like {
    pub id: String,
    pub post_id: String,
    pub user_id: String,
    pub created_at: String,
}

// Group models
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub cover_image: Option<String>,
    pub creator_id: String,
    pub is_private: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    pub description: Option<String>,
    pub is_private: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct GroupResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub cover_image: Option<String>,
    pub creator: UserResponse,
    pub is_private: bool,
    pub members_count: i32,
    pub is_member: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GroupMember {
    pub id: String,
    pub group_id: String,
    pub user_id: String,
    pub role: String, // admin, moderator, member
    pub joined_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateGroupPostRequest {
    pub content: String,
    pub is_anonymous: Option<bool>,
}

// Chat models
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Message {
    pub id: String,
    pub sender_id: String,
    pub receiver_id: String,
    pub encrypted_content: String,
    pub iv: String,  // Initialization vector for E2E encryption
    pub created_at: String,
    pub is_read: bool,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub receiver_id: String,
    pub encrypted_content: String,
    pub iv: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    pub id: String,
    pub sender_id: String,
    pub receiver_id: String,
    pub encrypted_content: String,
    pub iv: String,
    pub created_at: String,
    pub is_read: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserPublicKey {
    pub user_id: String,
    pub public_key: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct StorePublicKeyRequest {
    pub public_key: String,
}

#[derive(Debug, Serialize)]
pub struct ConversationResponse {
    pub user: UserResponse,
    pub last_message: Option<MessageResponse>,
    pub unread_count: i32,
}

// WebSocket messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    #[serde(rename = "auth")]
    Auth { token: String },
    #[serde(rename = "message")]
    ChatMessage {
        receiver_id: String,
        encrypted_content: String,
        iv: String,
    },
    #[serde(rename = "message_received")]
    MessageReceived {
        message: MessageResponse,
    },
    #[serde(rename = "typing")]
    Typing { receiver_id: String },
    #[serde(rename = "typing_indicator")]
    TypingIndicator { sender_id: String },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "connected")]
    Connected { user_id: String },
}
