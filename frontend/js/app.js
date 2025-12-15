/**
 * Social Space - Main Application
 */

class SocialSpaceApp {
    constructor() {
        this.currentUser = null;
        this.currentPage = 'feed';
        this.currentChatUser = null;
        this.currentGroup = null;
        this.currentPostId = null;
        
        this.init();
    }

    async init() {
        this.bindEvents();
        await this.checkAuth();
    }

    bindEvents() {
        // Auth events
        document.getElementById('login-form').addEventListener('submit', (e) => this.handleLogin(e));
        document.getElementById('register-form').addEventListener('submit', (e) => this.handleRegister(e));
        document.getElementById('show-register').addEventListener('click', (e) => {
            e.preventDefault();
            document.getElementById('login-form').classList.add('hidden');
            document.getElementById('register-form').classList.remove('hidden');
        });
        document.getElementById('show-login').addEventListener('click', (e) => {
            e.preventDefault();
            document.getElementById('register-form').classList.add('hidden');
            document.getElementById('login-form').classList.remove('hidden');
        });

        // User menu
        document.getElementById('user-menu').addEventListener('click', () => {
            document.getElementById('user-dropdown').classList.toggle('hidden');
        });
        document.getElementById('logout-btn').addEventListener('click', () => this.logout());

        // Navigation
        document.querySelectorAll('.nav-btn').forEach(btn => {
            btn.addEventListener('click', () => this.navigateTo(btn.dataset.page));
        });

        // Search
        document.getElementById('search-input').addEventListener('input', 
            this.debounce((e) => this.handleSearch(e.target.value), 300)
        );
        document.addEventListener('click', (e) => {
            if (!e.target.closest('.search-box') && !e.target.closest('.search-results')) {
                document.getElementById('search-results').classList.add('hidden');
            }
        });

        // Post modal
        document.getElementById('open-post-modal').addEventListener('click', () => this.openModal('post-modal'));
        document.getElementById('close-post-modal').addEventListener('click', () => this.closeModal('post-modal'));
        document.getElementById('submit-post').addEventListener('click', () => this.createPost());

        // Comment modal
        document.getElementById('close-comment-modal').addEventListener('click', () => this.closeModal('comment-modal'));
        document.getElementById('submit-comment').addEventListener('click', () => this.submitComment());

        // Friends tab
        document.querySelectorAll('.friends-tabs .tab-btn').forEach(btn => {
            btn.addEventListener('click', () => this.switchFriendsTab(btn.dataset.tab));
        });
        document.getElementById('find-friends-search').addEventListener('input',
            this.debounce((e) => this.searchFriends(e.target.value), 300)
        );

        // Groups
        document.getElementById('create-group-btn').addEventListener('click', () => this.openModal('create-group-modal'));
        document.getElementById('close-create-group-modal').addEventListener('click', () => this.closeModal('create-group-modal'));
        document.getElementById('submit-create-group').addEventListener('click', () => this.createGroup());
        document.getElementById('back-to-groups').addEventListener('click', () => this.closeGroupDetail());

        // Group post modal
        document.getElementById('open-group-post-modal').addEventListener('click', () => this.openModal('group-post-modal'));
        document.getElementById('close-group-post-modal').addEventListener('click', () => this.closeModal('group-post-modal'));
        document.getElementById('submit-group-post').addEventListener('click', () => this.createGroupPost());

        // Chat
        document.getElementById('message-input').addEventListener('keypress', (e) => {
            if (e.key === 'Enter') this.sendMessage();
        });
        document.getElementById('send-message-btn').addEventListener('click', () => this.sendMessage());
        document.getElementById('message-input').addEventListener('input', () => this.handleTyping());

        // Modal close on outside click
        document.querySelectorAll('.modal').forEach(modal => {
            modal.addEventListener('click', (e) => {
                if (e.target === modal) this.closeModal(modal.id);
            });
        });
    }

    // Auth methods
    async checkAuth() {
        const token = localStorage.getItem('auth_token');
        if (!token) {
            this.showAuth();
            return;
        }

        try {
            this.currentUser = await api.getMe();
            await this.initializeApp();
        } catch (error) {
            this.showAuth();
        }
    }

    async handleLogin(e) {
        e.preventDefault();
        const email = document.getElementById('login-email').value;
        const password = document.getElementById('login-password').value;

        try {
            const response = await api.login(email, password);
            this.currentUser = response.user;
            await this.initializeApp();
            this.showToast('Welcome back!', 'success');
        } catch (error) {
            this.showToast(error.message, 'error');
        }
    }

    async handleRegister(e) {
        e.preventDefault();
        const email = document.getElementById('register-email').value;
        const username = document.getElementById('register-username').value;
        const displayName = document.getElementById('register-displayname').value;
        const password = document.getElementById('register-password').value;

        try {
            const response = await api.register(email, password, username, displayName);
            this.currentUser = response.user;
            await this.initializeApp();
            this.showToast('Account created successfully!', 'success');
        } catch (error) {
            this.showToast(error.message, 'error');
        }
    }

    logout() {
        api.logout();
        chatWs.disconnect();
        this.currentUser = null;
        this.showAuth();
        this.showToast('Logged out successfully', 'success');
    }

    showAuth() {
        document.getElementById('auth-container').classList.remove('hidden');
        document.getElementById('app-container').classList.add('hidden');
    }

    async initializeApp() {
        document.getElementById('auth-container').classList.add('hidden');
        document.getElementById('app-container').classList.remove('hidden');

        // Update user info in UI
        this.updateUserUI();

        // Initialize E2E encryption
        try {
            const publicKey = await e2eCrypto.initialize();
            await api.storePublicKey(publicKey);
        } catch (error) {
            console.error('Failed to initialize E2E encryption:', error);
        }

        // Connect WebSocket
        chatWs.connect(api.token);
        chatWs.onMessage((msg) => this.handleIncomingMessage(msg));
        chatWs.onTyping((senderId) => this.handleTypingIndicator(senderId));

        // Load initial data
        this.navigateTo('feed');
        this.loadFriendRequests();
    }

    updateUserUI() {
        const initial = this.currentUser.display_name.charAt(0).toUpperCase();
        document.getElementById('nav-avatar-initial').textContent = initial;
        document.getElementById('create-avatar-initial').textContent = initial;
        document.getElementById('group-avatar-initial').textContent = initial;
        document.getElementById('dropdown-name').textContent = this.currentUser.display_name;
        document.getElementById('dropdown-email').textContent = this.currentUser.email;
    }

    // Navigation
    navigateTo(page) {
        this.currentPage = page;
        
        // Update nav buttons
        document.querySelectorAll('.nav-btn').forEach(btn => {
            btn.classList.toggle('active', btn.dataset.page === page);
        });

        // Hide all pages
        document.querySelectorAll('.page').forEach(p => p.classList.add('hidden'));
        
        // Show selected page
        document.getElementById(`${page}-page`).classList.remove('hidden');

        // Load page data
        switch (page) {
            case 'feed':
                this.loadFeed();
                break;
            case 'friends':
                this.loadFriends();
                break;
            case 'groups':
                this.loadGroups();
                break;
            case 'chat':
                this.loadConversations();
                break;
        }
    }

    // Search
    async handleSearch(query) {
        const resultsContainer = document.getElementById('search-results');
        
        if (!query.trim()) {
            resultsContainer.classList.add('hidden');
            return;
        }

        try {
            const users = await api.searchUsers(query);
            resultsContainer.innerHTML = users.map(user => `
                <div class="search-result-item" data-user-id="${user.id}">
                    <div class="avatar">
                        <span>${user.display_name.charAt(0).toUpperCase()}</span>
                    </div>
                    <span>${user.display_name} (@${user.username})</span>
                </div>
            `).join('') || '<div class="search-result-item">No users found</div>';
            
            resultsContainer.classList.remove('hidden');

            // Add click handlers
            resultsContainer.querySelectorAll('.search-result-item[data-user-id]').forEach(item => {
                item.addEventListener('click', () => {
                    this.startChat(item.dataset.userId);
                    resultsContainer.classList.add('hidden');
                    document.getElementById('search-input').value = '';
                });
            });
        } catch (error) {
            console.error('Search failed:', error);
        }
    }

    // Feed
    async loadFeed() {
        const feedContainer = document.getElementById('posts-feed');
        feedContainer.innerHTML = '<div class="loading-spinner"><div class="spinner"></div></div>';

        try {
            const posts = await api.getFeed();
            
            if (posts.length === 0) {
                feedContainer.innerHTML = `
                    <div class="empty-state">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>
                        </svg>
                        <p>No posts yet. Be the first to share something!</p>
                    </div>
                `;
                return;
            }

            feedContainer.innerHTML = posts.map(post => this.renderPost(post)).join('');
            this.bindPostEvents();
        } catch (error) {
            feedContainer.innerHTML = `<div class="empty-state"><p>Failed to load feed</p></div>`;
        }
    }

    renderPost(post) {
        const user = post.user;
        const displayName = post.is_anonymous ? 'Anonymous' : (user?.display_name || 'Unknown');
        const initial = displayName.charAt(0).toUpperCase();
        const timeAgo = this.formatTimeAgo(post.created_at);
        const visibilityLabel = {
            'public': 'Public',
            'friends_only': 'Friends',
            'private': 'Only me',
            'group': 'Group'
        }[post.visibility] || post.visibility;

        return `
            <div class="post-card" data-post-id="${post.id}">
                <div class="post-header">
                    <div class="avatar">
                        <span>${initial}</span>
                    </div>
                    <div class="post-user-info">
                        <div class="name">
                            ${displayName}
                            <span class="visibility-badge">${visibilityLabel}</span>
                        </div>
                        <div class="time">${timeAgo}</div>
                    </div>
                </div>
                <div class="post-content">${this.escapeHtml(post.content)}</div>
                <div class="post-stats">
                    <span>${post.likes_count} likes</span>
                    <span>${post.comments_count} comments</span>
                </div>
                <div class="post-actions">
                    <button class="post-action-btn like-btn ${post.is_liked ? 'liked' : ''}" data-post-id="${post.id}">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"/>
                        </svg>
                        Like
                    </button>
                    <button class="post-action-btn comment-btn" data-post-id="${post.id}">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>
                        </svg>
                        Comment
                    </button>
                </div>
            </div>
        `;
    }

    bindPostEvents() {
        document.querySelectorAll('.like-btn').forEach(btn => {
            btn.addEventListener('click', () => this.likePost(btn.dataset.postId));
        });
        document.querySelectorAll('.comment-btn').forEach(btn => {
            btn.addEventListener('click', () => this.openComments(btn.dataset.postId));
        });
    }

    async createPost() {
        const content = document.getElementById('post-content').value.trim();
        const visibility = document.getElementById('post-visibility').value;

        if (!content) {
            this.showToast('Please write something', 'error');
            return;
        }

        try {
            await api.createPost(content, visibility);
            this.closeModal('post-modal');
            document.getElementById('post-content').value = '';
            this.loadFeed();
            this.showToast('Post created!', 'success');
        } catch (error) {
            this.showToast(error.message, 'error');
        }
    }

    async likePost(postId) {
        try {
            const result = await api.likePost(postId);
            const btn = document.querySelector(`.like-btn[data-post-id="${postId}"]`);
            btn.classList.toggle('liked', result.liked);
            this.loadFeed();
        } catch (error) {
            this.showToast(error.message, 'error');
        }
    }

    async openComments(postId) {
        this.currentPostId = postId;
        this.openModal('comment-modal');
        
        const commentsContainer = document.getElementById('comments-list');
        commentsContainer.innerHTML = '<div class="loading-spinner"><div class="spinner"></div></div>';

        try {
            const comments = await api.getComments(postId);
            
            if (comments.length === 0) {
                commentsContainer.innerHTML = '<p style="text-align: center; color: var(--text-muted);">No comments yet</p>';
                return;
            }

            commentsContainer.innerHTML = comments.map(comment => {
                const displayName = comment.is_anonymous ? 'Anonymous' : (comment.user?.display_name || 'Unknown');
                const initial = displayName.charAt(0).toUpperCase();
                return `
                    <div class="comment-item">
                        <div class="avatar">
                            <span>${initial}</span>
                        </div>
                        <div class="comment-content">
                            <div class="name">${displayName}</div>
                            <div class="text">${this.escapeHtml(comment.content)}</div>
                            <div class="time">${this.formatTimeAgo(comment.created_at)}</div>
                        </div>
                    </div>
                `;
            }).join('');
        } catch (error) {
            commentsContainer.innerHTML = '<p>Failed to load comments</p>';
        }
    }

    async submitComment() {
        const content = document.getElementById('comment-input').value.trim();
        const isAnonymous = document.getElementById('comment-anonymous').checked;

        if (!content) return;

        try {
            await api.addComment(this.currentPostId, content, isAnonymous);
            document.getElementById('comment-input').value = '';
            document.getElementById('comment-anonymous').checked = false;
            this.openComments(this.currentPostId);
            this.loadFeed();
        } catch (error) {
            this.showToast(error.message, 'error');
        }
    }

    // Friends
    async loadFriends() {
        await this.loadFriendsList();
        await this.loadFriendRequests();
    }

    async loadFriendsList() {
        const container = document.getElementById('friends-list');
        container.innerHTML = '<div class="loading-spinner"><div class="spinner"></div></div>';

        try {
            const friends = await api.getFriends();
            
            if (friends.length === 0) {
                container.innerHTML = `
                    <div class="empty-state" style="grid-column: 1 / -1;">
                        <p>No friends yet. Start connecting!</p>
                    </div>
                `;
                return;
            }

            container.innerHTML = friends.map(friend => `
                <div class="user-card">
                    <div class="avatar">
                        <span>${friend.user.display_name.charAt(0).toUpperCase()}</span>
                    </div>
                    <div class="user-card-info">
                        <div class="name">${friend.user.display_name}</div>
                        <div class="username">@${friend.user.username}</div>
                    </div>
                    <div class="user-card-actions">
                        <button class="btn btn-primary btn-small" onclick="app.startChat('${friend.user.id}')">Message</button>
                    </div>
                </div>
            `).join('');
        } catch (error) {
            container.innerHTML = '<div class="empty-state"><p>Failed to load friends</p></div>';
        }
    }

    async loadFriendRequests() {
        const container = document.getElementById('friend-requests-list');
        
        try {
            const requests = await api.getFriendRequests();
            
            // Update badge
            const badge = document.getElementById('friend-requests-badge');
            const count = document.getElementById('requests-count');
            if (requests.length > 0) {
                badge.textContent = requests.length;
                badge.classList.remove('hidden');
                count.textContent = requests.length;
            } else {
                badge.classList.add('hidden');
                count.textContent = '0';
            }

            if (requests.length === 0) {
                container.innerHTML = `
                    <div class="empty-state" style="grid-column: 1 / -1;">
                        <p>No pending friend requests</p>
                    </div>
                `;
                return;
            }

            container.innerHTML = requests.map(request => `
                <div class="user-card" data-user-id="${request.user.id}">
                    <div class="avatar">
                        <span>${request.user.display_name.charAt(0).toUpperCase()}</span>
                    </div>
                    <div class="user-card-info">
                        <div class="name">${request.user.display_name}</div>
                        <div class="username">@${request.user.username}</div>
                    </div>
                    <div class="user-card-actions">
                        <button class="btn btn-primary btn-small accept-btn">Accept</button>
                        <button class="btn btn-secondary btn-small reject-btn">Reject</button>
                    </div>
                </div>
            `).join('');

            // Bind events
            container.querySelectorAll('.accept-btn').forEach(btn => {
                btn.addEventListener('click', async () => {
                    const userId = btn.closest('.user-card').dataset.userId;
                    await this.acceptFriendRequest(userId);
                });
            });
            container.querySelectorAll('.reject-btn').forEach(btn => {
                btn.addEventListener('click', async () => {
                    const userId = btn.closest('.user-card').dataset.userId;
                    await this.rejectFriendRequest(userId);
                });
            });
        } catch (error) {
            console.error('Failed to load friend requests:', error);
        }
    }

    async searchFriends(query) {
        const container = document.getElementById('find-friends-list');
        
        if (!query.trim()) {
            container.innerHTML = '';
            return;
        }

        try {
            const users = await api.searchUsers(query);
            
            if (users.length === 0) {
                container.innerHTML = '<div class="empty-state"><p>No users found</p></div>';
                return;
            }

            container.innerHTML = users.map(user => `
                <div class="user-card" data-user-id="${user.id}">
                    <div class="avatar">
                        <span>${user.display_name.charAt(0).toUpperCase()}</span>
                    </div>
                    <div class="user-card-info">
                        <div class="name">${user.display_name}</div>
                        <div class="username">@${user.username}</div>
                    </div>
                    <div class="user-card-actions">
                        <button class="btn btn-primary btn-small add-friend-btn">Add Friend</button>
                    </div>
                </div>
            `).join('');

            // Bind events
            container.querySelectorAll('.add-friend-btn').forEach(btn => {
                btn.addEventListener('click', async () => {
                    const userId = btn.closest('.user-card').dataset.userId;
                    await this.sendFriendRequest(userId, btn);
                });
            });
        } catch (error) {
            container.innerHTML = '<div class="empty-state"><p>Search failed</p></div>';
        }
    }

    async sendFriendRequest(userId, btn) {
        try {
            await api.sendFriendRequest(userId);
            btn.textContent = 'Sent';
            btn.disabled = true;
            this.showToast('Friend request sent!', 'success');
        } catch (error) {
            this.showToast(error.message, 'error');
        }
    }

    async acceptFriendRequest(userId) {
        try {
            await api.acceptFriendRequest(userId);
            this.loadFriendRequests();
            this.loadFriendsList();
            this.showToast('Friend request accepted!', 'success');
        } catch (error) {
            this.showToast(error.message, 'error');
        }
    }

    async rejectFriendRequest(userId) {
        try {
            await api.rejectFriendRequest(userId);
            this.loadFriendRequests();
            this.showToast('Friend request rejected', 'success');
        } catch (error) {
            this.showToast(error.message, 'error');
        }
    }

    switchFriendsTab(tab) {
        document.querySelectorAll('.friends-tabs .tab-btn').forEach(btn => {
            btn.classList.toggle('active', btn.dataset.tab === tab);
        });
        document.querySelectorAll('.tab-content').forEach(content => {
            content.classList.add('hidden');
        });
        document.getElementById(`${tab}-tab`).classList.remove('hidden');
    }

    // Groups
    async loadGroups() {
        const container = document.getElementById('groups-list');
        container.innerHTML = '<div class="loading-spinner"><div class="spinner"></div></div>';

        document.getElementById('group-detail').classList.add('hidden');
        document.querySelector('.groups-container').classList.remove('hidden');

        try {
            const groups = await api.getGroups();
            
            if (groups.length === 0) {
                container.innerHTML = `
                    <div class="empty-state" style="grid-column: 1 / -1;">
                        <p>No groups yet. Create one to get started!</p>
                    </div>
                `;
                return;
            }

            container.innerHTML = groups.map(group => `
                <div class="group-card" data-group-id="${group.id}">
                    <div class="group-card-header">
                        <div class="group-icon">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/>
                                <circle cx="9" cy="7" r="4"/>
                                <path d="M23 21v-2a4 4 0 0 0-4-4h-4"/>
                                <circle cx="17" cy="7" r="4"/>
                            </svg>
                        </div>
                        <div class="group-card-info">
                            <h4>${this.escapeHtml(group.name)}</h4>
                            <span>${group.members_count} members</span>
                        </div>
                    </div>
                    <p>${this.escapeHtml(group.description || 'No description')}</p>
                    ${group.is_private ? '<span class="private-badge">Private</span>' : ''}
                </div>
            `).join('');

            // Bind click events
            container.querySelectorAll('.group-card').forEach(card => {
                card.addEventListener('click', () => this.openGroup(card.dataset.groupId));
            });
        } catch (error) {
            container.innerHTML = '<div class="empty-state"><p>Failed to load groups</p></div>';
        }
    }

    async createGroup() {
        const name = document.getElementById('group-name').value.trim();
        const description = document.getElementById('group-description').value.trim();
        const isPrivate = document.getElementById('group-private').checked;

        if (!name) {
            this.showToast('Please enter a group name', 'error');
            return;
        }

        try {
            await api.createGroup(name, description, isPrivate);
            this.closeModal('create-group-modal');
            document.getElementById('group-name').value = '';
            document.getElementById('group-description').value = '';
            document.getElementById('group-private').checked = false;
            this.loadGroups();
            this.showToast('Group created!', 'success');
        } catch (error) {
            this.showToast(error.message, 'error');
        }
    }

    async openGroup(groupId) {
        this.currentGroup = groupId;
        
        document.querySelector('.groups-container').classList.add('hidden');
        document.getElementById('group-detail').classList.remove('hidden');

        try {
            const group = await api.getGroup(groupId);
            document.getElementById('group-detail-name').textContent = group.name;
            document.getElementById('group-detail-description').textContent = group.description || '';
            document.getElementById('group-detail-members').textContent = `${group.members_count} members`;

            await this.loadGroupPosts(groupId);
        } catch (error) {
            this.showToast('Failed to load group', 'error');
        }
    }

    async loadGroupPosts(groupId) {
        const container = document.getElementById('group-posts');
        container.innerHTML = '<div class="loading-spinner"><div class="spinner"></div></div>';

        try {
            const posts = await api.getGroupPosts(groupId);
            
            if (posts.length === 0) {
                container.innerHTML = `
                    <div class="empty-state">
                        <p>No posts in this group yet</p>
                    </div>
                `;
                return;
            }

            container.innerHTML = posts.map(post => this.renderPost(post)).join('');
            this.bindPostEvents();
        } catch (error) {
            container.innerHTML = '<div class="empty-state"><p>Failed to load posts</p></div>';
        }
    }

    async createGroupPost() {
        const content = document.getElementById('group-post-content').value.trim();
        const isAnonymous = document.getElementById('group-post-anonymous').checked;

        if (!content) {
            this.showToast('Please write something', 'error');
            return;
        }

        try {
            await api.createGroupPost(this.currentGroup, content, isAnonymous);
            this.closeModal('group-post-modal');
            document.getElementById('group-post-content').value = '';
            document.getElementById('group-post-anonymous').checked = false;
            this.loadGroupPosts(this.currentGroup);
            this.showToast('Post created!', 'success');
        } catch (error) {
            this.showToast(error.message, 'error');
        }
    }

    closeGroupDetail() {
        this.currentGroup = null;
        document.getElementById('group-detail').classList.add('hidden');
        document.querySelector('.groups-container').classList.remove('hidden');
    }

    // Chat
    async loadConversations() {
        const container = document.getElementById('conversations-list');
        container.innerHTML = '<div class="loading-spinner"><div class="spinner"></div></div>';

        try {
            const conversations = await api.getConversations();
            
            if (conversations.length === 0) {
                container.innerHTML = `
                    <div class="empty-state">
                        <p>No conversations yet</p>
                    </div>
                `;
                return;
            }

            container.innerHTML = conversations.map(conv => `
                <div class="conversation-item" data-user-id="${conv.user.id}">
                    <div class="avatar">
                        <span>${conv.user.display_name.charAt(0).toUpperCase()}</span>
                    </div>
                    <div class="conversation-info">
                        <div class="name">
                            ${conv.user.display_name}
                            <span class="time">${conv.last_message ? this.formatTimeAgo(conv.last_message.created_at) : ''}</span>
                        </div>
                        <div class="preview">${conv.last_message ? 'Encrypted message' : 'Start chatting'}</div>
                    </div>
                    ${conv.unread_count > 0 ? `<div class="unread-badge">${conv.unread_count}</div>` : ''}
                </div>
            `).join('');

            // Bind click events
            container.querySelectorAll('.conversation-item').forEach(item => {
                item.addEventListener('click', () => this.startChat(item.dataset.userId));
            });
        } catch (error) {
            container.innerHTML = '<div class="empty-state"><p>Failed to load conversations</p></div>';
        }
    }

    async startChat(userId) {
        this.currentChatUser = userId;
        
        // Navigate to chat if not already there
        if (this.currentPage !== 'chat') {
            this.navigateTo('chat');
        }

        // Update UI
        document.getElementById('chat-placeholder').classList.add('hidden');
        document.getElementById('chat-window').classList.remove('hidden');

        // Mark conversation as active
        document.querySelectorAll('.conversation-item').forEach(item => {
            item.classList.toggle('active', item.dataset.userId === userId);
        });

        try {
            // Get user info
            const user = await api.getUser(userId);
            document.getElementById('chat-user-name').textContent = user.display_name;
            document.getElementById('chat-avatar-initial').textContent = user.display_name.charAt(0).toUpperCase();

            // Get and store peer's public key
            try {
                const keyResponse = await api.getPublicKey(userId);
                await e2eCrypto.storePeerPublicKey(userId, keyResponse.public_key);
            } catch (error) {
                console.warn('Could not get peer public key:', error);
            }

            // Load messages
            await this.loadMessages(userId);
        } catch (error) {
            this.showToast('Failed to start chat', 'error');
        }
    }

    async loadMessages(userId) {
        const container = document.getElementById('messages-container');
        container.innerHTML = '<div class="loading-spinner"><div class="spinner"></div></div>';

        try {
            const messages = await api.getMessages(userId);
            
            if (messages.length === 0) {
                container.innerHTML = `
                    <div class="empty-state">
                        <p>No messages yet. Start the conversation!</p>
                    </div>
                `;
                return;
            }

            const decryptedMessages = await Promise.all(messages.map(async (msg) => {
                const peerId = msg.sender_id === this.currentUser.id ? msg.receiver_id : msg.sender_id;
                let content;
                try {
                    content = await e2eCrypto.decryptMessage(peerId, msg.encrypted_content, msg.iv);
                } catch (error) {
                    content = '[Encrypted message]';
                }
                return { ...msg, content };
            }));

            container.innerHTML = decryptedMessages.map(msg => {
                const isSent = msg.sender_id === this.currentUser.id;
                return `
                    <div class="message ${isSent ? 'sent' : 'received'}">
                        <div class="text">${this.escapeHtml(msg.content)}</div>
                        <div class="time">${this.formatTime(msg.created_at)}</div>
                    </div>
                `;
            }).join('');

            // Scroll to bottom
            container.scrollTop = container.scrollHeight;
        } catch (error) {
            container.innerHTML = '<div class="empty-state"><p>Failed to load messages</p></div>';
        }
    }

    async sendMessage() {
        const input = document.getElementById('message-input');
        const content = input.value.trim();

        if (!content || !this.currentChatUser) return;

        try {
            const encrypted = await e2eCrypto.encryptMessage(this.currentChatUser, content);
            chatWs.sendMessage(this.currentChatUser, encrypted.encrypted_content, encrypted.iv);
            
            input.value = '';
            
            // Add message to UI immediately
            const container = document.getElementById('messages-container');
            const emptyState = container.querySelector('.empty-state');
            if (emptyState) emptyState.remove();
            
            container.innerHTML += `
                <div class="message sent">
                    <div class="text">${this.escapeHtml(content)}</div>
                    <div class="time">${this.formatTime(new Date().toISOString())}</div>
                </div>
            `;
            container.scrollTop = container.scrollHeight;
        } catch (error) {
            this.showToast('Failed to send message', 'error');
        }
    }

    async handleIncomingMessage(msg) {
        // If it's from or to the current chat user, add to UI
        if (msg.sender_id === this.currentChatUser || msg.receiver_id === this.currentChatUser) {
            const peerId = msg.sender_id === this.currentUser.id ? msg.receiver_id : msg.sender_id;
            
            // Only add if it's from the other user (not our own message echoed back)
            if (msg.sender_id !== this.currentUser.id) {
                let content;
                try {
                    content = await e2eCrypto.decryptMessage(peerId, msg.encrypted_content, msg.iv);
                } catch (error) {
                    content = '[Encrypted message]';
                }

                const container = document.getElementById('messages-container');
                const emptyState = container.querySelector('.empty-state');
                if (emptyState) emptyState.remove();
                
                container.innerHTML += `
                    <div class="message received">
                        <div class="text">${this.escapeHtml(content)}</div>
                        <div class="time">${this.formatTime(msg.created_at)}</div>
                    </div>
                `;
                container.scrollTop = container.scrollHeight;
            }
        }

        // Update conversations list
        this.loadConversations();
    }

    handleTyping() {
        if (this.currentChatUser && chatWs.isConnected) {
            chatWs.sendTyping(this.currentChatUser);
        }
    }

    handleTypingIndicator(senderId) {
        if (senderId === this.currentChatUser) {
            const indicator = document.getElementById('typing-indicator');
            indicator.classList.remove('hidden');
            
            clearTimeout(this.typingTimeout);
            this.typingTimeout = setTimeout(() => {
                indicator.classList.add('hidden');
            }, 2000);
        }
    }

    // Utility methods
    openModal(modalId) {
        document.getElementById(modalId).classList.remove('hidden');
    }

    closeModal(modalId) {
        document.getElementById(modalId).classList.add('hidden');
    }

    showToast(message, type = '') {
        const toast = document.getElementById('toast');
        toast.textContent = message;
        toast.className = `toast ${type}`;
        toast.classList.remove('hidden');
        
        setTimeout(() => {
            toast.classList.add('hidden');
        }, 3000);
    }

    formatTimeAgo(dateString) {
        const date = new Date(dateString);
        const now = new Date();
        const diff = Math.floor((now - date) / 1000);

        if (diff < 60) return 'Just now';
        if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
        if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
        if (diff < 604800) return `${Math.floor(diff / 86400)}d ago`;
        
        return date.toLocaleDateString();
    }

    formatTime(dateString) {
        const date = new Date(dateString);
        return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    }

    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    debounce(func, wait) {
        let timeout;
        return function executedFunction(...args) {
            const later = () => {
                clearTimeout(timeout);
                func(...args);
            };
            clearTimeout(timeout);
            timeout = setTimeout(later, wait);
        };
    }
}

// Initialize app
const app = new SocialSpaceApp();
