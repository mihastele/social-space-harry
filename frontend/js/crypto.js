/**
 * End-to-End Encryption Module
 * Uses Web Crypto API for secure message encryption
 */

class E2ECrypto {
    constructor() {
        this.keyPair = null;
        this.peerPublicKeys = new Map();
    }

    /**
     * Generate a new ECDH key pair for the current user
     */
    async generateKeyPair() {
        try {
            this.keyPair = await window.crypto.subtle.generateKey(
                {
                    name: 'ECDH',
                    namedCurve: 'P-256'
                },
                true,
                ['deriveKey', 'deriveBits']
            );
            return this.keyPair;
        } catch (error) {
            console.error('Failed to generate key pair:', error);
            throw error;
        }
    }

    /**
     * Export the public key as a base64 string
     */
    async exportPublicKey() {
        if (!this.keyPair) {
            await this.generateKeyPair();
        }
        
        const exported = await window.crypto.subtle.exportKey('spki', this.keyPair.publicKey);
        return this.arrayBufferToBase64(exported);
    }

    /**
     * Import a peer's public key from base64 string
     */
    async importPublicKey(base64Key) {
        const keyData = this.base64ToArrayBuffer(base64Key);
        return await window.crypto.subtle.importKey(
            'spki',
            keyData,
            {
                name: 'ECDH',
                namedCurve: 'P-256'
            },
            true,
            []
        );
    }

    /**
     * Store a peer's public key
     */
    async storePeerPublicKey(peerId, base64Key) {
        const publicKey = await this.importPublicKey(base64Key);
        this.peerPublicKeys.set(peerId, publicKey);
    }

    /**
     * Derive a shared secret key for encryption/decryption
     */
    async deriveSharedKey(peerPublicKey) {
        if (!this.keyPair) {
            throw new Error('Key pair not generated');
        }

        const sharedSecret = await window.crypto.subtle.deriveBits(
            {
                name: 'ECDH',
                public: peerPublicKey
            },
            this.keyPair.privateKey,
            256
        );

        return await window.crypto.subtle.importKey(
            'raw',
            sharedSecret,
            { name: 'AES-GCM' },
            false,
            ['encrypt', 'decrypt']
        );
    }

    /**
     * Encrypt a message for a specific peer
     */
    async encryptMessage(peerId, plaintext) {
        const peerPublicKey = this.peerPublicKeys.get(peerId);
        if (!peerPublicKey) {
            throw new Error('Peer public key not found');
        }

        const sharedKey = await this.deriveSharedKey(peerPublicKey);
        
        // Generate random IV
        const iv = window.crypto.getRandomValues(new Uint8Array(12));
        
        // Encode message
        const encoder = new TextEncoder();
        const data = encoder.encode(plaintext);

        // Encrypt
        const encryptedData = await window.crypto.subtle.encrypt(
            {
                name: 'AES-GCM',
                iv: iv
            },
            sharedKey,
            data
        );

        return {
            encrypted_content: this.arrayBufferToBase64(encryptedData),
            iv: this.arrayBufferToBase64(iv)
        };
    }

    /**
     * Decrypt a message from a specific peer
     */
    async decryptMessage(peerId, encryptedContent, ivBase64) {
        const peerPublicKey = this.peerPublicKeys.get(peerId);
        if (!peerPublicKey) {
            throw new Error('Peer public key not found');
        }

        const sharedKey = await this.deriveSharedKey(peerPublicKey);
        const iv = this.base64ToArrayBuffer(ivBase64);
        const encryptedData = this.base64ToArrayBuffer(encryptedContent);

        try {
            const decryptedData = await window.crypto.subtle.decrypt(
                {
                    name: 'AES-GCM',
                    iv: iv
                },
                sharedKey,
                encryptedData
            );

            const decoder = new TextDecoder();
            return decoder.decode(decryptedData);
        } catch (error) {
            console.error('Decryption failed:', error);
            return '[Unable to decrypt message]';
        }
    }

    /**
     * Save key pair to localStorage
     */
    async saveKeyPair() {
        if (!this.keyPair) return;

        const privateKeyJwk = await window.crypto.subtle.exportKey('jwk', this.keyPair.privateKey);
        const publicKeyJwk = await window.crypto.subtle.exportKey('jwk', this.keyPair.publicKey);

        localStorage.setItem('e2e_private_key', JSON.stringify(privateKeyJwk));
        localStorage.setItem('e2e_public_key', JSON.stringify(publicKeyJwk));
    }

    /**
     * Load key pair from localStorage
     */
    async loadKeyPair() {
        const privateKeyJwk = localStorage.getItem('e2e_private_key');
        const publicKeyJwk = localStorage.getItem('e2e_public_key');

        if (!privateKeyJwk || !publicKeyJwk) {
            return false;
        }

        try {
            const privateKey = await window.crypto.subtle.importKey(
                'jwk',
                JSON.parse(privateKeyJwk),
                {
                    name: 'ECDH',
                    namedCurve: 'P-256'
                },
                true,
                ['deriveKey', 'deriveBits']
            );

            const publicKey = await window.crypto.subtle.importKey(
                'jwk',
                JSON.parse(publicKeyJwk),
                {
                    name: 'ECDH',
                    namedCurve: 'P-256'
                },
                true,
                []
            );

            this.keyPair = { privateKey, publicKey };
            return true;
        } catch (error) {
            console.error('Failed to load key pair:', error);
            return false;
        }
    }

    /**
     * Initialize crypto - load existing keys or generate new ones
     */
    async initialize() {
        const loaded = await this.loadKeyPair();
        if (!loaded) {
            await this.generateKeyPair();
            await this.saveKeyPair();
        }
        return await this.exportPublicKey();
    }

    // Utility functions
    arrayBufferToBase64(buffer) {
        const bytes = new Uint8Array(buffer);
        let binary = '';
        for (let i = 0; i < bytes.byteLength; i++) {
            binary += String.fromCharCode(bytes[i]);
        }
        return btoa(binary);
    }

    base64ToArrayBuffer(base64) {
        const binary = atob(base64);
        const bytes = new Uint8Array(binary.length);
        for (let i = 0; i < binary.length; i++) {
            bytes[i] = binary.charCodeAt(i);
        }
        return bytes.buffer;
    }
}

// Export singleton instance
window.e2eCrypto = new E2ECrypto();
