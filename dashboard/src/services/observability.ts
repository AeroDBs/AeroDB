import { useApi } from '@/composables/useApi'
import type { LogEntry, MetricDataPoint, LogFilter } from '@/types'

const { api } = useApi()

export const observabilityService = {
    /**
     * Get logs with filtering
     */
    async getLogs(options?: {
        limit?: number
        offset?: number
        level?: 'debug' | 'info' | 'warn' | 'error'
        module?: string
        search?: string
        since?: string
        until?: string
    }): Promise<{
        logs: LogEntry[]
        total: number
    }> {
        const params = new URLSearchParams()
        if (options?.limit) params.append('limit', options.limit.toString())
        if (options?.offset) params.append('offset', options.offset.toString())
        if (options?.level) params.append('level', options.level)
        if (options?.module) params.append('module', options.module)
        if (options?.search) params.append('search', options.search)
        if (options?.since) params.append('since', options.since)
        if (options?.until) params.append('until', options.until)

        const response = await api.get(`/observability/logs?${params}`)
        return response.data
    },

    /**
     * Stream logs in real-time (returns event source URL)
     */
    getLogStreamUrl(filter?: LogFilter): string {
        const { baseURL } = useApi()
        const params = new URLSearchParams()
        if (filter?.level) params.append('level', filter.level)
        if (filter?.module) params.append('module', filter.module)

        return `${baseURL}/observability/logs/stream?${params}`
    },

    /**
     * Get available log modules
     */
    async getLogModules(): Promise<string[]> {
        const response = await api.get('/observability/logs/modules')
        return response.data
    },

    /**
     * Get metrics data
     */
    async getMetrics(
        metricName: string,
        options?: {
            since?: string
            until?: string
            resolution?: '1m' | '5m' | '15m' | '1h'
        }
    ): Promise<{
        metric: string
        datapoints: MetricDataPoint[]
    }> {
        const params = new URLSearchParams()
        params.append('metric', metricName)
        if (options?.since) params.append('since', options.since)
        if (options?.until) params.append('until', options.until)
        if (options?.resolution) params.append('resolution', options.resolution)

        const response = await api.get(`/observability/metrics?${params}`)
        return response.data
    },

    /**
     * Get multiple metrics at once
     */
    async getMultipleMetrics(
        metricNames: string[],
        options?: {
            since?: string
            until?: string
            resolution?: '1m' | '5m' | '15m' | '1h'
        }
    ): Promise<Record<string, MetricDataPoint[]>> {
        const params = new URLSearchParams()
        metricNames.forEach((name) => params.append('metrics[]', name))
        if (options?.since) params.append('since', options.since)
        if (options?.until) params.append('until', options.until)
        if (options?.resolution) params.append('resolution', options.resolution)

        const response = await api.get(`/observability/metrics/batch?${params}`)
        return response.data
    },

    /**
     * Get available metric names
     */
    async getMetricNames(): Promise<Array<{
        name: string
        description: string
        unit: string
    }>> {
        const response = await api.get('/observability/metrics/available')
        return response.data
    },

    /**
     * Get current system metrics snapshot
     */
    async getCurrentMetrics(): Promise<{
        cpu_usage_percent: number
        memory_usage_bytes: number
        memory_total_bytes: number
        disk_usage_bytes: number
        disk_total_bytes: number
        active_connections: number
        queries_per_second: number
        wal_size_bytes: number
    }> {
        const response = await api.get('/observability/metrics/current')
        return response.data
    },

    /**
     * Get performance statistics
     */
    async getPerformanceStats(): Promise<{
        avg_query_duration_ms: number
        p95_query_duration_ms: number
        p99_query_duration_ms: number
        total_queries: number
        cache_hit_rate: number
        index_efficiency: number
    }> {
        const response = await api.get('/observability/performance')
        return response.data
    },

    /**
     * Export logs to file
     */
    async exportLogs(options?: {
        format?: 'json' | 'csv'
        since?: string
        until?: string
        level?: 'debug' | 'info' | 'warn' | 'error'
    }): Promise<Blob> {
        const params = new URLSearchParams()
        if (options?.format) params.append('format', options.format)
        if (options?.since) params.append('since', options.since)
        if (options?.until) params.append('until', options.until)
        if (options?.level) params.append('level', options.level)

        const response = await api.get(`/observability/logs/export?${params}`, {
            responseType: 'blob',
        })
        return response.data
    },

    // ========== Alerts ==========

    /**
     * Get all alert rules
     */
    async getAlerts(): Promise<Array<{
        id: string
        name: string
        metric: string
        condition: 'gt' | 'lt' | 'eq'
        threshold: number
        duration_minutes: number
        enabled: boolean
        notification_channels: string[]
        last_triggered_at?: string
    }>> {
        const response = await api.get('/observability/alerts')
        return response.data
    },

    /**
     * Create alert rule
     */
    async createAlert(data: {
        name: string
        metric: string
        condition: 'gt' | 'lt' | 'eq'
        threshold: number
        duration_minutes: number
        notification_channels: string[]
    }): Promise<{ id: string }> {
        const response = await api.post('/observability/alerts', data)
        return response.data
    },

    /**
     * Update alert rule
     */
    async updateAlert(alertId: string, data: {
        name?: string
        threshold?: number
        enabled?: boolean
        notification_channels?: string[]
    }): Promise<void> {
        await api.patch(`/observability/alerts/${alertId}`, data)
    },

    /**
     * Delete alert rule
     */
    async deleteAlert(alertId: string): Promise<void> {
        await api.delete(`/observability/alerts/${alertId}`)
    },

    /**
     * Get alert history
     */
    async getAlertHistory(options?: {
        limit?: number
        since?: string
    }): Promise<Array<{
        alert_id: string
        alert_name: string
        triggered_at: string
        value: number
        resolved_at?: string
    }>> {
        const params = new URLSearchParams()
        if (options?.limit) params.append('limit', options.limit.toString())
        if (options?.since) params.append('since', options.since)

        const response = await api.get(`/observability/alerts/history?${params}`)
        return response.data
    },

    // ========== Slow Queries ==========

    /**
     * Get slow queries
     */
    async getSlowQueries(options?: {
        limit?: number
        threshold_ms?: number
        since?: string
    }): Promise<Array<{
        id: string
        query: string
        duration_ms: number
        timestamp: string
        user_id?: string
        rows_examined: number
        rows_returned: number
        index_used: boolean
    }>> {
        const params = new URLSearchParams()
        if (options?.limit) params.append('limit', options.limit.toString())
        if (options?.threshold_ms) params.append('threshold_ms', options.threshold_ms.toString())
        if (options?.since) params.append('since', options.since)

        const response = await api.get(`/observability/slow-queries?${params}`)
        return response.data
    },

    /**
     * Get query execution plan
     */
    async explainQuery(query: string): Promise<{
        plan: string
        estimated_cost: number
        index_suggestions: string[]
    }> {
        const response = await api.post('/observability/explain', { query })
        return response.data
    },

    // ========== Query Profiler ==========

    /**
     * Get query profile statistics
     */
    async getQueryProfile(options?: {
        since?: string
        until?: string
    }): Promise<{
        top_queries: Array<{
            query_pattern: string
            call_count: number
            avg_duration_ms: number
            total_duration_ms: number
        }>
        query_types: Record<string, number>
        performance_trends: Array<{
            timestamp: string
            avg_duration_ms: number
            query_count: number
        }>
    }> {
        const params = new URLSearchParams()
        if (options?.since) params.append('since', options.since)
        if (options?.until) params.append('until', options.until)

        const response = await api.get(`/observability/profiler?${params}`)
        return response.data
    },

    // ========== Audit Log ==========

    /**
     * Get audit log entries
     */
    async getAuditLog(options?: {
        limit?: number
        offset?: number
        action?: string
        user_id?: string
        resource_type?: string
        since?: string
        until?: string
    }): Promise<{
        entries: Array<{
            id: string
            timestamp: string
            action: string
            user_id: string
            resource_type: string
            resource_id: string
            details: Record<string, unknown>
            ip_address?: string
        }>
        total: number
    }> {
        const params = new URLSearchParams()
        if (options?.limit) params.append('limit', options.limit.toString())
        if (options?.offset) params.append('offset', options.offset.toString())
        if (options?.action) params.append('action', options.action)
        if (options?.user_id) params.append('user_id', options.user_id)
        if (options?.resource_type) params.append('resource_type', options.resource_type)
        if (options?.since) params.append('since', options.since)
        if (options?.until) params.append('until', options.until)

        const response = await api.get(`/observability/audit?${params}`)
        return response.data
    },

    /**
     * Get available audit actions
     */
    async getAuditActions(): Promise<string[]> {
        const response = await api.get('/observability/audit/actions')
        return response.data
    },
}

