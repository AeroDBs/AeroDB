import { useApi } from '@/composables/useApi'

const { api } = useApi()

// ==================
// Types
// ==================

export interface SlowQueryConfig {
    enabled: boolean
    threshold_ms: number
    emit_log: boolean
    webhook_url: string | null
    webhook_timeout_ms: number
}

export interface BackpressureConfig {
    max_pending_messages: number
    drop_policy: 'oldest_first' | 'newest_first' | 'reject'
}

export interface ObservabilitySettings {
    slow_query: SlowQueryConfig
    operation_log_enabled: boolean
}

export interface RealtimeSettings {
    backpressure: BackpressureConfig
}

export interface Settings {
    observability: ObservabilitySettings
    realtime: RealtimeSettings
}

// ==================
// Service
// ==================

export const settingsService = {
    /**
     * Get all current settings
     */
    async getSettings(): Promise<Settings> {
        const response = await api.get('/settings')
        return response.data
    },

    /**
     * Get observability settings
     */
    async getObservabilitySettings(): Promise<ObservabilitySettings> {
        const response = await api.get('/settings/observability')
        return response.data
    },

    /**
     * Update observability settings
     */
    async updateObservabilitySettings(settings: Partial<ObservabilitySettings>): Promise<ObservabilitySettings> {
        const response = await api.patch('/settings/observability', settings)
        return response.data
    },

    /**
     * Get realtime settings
     */
    async getRealtimeSettings(): Promise<RealtimeSettings> {
        const response = await api.get('/settings/realtime')
        return response.data
    },

    /**
     * Update realtime settings
     */
    async updateRealtimeSettings(settings: Partial<RealtimeSettings>): Promise<RealtimeSettings> {
        const response = await api.patch('/settings/realtime', settings)
        return response.data
    },
}
