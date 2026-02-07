<script setup lang="ts">
import { ref } from 'vue'
import { useQuery } from '@tanstack/vue-query'
import AppLayout from '@/components/layout/AppLayout.vue'
import { useApi } from '@/composables/useApi'
import type { LogEntry } from '@/types'

const { api } = useApi()

const page = ref(0)
const pageSize = ref(50)
const selectedLevel = ref<string>('all')
const selectedModule = ref<string>('all')
const searchQuery = ref('')

const logLevels = ['all', 'debug', 'info', 'warn', 'error']

// Fetch logs
const { data: logsData, isLoading, error, refetch } = useQuery({
  queryKey: ['logs', page, selectedLevel, selectedModule, searchQuery],
  queryFn: async () => {
    const params: Record<string, string> = {
      limit: String(pageSize.value),
      offset: String(page.value * pageSize.value),
    }
    
    if (selectedLevel.value !== 'all') {
      params.level = selectedLevel.value
    }
    
    if (selectedModule.value !== 'all') {
      params.module = selectedModule.value
    }
    
    if (searchQuery.value) {
      params.search = searchQuery.value
    }
    
    const { data } = await api!.get('/observability/logs', { params })
    return data as { logs: LogEntry[]; total: number; modules: string[] }
  },
})

const getLevelColor = (level: string) => {
  switch (level) {
    case 'error': return 'text-red-400'
    case 'warn': return 'text-yellow-400'
    case 'info': return 'text-blue-400'
    case 'debug': return 'text-gray-400'
    default: return 'text-foreground'
  }
}

const handlePageChange = (newPage: number) => {
  page.value = newPage
}

const totalPages = () => {
  if (!logsData.value) return 1
  return Math.ceil(logsData.value.total / pageSize.value)
}

const handleRefresh = () => {
  refetch()
}

const clearFilters = () => {
  selectedLevel.value = 'all'
  selectedModule.value = 'all'
  searchQuery.value = ''
  page.value = 0
}
</script>

<template>
  <AppLayout>
    <div class="space-y-6">
      <div class="flex items-center justify-between">
        <h1 class="text-3xl font-bold">Logs</h1>
        <button
          @click="handleRefresh"
          class="px-4 py-2 rounded border border-border hover:bg-secondary"
        >
          🔄 Refresh
        </button>
      </div>

      <!-- Filters -->
      <div class="p-4 bg-card border border-border rounded-lg space-y-4">
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div>
            <label class="block text-sm font-medium mb-2">Level</label>
            <select
              v-model="selectedLevel"
              class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
            >
              <option v-for="level in logLevels" :key="level" :value="level">
                {{ level.charAt(0).toUpperCase() + level.slice(1) }}
              </option>
            </select>
          </div>
          
          <div>
            <label class="block text-sm font-medium mb-2">Module</label>
            <select
              v-model="selectedModule"
              class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
            >
              <option value="all">All Modules</option>
              <option v-for="module in logsData?.modules || []" :key="module" :value="module">
                {{ module }}
              </option>
            </select>
          </div>
          
          <div>
            <label class="block text-sm font-medium mb-2">Search</label>
            <input
              v-model="searchQuery"
              type="text"
              placeholder="Search logs..."
              class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
            />
          </div>
        </div>
        
        <button
          @click="clearFilters"
          class="px-4 py-2 rounded border border-border hover:bg-secondary text-sm"
        >
          Clear Filters
        </button>
      </div>

      <div v-if="isLoading" class="text-center py-12">
        <p class="text-muted-foreground">Loading logs...</p>
      </div>

      <div v-else-if="error" class="p-4 bg-destructive/10 text-destructive rounded-lg">
        Failed to load logs: {{ error }}
      </div>

      <div v-else-if="!logsData || !logsData.logs || logsData.logs.length === 0" class="text-center py-12">
        <p class="text-muted-foreground">No logs found</p>
      </div>

      <div v-else class="space-y-2">
        <div
          v-for="(log, idx) in logsData.logs"
          :key="idx"
          class="p-4 bg-card border border-border rounded-lg font-mono text-sm"
        >
          <div class="flex items-start gap-4">
            <span class="text-muted-foreground whitespace-nowrap">{{ log.timestamp }}</span>
            <span :class="['font-semibold whitespace-nowrap', getLevelColor(log.level)]">
              {{ log.level.toUpperCase() }}
            </span>
            <span class="text-muted-foreground whitespace-nowrap">[{{ log.module }}]</span>
            <span class="flex-1">{{ log.message }}</span>
          </div>
        </div>

        <!-- Pagination -->
        <div class="flex items-center justify-between pt-4">
          <div class="text-sm text-muted-foreground">
            Showing {{ page * pageSize + 1 }} to {{ Math.min((page + 1) * pageSize, logsData.total) }} of {{ logsData.total }} logs
          </div>
          
          <div class="flex gap-2">
            <button
              @click="handlePageChange(page - 1)"
              :disabled="page === 0"
              class="px-3 py-1 rounded border border-border hover:bg-secondary disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Previous
            </button>
            
            <span class="px-3 py-1">
              Page {{ page + 1 }} of {{ totalPages() }}
            </span>
            
            <button
              @click="handlePageChange(page + 1)"
              :disabled="page >= totalPages() - 1"
              class="px-3 py-1 rounded border border-border hover:bg-secondary disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Next
            </button>
          </div>
        </div>
      </div>
    </div>
  </AppLayout>
</template>

<style scoped>
</style>
