import { defineStore } from 'pinia'
import { ref } from 'vue'
import { clusterService } from '@/services'
import type { ClusterNode, ReplicationStatus } from '@/types'

export const useClusterStore = defineStore('cluster', () => {
    const nodes = ref<ClusterNode[]>([])
    const replicationStatus = ref<ReplicationStatus | null>(null)
    const loading = ref(false)
    const error = ref<string | null>(null)

    const fetchNodes = async () => {
        loading.value = true
        error.value = null
        try {
            nodes.value = await clusterService.getNodes()
        } catch (err) {
            error.value = err instanceof Error ? err.message : 'Failed to fetch nodes'
        } finally {
            loading.value = false
        }
    }

    const fetchReplicationStatus = async () => {
        try {
            const statuses = await clusterService.getReplicationStatus()
            // Take first status if array, or set to null
            replicationStatus.value = Array.isArray(statuses) && statuses.length > 0 ? statuses[0] : null
        } catch (err) {
            error.value = err instanceof Error ? err.message : 'Failed to fetch replication status'
        }
    }

    const promoteReplica = async (nodeId: string) => {
        return await clusterService.promoteReplica(nodeId)
    }

    const addReplica = async (host: string, port: number, mode: 'sync' | 'async' = 'async') => {
        const node = await clusterService.addReplica({ host, port, replication_mode: mode })
        nodes.value.push(node)
        return node
    }

    const removeNode = async (nodeId: string) => {
        // Remove from local state - API may not have this method yet
        nodes.value = nodes.value.filter((n) => n.id !== nodeId)
    }

    return {
        nodes,
        replicationStatus,
        loading,
        error,
        fetchNodes,
        fetchReplicationStatus,
        promoteReplica,
        addReplica,
        removeNode,
    }
})
