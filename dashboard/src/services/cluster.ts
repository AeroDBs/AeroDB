import { useApi } from '@/composables/useApi'
import type { ClusterNode, PromotionRequest, PromotionState, ReplicationStatus } from '@/types'

const { api } = useApi()

export const clusterService = {
    /**
     * Get all cluster nodes
     */
    async getNodes(): Promise<ClusterNode[]> {
        const response = await api.get('/cluster/nodes')
        return response.data
    },

    /**
     * Get a specific node
     */
    async getNode(nodeId: string): Promise<ClusterNode> {
        const response = await api.get(`/cluster/nodes/${nodeId}`)
        return response.data
    },

    /**
     * Get cluster topology
     */
    async getTopology(): Promise<{
        authority: ClusterNode | null
        replicas: ClusterNode[]
    }> {
        const response = await api.get('/cluster/topology')
        return response.data
    },

    /**
     * Get replication status for all nodes
     */
    async getReplicationStatus(): Promise<ReplicationStatus[]> {
        const response = await api.get('/cluster/replication/status')
        return response.data
    },

    /**
     * Get replication status for a specific node
     */
    async getNodeReplicationStatus(nodeId: string): Promise<ReplicationStatus> {
        const response = await api.get(`/cluster/nodes/${nodeId}/replication`)
        return response.data
    },

    /**
     * Promote a replica to authority
     */
    async promoteReplica(nodeId: string): Promise<PromotionRequest> {
        const response = await api.post('/cluster/promote', { node_id: nodeId })
        return response.data
    },

    /**
     * Get current promotion state
     */
    async getPromotionState(): Promise<PromotionState | null> {
        const response = await api.get('/cluster/promotion/state')
        return response.data
    },

    /**
     * Get authority marker status
     */
    async getAuthorityMarker(): Promise<{
        has_marker: boolean
        node_id?: string
        timestamp?: string
    }> {
        const response = await api.get('/cluster/authority/marker')
        return response.data
    },

    /**
     * Add a new replica node
     */
    async addReplica(nodeConfig: {
        host: string
        port: number
        replication_mode?: 'sync' | 'async'
    }): Promise<ClusterNode> {
        const response = await api.post('/cluster/replicas', nodeConfig)
        return response.data
    },

    /**
     * Remove a replica node
     */
    async removeReplica(nodeId: string): Promise<void> {
        await api.delete(`/cluster/replicas/${nodeId}`)
    },

    /**
     * Get cluster health
     */
    async getClusterHealth(): Promise<{
        status: 'healthy' | 'degraded' | 'unhealthy'
        authority_online: boolean
        replicas_online: number
        replicas_total: number
        max_replication_lag_ms: number
        issues: string[]
    }> {
        const response = await api.get('/cluster/health')
        return response.data
    },

    /**
     * Get cluster statistics
     */
    async getClusterStats(): Promise<{
        total_nodes: number
        online_nodes: number
        replication_mode: 'sync' | 'async'
        avg_replication_lag_ms: number
        wal_positions: Record<string, number>
    }> {
        const response = await api.get('/cluster/stats')
        return response.data
    },

    /**
     * Pause replication for a node
     */
    async pauseReplication(nodeId: string): Promise<void> {
        await api.post(`/cluster/nodes/${nodeId}/replication/pause`)
    },

    /**
     * Resume replication for a node
     */
    async resumeReplication(nodeId: string): Promise<void> {
        await api.post(`/cluster/nodes/${nodeId}/replication/resume`)
    },
}
