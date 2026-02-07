<script setup lang="ts">
import { ref } from 'vue'
import { useQuery, useMutation, useQueryClient } from '@tanstack/vue-query'
import AppLayout from '@/components/layout/AppLayout.vue'
import ConfirmDialog from '@/components/common/ConfirmDialog.vue'
import { useApi } from '@/composables/useApi'
import type { ClusterNode } from '@/types'

const { api } = useApi()
const queryClient = useQueryClient()

const showPromoteDialog = ref(false)
const selectedNode = ref<ClusterNode | null>(null)

// Fetch cluster topology
const { data: topology, isLoading, error, refetch } = useQuery({
  queryKey: ['cluster-topology'],
  queryFn: async () => {
    const { data } = await api!.get('/control/topology')
    return data as { nodes: ClusterNode[] }
  },
})

// Promote replica mutation
const promoteMutation = useMutation({
  mutationFn: async (nodeId: string) => {
    const { data } = await api!.post('/control/promote', { node_id: nodeId })
    return data
  },
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['cluster-topology'] })
    showPromoteDialog.value = false
    selectedNode.value = null
  },
})

const handlePromoteClick = (node: ClusterNode) => {
  selectedNode.value = node
  showPromoteDialog.value = true
}

const confirmPromote = () => {
  if (selectedNode.value) {
    promoteMutation.mutate(selectedNode.value.id)
  }
}

const getStatusColor = (status: string) => {
  return status === 'online' ? 'bg-green-500/20 text-green-400' : 'bg-red-500/20 text-red-400'
}

const getRoleColor = (role: string) => {
  return role === 'authority' ? 'bg-blue-500/20 text-blue-400' : 'bg-purple-500/20 text-purple-400'
}
</script>

<template>
  <AppLayout>
    <div class="space-y-6">
      <div class="flex items-center justify-between">
        <h1 class="text-3xl font-bold">Cluster</h1>
        <button
          @click="refetch()"
          class="px-4 py-2 rounded border border-border hover:bg-secondary"
        >
          🔄 Refresh
        </button>
      </div>

      <div v-if="isLoading" class="text-center py-12">
        <p class="text-muted-foreground">Loading cluster topology...</p>
      </div>

      <div v-else-if="error" class="p-4 bg-destructive/10 text-destructive rounded-lg">
        Failed to load topology: {{ error }}
      </div>

      <div v-else-if="!topology || !topology.nodes || topology.nodes.length === 0" class="text-center py-12">
        <p class="text-muted-foreground">No nodes found</p>
      </div>

      <div v-else class="space-y-6">
        <!-- Topology Visualization -->
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <div
            v-for="node in topology.nodes"
            :key="node.id"
            class="p-6 bg-card border border-border rounded-lg"
          >
            <div class="flex items-start justify-between mb-4">
              <div>
                <h3 class="text-lg font-semibold mb-2">{{ node.id }}</h3>
                <div class="flex gap-2">
                  <span :class="['text-xs px-2 py-1 rounded', getRoleColor(node.role)]">
                    {{ node.role }}
                  </span>
                  <span :class="['text-xs px-2 py-1 rounded', getStatusColor(node.status)]">
                    {{ node.status }}
                  </span>
                </div>
              </div>
            </div>

            <div v-if="node.replication_lag !== undefined" class="space-y-2">
              <div class="text-sm text-muted-foreground">
                Replication Lag: <span class="font-mono">{{ node.replication_lag }}ms</span>
              </div>
              <div class="w-full bg-secondary rounded-full h-2">
                <div
                  :class="[
                    'h-2 rounded-full',
                    node.replication_lag < 100 ? 'bg-green-500' :
                    node.replication_lag < 500 ? 'bg-yellow-500' : 'bg-red-500'
                  ]"
                  :style="{ width: `${Math.min((node.replication_lag / 1000) * 100, 100)}%` }"
                ></div>
              </div>
            </div>

            <button
              v-if="node.role === 'replica' && node.status === 'online'"
              @click="handlePromoteClick(node)"
              class="mt-4 w-full px-4 py-2 rounded bg-primary text-primary-foreground hover:opacity-90"
            >
              Promote to Authority
            </button>
          </div>
        </div>

        <!-- Statistics -->
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div class="p-4 bg-card border border-border rounded-lg">
            <div class="text-sm text-muted-foreground mb-1">Total Nodes</div>
            <div class="text-2xl font-bold">{{ topology.nodes.length }}</div>
          </div>
          
          <div class="p-4 bg-card border border-border rounded-lg">
            <div class="text-sm text-muted-foreground mb-1">Online Nodes</div>
            <div class="text-2xl font-bold text-green-400">
              {{ topology.nodes.filter(n => n.status === 'online').length }}
            </div>
          </div>
          
          <div class="p-4 bg-card border border-border rounded-lg">
            <div class="text-sm text-muted-foreground mb-1">Replicas</div>
            <div class="text-2xl font-bold text-purple-400">
              {{ topology.nodes.filter(n => n.role === 'replica').length }}
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Promote Confirmation Dialog -->
    <ConfirmDialog
      v-model:open="showPromoteDialog"
      title="Promote Replica to Authority"
      :description="`Are you sure you want to promote ${selectedNode?.id} to authority? This will trigger a failover.`"
      action-label="Promote"
      variant="default"
      @confirm="confirmPromote"
    />
  </AppLayout>
</template>

<style scoped>
</style>
