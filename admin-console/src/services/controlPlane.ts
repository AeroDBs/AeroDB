import { useApi } from '@/composables/useApi'
import type { Tenant, TenantDetails, TenantUsage, TenantQuota, Invoice } from '@/types'

const { api } = useApi()

export const controlPlaneService = {
    /**
     * List all tenants
     */
    async listTenants(params?: {
        plan?: 'free' | 'pro' | 'enterprise'
        status?: 'active' | 'suspended' | 'deleted'
        limit?: number
        offset?: number
    }): Promise<{ tenants: Tenant[]; total: number }> {
        const queryParams = new URLSearchParams()
        if (params?.plan) queryParams.append('plan', params.plan)
        if (params?.status) queryParams.append('status', params.status)
        if (params?.limit) queryParams.append('limit', params.limit.toString())
        if (params?.offset) queryParams.append('offset', params.offset.toString())

        const response = await api.get(`/control-plane/tenants?${queryParams}`)
        return response.data
    },

    /**
     * Get tenant details
     */
    async getTenant(tenantId: string): Promise<TenantDetails> {
        const response = await api.get(`/control-plane/tenants/${tenantId}`)
        return response.data
    },

    /**
     * Create a new tenant
     */
    async createTenant(data: {
        name: string
        admin_email: string
        plan: 'free' | 'pro' | 'enterprise'
        isolation_model?: 'schema' | 'database'
    }): Promise<Tenant> {
        const response = await api.post('/control-plane/tenants', data)
        return response.data
    },

    /**
     * Update tenant
     */
    async updateTenant(
        tenantId: string,
        data: {
            name?: string
            plan?: 'free' | 'pro' | 'enterprise'
            status?: 'active' | 'suspended' | 'deleted'
        }
    ): Promise<Tenant> {
        const response = await api.patch(`/control-plane/tenants/${tenantId}`, data)
        return response.data
    },

    /**
     * Delete tenant
     */
    async deleteTenant(tenantId: string): Promise<void> {
        await api.delete(`/control-plane/tenants/${tenantId}`)
    },

    /**
     * Get tenant usage
     */
    async getTenantUsage(
        tenantId: string,
        params?: {
            since?: string
            until?: string
        }
    ): Promise<TenantUsage> {
        const queryParams = new URLSearchParams()
        if (params?.since) queryParams.append('since', params.since)
        if (params?.until) queryParams.append('until', params.until)

        const response = await api.get(
            `/control-plane/tenants/${tenantId}/usage?${queryParams}`
        )
        return response.data
    },

    /**
     * Get tenant quota
     */
    async getTenantQuota(tenantId: string): Promise<{
        quota: TenantQuota
        usage: {
            api_requests: number
            storage_bytes: number
            connections: number
            databases: number
        }
    }> {
        const response = await api.get(`/control-plane/tenants/${tenantId}/quota`)
        return response.data
    },

    /**
     * Update tenant quota
     */
    async updateTenantQuota(
        tenantId: string,
        quota: Partial<TenantQuota>
    ): Promise<TenantQuota> {
        const response = await api.patch(`/control-plane/tenants/${tenantId}/quota`, quota)
        return response.data
    },

    /**
     * Get tenant invoice
     */
    async getTenantInvoice(
        tenantId: string,
        params?: {
            period_start?: string
            period_end?: string
        }
    ): Promise<Invoice> {
        const queryParams = new URLSearchParams()
        if (params?.period_start) queryParams.append('period_start', params.period_start)
        if (params?.period_end) queryParams.append('period_end', params.period_end)

        const response = await api.get(
            `/control-plane/tenants/${tenantId}/invoice?${queryParams}`
        )
        return response.data
    },

    /**
     * List invoices for a tenant
     */
    async listTenantInvoices(
        tenantId: string,
        params?: {
            status?: 'draft' | 'pending' | 'paid' | 'overdue'
            limit?: number
        }
    ): Promise<{ invoices: Invoice[]; total: number }> {
        const queryParams = new URLSearchParams()
        if (params?.status) queryParams.append('status', params.status)
        if (params?.limit) queryParams.append('limit', params.limit.toString())

        const response = await api.get(
            `/control-plane/tenants/${tenantId}/invoices?${queryParams}`
        )
        return response.data
    },

    /**
     * Suspend tenant (admin action)
     */
    async suspendTenant(tenantId: string): Promise<Tenant> {
        return this.updateTenant(tenantId, { status: 'suspended' })
    },

    /**
     * Activate tenant (admin action)
     */
    async activateTenant(tenantId: string): Promise<Tenant> {
        return this.updateTenant(tenantId, { status: 'active' })
    },
}
