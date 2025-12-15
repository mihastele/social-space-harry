/**
 * API Client Module
 * Handles all HTTP requests to the backend
 */

const API_BASE_URL = '/api';
const WS_URL = `${window.location.protocol === 'https:' ? 'wss' : 'ws'}://${window.location.host}/ws/chat`;

class ApiClient {
    constructor() {
        this.token = localStorage.getItem('auth_token');
    }

    setToken(token) {
        this.token = token;
        if (token) {
            localStorage.setItem('auth_token', token);
        } else {
            localStorage.removeItem('auth_token');
        }
    }

    getHeaders() {
        const headers = {
            'Content-Type': 'application/json'
        };
        if (this.token) {
            headers['Authorization'] = `Bearer ${this.token}`;
        }
        return headers;
    }

    async request(endpoint, options = {}) {
        const url = `${API_BASE_URL}${endpoint}`;
        const config = {
            ...options,
            headers: {
                ...this.getHeaders(),
                ...options.headers
            }
        };

        try {
            const response = await fetch(url, config);
            const data = await response.json();
            
            if (!response.ok) {
                throw new Error(data.error || 'Request failed');
            }
            
            return data;
        } catch (error) {
            console.error(`API Error [${endpoint}]:`, error);
            throw error;
        }
    }

    // Auth endpoints
    async register(email, password, username, displayName) {
        const data = await this.request('/auth/register', {
            method: 'POST',
            body: JSON.stringify({
                email,
                password,
                username,
                display_name: displayName
            })
        });
        this.setToken(data.token);
        return data;
    }

    async login(email, password) {
        const data = await this.request('/auth/login', {
            method: 'POST',
            body: JSON.stringify({ email, password })
        });
        this.setToken(data.token);
        return data;
    }

    async getMe() {
        return await this.request('/auth/me');
    }

    logout() {
        this.setToken(null);
        localStorage.removeItem('e2e_private_key');
        localStorage.removeItem('e2e_public_key');
    }

    // User endpoints
    async searchUsers(query = '') {
        return await this.request(`/users?q=${encodeURIComponent(query)}`);
    }

    async getUser(userId) {
        return await this.request(`/users/${userId}`);
    }

    // Friend endpoints
    async getFriends() {
        return await this.request('/friends');
    }

    async getFriendRequests() {
        return await this.request('/friends/requests');
    }

    async sendFriendRequest(userId) {
        return await this.request(`/friends/request/${userId}`, {
            method: 'POST'
        });
    }

    async acceptFriendRequest(userId) {
        return await this.request(`/friends/accept/${userId}`, {
            method: 'POST'
        });
    }

    async rejectFriendRequest(userId) {
        return await this.request(`/friends/reject/${userId}`, {
            method: 'POST'
        });
    }

    // Post endpoints
    async getFeed() {
        return await this.request('/posts');
    }

    async createPost(content, visibility = 'friends_only') {
        return await this.request('/posts', {
            method: 'POST',
            body: JSON.stringify({ content, visibility })
        });
    }

    async getPost(postId) {
        return await this.request(`/posts/${postId}`);
    }

    async deletePost(postId) {
        return await this.request(`/posts/${postId}`, {
            method: 'DELETE'
        });
    }

    async likePost(postId) {
        return await this.request(`/posts/${postId}/like`, {
            method: 'POST'
        });
    }

    async addComment(postId, content, isAnonymous = false) {
        return await this.request(`/posts/${postId}/comment`, {
            method: 'POST',
            body: JSON.stringify({ content, is_anonymous: isAnonymous })
        });
    }

    async getComments(postId) {
        return await this.request(`/posts/${postId}/comments`);
    }

    // Group endpoints
    async getGroups() {
        return await this.request('/groups');
    }

    async createGroup(name, description, isPrivate = false) {
        return await this.request('/groups', {
            method: 'POST',
            body: JSON.stringify({ name, description, is_private: isPrivate })
        });
    }

    async getGroup(groupId) {
        return await this.request(`/groups/${groupId}`);
    }

    async joinGroup(groupId) {
        return await this.request(`/groups/${groupId}/join`, {
            method: 'POST'
        });
    }

    async leaveGroup(groupId) {
        return await this.request(`/groups/${groupId}/leave`, {
            method: 'POST'
        });
    }

    async getGroupPosts(groupId) {
        return await this.request(`/groups/${groupId}/posts`);
    }

    async createGroupPost(groupId, content, isAnonymous = false) {
        return await this.request(`/groups/${groupId}/posts`, {
            method: 'POST',
            body: JSON.stringify({ content, is_anonymous: isAnonymous })
        });
    }

    // Chat endpoints
    async getConversations() {
        return await this.request('/chat/conversations');
    }

    async getMessages(userId) {
        return await this.request(`/chat/messages/${userId}`);
    }

    async getPublicKey(userId) {
        return await this.request(`/chat/keys/${userId}`);
    }

    async storePublicKey(publicKey) {
        return await this.request('/chat/keys', {
            method: 'POST',
            body: JSON.stringify({ public_key: publicKey })
        });
    }
}

/**
 * WebSocket Client for real-time chat
 */
class ChatWebSocket {
    constructor() {
        this.ws = null;
        this.isConnected = false;
        this.reconnectAttempts = 0;
        this.maxReconnectAttempts = 5;
        this.messageHandlers = [];
        this.typingHandlers = [];
    }

    connect(token) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            return;
        }

        this.ws = new WebSocket(WS_URL);
        
        this.ws.onopen = () => {
            console.log('WebSocket connected');
            this.reconnectAttempts = 0;
            // Authenticate
            this.send({
                type: 'auth',
                token: token
            });
        };

        this.ws.onmessage = (event) => {
            try {
                const data = JSON.parse(event.data);
                this.handleMessage(data);
            } catch (error) {
                console.error('Failed to parse WebSocket message:', error);
            }
        };

        this.ws.onclose = () => {
            console.log('WebSocket disconnected');
            this.isConnected = false;
            this.attemptReconnect(token);
        };

        this.ws.onerror = (error) => {
            console.error('WebSocket error:', error);
        };
    }

    attemptReconnect(token) {
        if (this.reconnectAttempts < this.maxReconnectAttempts) {
            this.reconnectAttempts++;
            console.log(`Attempting to reconnect (${this.reconnectAttempts}/${this.maxReconnectAttempts})...`);
            setTimeout(() => this.connect(token), 2000 * this.reconnectAttempts);
        }
    }

    handleMessage(data) {
        switch (data.type) {
            case 'connected':
                this.isConnected = true;
                console.log('WebSocket authenticated as:', data.user_id);
                break;
            case 'message_received':
                this.messageHandlers.forEach(handler => handler(data.message));
                break;
            case 'typing_indicator':
                this.typingHandlers.forEach(handler => handler(data.sender_id));
                break;
            case 'error':
                console.error('WebSocket error:', data.message);
                break;
        }
    }

    send(data) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(data));
        }
    }

    sendMessage(receiverId, encryptedContent, iv) {
        this.send({
            type: 'message',
            receiver_id: receiverId,
            encrypted_content: encryptedContent,
            iv: iv
        });
    }

    sendTyping(receiverId) {
        this.send({
            type: 'typing',
            receiver_id: receiverId
        });
    }

    onMessage(handler) {
        this.messageHandlers.push(handler);
    }

    onTyping(handler) {
        this.typingHandlers.push(handler);
    }

    disconnect() {
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
    }
}

// Export singleton instances
window.api = new ApiClient();
window.chatWs = new ChatWebSocket();
