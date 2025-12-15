# Social Space

A modern social media platform built with **Rust** backend and **HTML/CSS/JavaScript** frontend.

## Features

### 🔐 Authentication
- User registration and login
- JWT-based authentication
- Secure password hashing with bcrypt

### 👥 Social Features
- **Friends System**: Send, accept, and reject friend requests
- **Posts**: Create posts with visibility settings:
  - **Public**: Visible to everyone
  - **Friends Only**: Visible to friends (default)
  - **Private**: Visible only to you
- **Likes & Comments**: Engage with posts
- **Anonymous Posting**: Option to post anonymously in groups

### 👥 Groups
- Create and join groups
- Group posts visible only to members
- Anonymous posting and commenting in groups
- Private/public group settings

### 💬 End-to-End Encrypted Chat
- Real-time messaging via WebSocket
- **E2E Encryption** using ECDH key exchange + AES-GCM
- Typing indicators
- Message read receipts

## Tech Stack

### Backend
- **Rust** with Actix-web framework
- **SQLite** database with SQLx
- **WebSocket** for real-time chat
- **JWT** authentication
- **bcrypt** password hashing

### Frontend
- **HTML5** / **CSS3** / **JavaScript**
- Modern dark theme UI
- Web Crypto API for E2E encryption
- Responsive design

## Getting Started

### Prerequisites
- [Rust](https://rustup.rs/) (1.70+)
- A modern web browser

### Running the Backend

```bash
cd backend
cargo run
```

The server will start at `http://localhost:8080`

### Running the Frontend

You can serve the frontend using any static file server. For example:

**Using Python:**
```bash
cd frontend
python -m http.server 3000
```

**Using Node.js (with serve):**
```bash
npx serve frontend -p 3000
```

Then open `http://localhost:3000` in your browser.

## API Endpoints

### Authentication
- `POST /api/auth/register` - Register new user
- `POST /api/auth/login` - Login user
- `GET /api/auth/me` - Get current user

### Users
- `GET /api/users?q=query` - Search users
- `GET /api/users/:id` - Get user by ID

### Friends
- `GET /api/friends` - Get friends list
- `GET /api/friends/requests` - Get pending friend requests
- `POST /api/friends/request/:user_id` - Send friend request
- `POST /api/friends/accept/:user_id` - Accept friend request
- `POST /api/friends/reject/:user_id` - Reject friend request

### Posts
- `GET /api/posts` - Get feed
- `POST /api/posts` - Create post
- `GET /api/posts/:id` - Get post
- `DELETE /api/posts/:id` - Delete post
- `POST /api/posts/:id/like` - Like/unlike post
- `POST /api/posts/:id/comment` - Add comment
- `GET /api/posts/:id/comments` - Get comments

### Groups
- `GET /api/groups` - Get user's groups
- `POST /api/groups` - Create group
- `GET /api/groups/:id` - Get group
- `POST /api/groups/:id/join` - Join group
- `POST /api/groups/:id/leave` - Leave group
- `GET /api/groups/:id/posts` - Get group posts
- `POST /api/groups/:id/posts` - Create group post

### Chat
- `GET /api/chat/conversations` - Get conversations
- `GET /api/chat/messages/:user_id` - Get messages with user
- `GET /api/chat/keys/:user_id` - Get user's public key
- `POST /api/chat/keys` - Store public key
- `WS /ws/chat` - WebSocket for real-time chat

## E2E Encryption

The chat feature uses end-to-end encryption:

1. **Key Generation**: Each user generates an ECDH P-256 key pair
2. **Key Exchange**: Public keys are stored on the server
3. **Message Encryption**: Messages are encrypted using AES-GCM with a shared secret derived from ECDH
4. **Server Cannot Read**: The server only sees encrypted content

## Project Structure

```
social-space/
├── backend/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs          # Server entry point
│       ├── models.rs        # Data models
│       ├── db.rs            # Database initialization
│       ├── auth.rs          # Authentication utilities
│       ├── websocket.rs     # WebSocket handler
│       └── handlers/        # API handlers
│           ├── mod.rs
│           ├── auth.rs
│           ├── users.rs
│           ├── friends.rs
│           ├── posts.rs
│           ├── groups.rs
│           └── chat.rs
└── frontend/
    ├── index.html           # Main HTML
    ├── css/
    │   └── styles.css       # Styles
    └── js/
        ├── app.js           # Main application
        ├── api.js           # API client
        └── crypto.js        # E2E encryption
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `sqlite:social_space.db?mode=rwc` | SQLite database path |
| `JWT_SECRET` | `super_secret_key_change_in_production` | JWT signing secret |

## Security Notes

- Change `JWT_SECRET` in production
- Use HTTPS in production
- The E2E encryption keys are stored in localStorage - consider more secure storage for production
- Implement rate limiting for production use

## License

MIT License
# social-space-harry
