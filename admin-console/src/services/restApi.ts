import { useApi } from '@/composables/useApi'

const { api } = useApi()

export interface Collection {
    name: string
    row_count: number
    columns: Array<{
        name: string
        type: string
        nullable: boolean
        default_value?: string
    }>
}

export interface ApiKey {
    id: string
    name: string
    key_prefix: string  // First 8 chars only for display
    permissions: string[]
    created_at: string
    last_used_at?: string
    expires_at?: string
}

export interface ApiRequestResult {
    status: number
    headers: Record<string, string>
    body: unknown
    duration_ms: number
}

export const restApiService = {
    /**
     * Get all collections/tables
     */
    async getCollections(): Promise<Collection[]> {
        const response = await api.get('/api/_schema/collections')
        return response.data
    },

    /**
     * Get schema for a specific collection
     */
    async getCollectionSchema(name: string): Promise<Collection> {
        const response = await api.get(`/api/_schema/collections/${name}`)
        return response.data
    },

    /**
     * Execute an API request (for testing)
     */
    async executeRequest(
        method: 'GET' | 'POST' | 'PATCH' | 'DELETE',
        path: string,
        body?: unknown,
        headers?: Record<string, string>
    ): Promise<ApiRequestResult> {
        const startTime = Date.now()

        try {
            const response = await api.request({
                method,
                url: path,
                data: body,
                headers,
            })

            return {
                status: response.status,
                headers: response.headers as Record<string, string>,
                body: response.data,
                duration_ms: Date.now() - startTime,
            }
        } catch (error: unknown) {
            const axiosError = error as { response?: { status: number; headers: Record<string, string>; data: unknown } }
            if (axiosError.response) {
                return {
                    status: axiosError.response.status,
                    headers: axiosError.response.headers,
                    body: axiosError.response.data,
                    duration_ms: Date.now() - startTime,
                }
            }
            throw error
        }
    },

    /**
     * Get OpenAPI schema
     */
    async getOpenApiSchema(): Promise<unknown> {
        const response = await api.get('/api/_schema/openapi')
        return response.data
    },

    // ========== API Keys ==========

    /**
     * Get all API keys
     */
    async getApiKeys(): Promise<ApiKey[]> {
        const response = await api.get('/auth/api-keys')
        return response.data
    },

    /**
     * Create a new API key
     */
    async createApiKey(data: {
        name: string
        permissions: string[]
        expires_in_days?: number
    }): Promise<{ key: ApiKey; secret: string }> {
        const response = await api.post('/auth/api-keys', data)
        return response.data
    },

    /**
     * Revoke an API key
     */
    async revokeApiKey(keyId: string): Promise<void> {
        await api.delete(`/auth/api-keys/${keyId}`)
    },

    /**
     * Get available permissions for API keys
     */
    async getAvailablePermissions(): Promise<Array<{
        name: string
        description: string
    }>> {
        const response = await api.get('/auth/api-keys/permissions')
        return response.data
    },
}
