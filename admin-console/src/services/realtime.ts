import { useApi } from '@/composables/useApi'
import type { Subscription } from '@/types'

const { api } = useApi()

export const realtimeService = {
    /**
     * Get all active subscriptions
     */
    async getSubscriptions(): Promise<Subscription[]> {
        const response = await api.get('/realtime/subscriptions')
        return response.data
    },

    /**
     * Get subscriptions for a specific user
     */
    async getUserSubscriptions(userId: string): Promise<Subscription[]> {
        const response = await api.get(`/realtime/subscriptions?user_id=${userId}`)
        return response.data
    },

    /**
     * Get subscriptions for a specific channel
     */
    async getChannelSubscriptions(channel: string): Promise<Subscription[]> {
        const response = await api.get(`/realtime/subscriptions?channel=${channel}`)
        return response.data
    },

    /**
     * Broadcast a message to a channel
     */
    async broadcast(channel: string, event: string, payload: unknown): Promise<{
        recipients: number
    }> {
        const response = await api.post('/realtime/broadcast', {
            channel,
            event,
            payload,
        })
        return response.data
    },

    /**
     * Disconnect a specific subscription
     */
    async disconnectSubscription(subscriptionId: string): Promise<void> {
        await api.delete(`/realtime/subscriptions/${subscriptionId}`)
    },

    /**
     * Get realtime statistics
     */
    async getRealtimeStats(): Promise<{
        total_connections: number
        total_channels: number
        messages_per_second: number
        bytes_per_second: number
    }> {
        const response = await api.get('/realtime/stats')
        return response.data
    },

    /**
     * Get WebSocket connection URL
     */
    getWebSocketUrl(): string {
        const { baseURL } = useApi()
        // Convert http:// to ws:// and https:// to wss://
        const wsBaseUrl = baseURL.replace(/^http/, 'ws')
        return `${wsBaseUrl}/realtime`
    },
}
