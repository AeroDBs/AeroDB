import { useApi } from '@/composables/useApi'
import type { User, Session, AuthTokens } from '@/types'

const { api } = useApi()

export const authService = {
    /**
     * Sign in with email and password
     */
    async signIn(email: string, password: string): Promise<{
        user: User
        tokens: AuthTokens
    }> {
        const response = await api.post('/auth/login', { email, password })
        return response.data
    },

    /**
     * Sign up a new user
     */
    async signUp(email: string, password: string, metadata?: Record<string, unknown>): Promise<{
        user: User
        tokens: AuthTokens
    }> {
        const response = await api.post('/auth/signup', { email, password, metadata })
        return response.data
    },

    /**
     * Sign out the current user
     */
    async signOut(): Promise<void> {
        await api.post('/auth/logout')
    },

    /**
     * Refresh access token
     */
    async refreshToken(refreshToken: string): Promise<AuthTokens> {
        const response = await api.post('/auth/refresh', { refresh_token: refreshToken })
        return response.data
    },

    /**
     * Get all users (admin only)
     */
    async getUsers(options?: {
        limit?: number
        offset?: number
        search?: string
    }): Promise<{
        users: User[]
        total: number
    }> {
        const params = new URLSearchParams()
        if (options?.limit) params.append('limit', options.limit.toString())
        if (options?.offset) params.append('offset', options.offset.toString())
        if (options?.search) params.append('search', options.search)

        const response = await api.get(`/auth/users?${params}`)
        return response.data
    },

    /**
     * Get a specific user
     */
    async getUser(userId: string): Promise<User> {
        const response = await api.get(`/auth/users/${userId}`)
        return response.data
    },

    /**
     * Create a new user (admin only)
     */
    async createUser(data: {
        email: string
        password: string
        role?: string
        metadata?: Record<string, unknown>
    }): Promise<User> {
        const response = await api.post('/auth/users', data)
        return response.data
    },

    /**
     * Update a user
     */
    async updateUser(userId: string, data: Partial<User>): Promise<User> {
        const response = await api.patch(`/auth/users/${userId}`, data)
        return response.data
    },

    /**
     * Delete a user
     */
    async deleteUser(userId: string): Promise<void> {
        await api.delete(`/auth/users/${userId}`)
    },

    /**
     * Get all active sessions
     */
    async getSessions(userId?: string): Promise<Session[]> {
        const url = userId ? `/auth/sessions?user_id=${userId}` : '/auth/sessions'
        const response = await api.get(url)
        return response.data
    },

    /**
     * Revoke a session
     */
    async revokeSession(sessionId: string): Promise<void> {
        await api.delete(`/auth/sessions/${sessionId}`)
    },

    /**
     * Get RLS policies for a table
     */
    async getRLSPolicies(tableName: string): Promise<Array<{
        id: string
        name: string
        operation: 'select' | 'insert' | 'update' | 'delete'
        expression: string
        enabled: boolean
    }>> {
        const response = await api.get(`/auth/rls/${tableName}`)
        return response.data
    },

    /**
     * Create an RLS policy
     */
    async createRLSPolicy(tableName: string, policy: {
        name: string
        operation: 'select' | 'insert' | 'update' | 'delete'
        expression: string
    }): Promise<void> {
        await api.post(`/auth/rls/${tableName}`, policy)
    },

    /**
     * Delete an RLS policy
     */
    async deleteRLSPolicy(tableName: string, policyId: string): Promise<void> {
        await api.delete(`/auth/rls/${tableName}/${policyId}`)
    },

    /**
     * Toggle RLS policy enabled status
     */
    async toggleRLSPolicy(tableName: string, policyId: string, enabled: boolean): Promise<void> {
        await api.patch(`/auth/rls/${tableName}/${policyId}`, { enabled })
    },

    // ========== Password Management ==========

    /**
     * Request password reset email
     */
    async forgotPassword(email: string): Promise<void> {
        await api.post('/auth/forgot-password', { email })
    },

    /**
     * Reset password using token from email
     */
    async resetPassword(token: string, newPassword: string): Promise<void> {
        await api.post('/auth/reset-password', { token, new_password: newPassword })
    },

    /**
     * Change password for authenticated user
     */
    async changePassword(currentPassword: string, newPassword: string): Promise<void> {
        await api.post('/auth/change-password', {
            current_password: currentPassword,
            new_password: newPassword
        })
    },

    /**
     * Get password policy requirements
     */
    async getPasswordPolicy(): Promise<{
        min_length: number
        require_uppercase: boolean
        require_lowercase: boolean
        require_numbers: boolean
        require_symbols: boolean
    }> {
        const response = await api.get('/auth/password-policy')
        return response.data
    },

    // ========== Email Verification ==========

    /**
     * Verify email using token
     */
    async verifyEmail(token: string): Promise<void> {
        await api.post('/auth/verify-email', { token })
    },

    /**
     * Resend verification email
     */
    async resendVerificationEmail(userId: string): Promise<void> {
        await api.post(`/auth/users/${userId}/resend-verification`)
    },
}

