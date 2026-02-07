<script setup lang="ts">
import { ref } from 'vue'
import { useMutation } from '@tanstack/vue-query'
import AppLayout from '@/components/layout/AppLayout.vue'
import { useApi } from '@/composables/useApi'

const { api } = useApi()

const sqlQuery = ref('SELECT * FROM users LIMIT 10;')
const results = ref<{ rows: Record<string, unknown>[]; columns: string[] } | null>(null)
const error = ref<string | null>(null)
const executionTime = ref<number | null>(null)



const executeMutation = useMutation({
  mutationFn: async (query: string) => {
    const startTime = performance.now()
    const { data } = await api!.post('/rest/v1/_query', { query })
    const endTime = performance.now()
    executionTime.value = endTime - startTime
    return data
  },
  onSuccess: (data) => {
    results.value = data
    error.value = null
  },
  onError: (err: any) => {
    error.value = err.response?.data?.message || err.message || 'Query failed'
    results.value = null
  },
})

const executeQuery = () => {
  if (!sqlQuery.value.trim()) {
    error.value = 'Please enter a SQL query'
    return
  }
  executeMutation.mutate(sqlQuery.value)
}

const formatValue = (value: unknown): string => {
  if (value === null) return 'NULL'
  if (value === undefined) return 'undefined'
  if (typeof value === 'object') return JSON.stringify(value, null, 2)
  return String(value)
}


</script>

<template>
  <AppLayout>
    <div class="h-full flex flex-col space-y-4">
      <div class="flex items-center justify-between">
        <h1 class="text-3xl font-bold">SQL Console</h1>
        <button
          @click="executeQuery"
          :disabled="executeMutation.isPending.value"
          class="px-4 py-2 rounded bg-primary text-primary-foreground hover:opacity-90 disabled:opacity-50"
        >
          {{ executeMutation.isPending.value ? 'Executing...' : '▶ Execute' }}
        </button>
      </div>

      <!-- SQL Editor -->
      <div class="flex-1 min-h-[200px] border border-border rounded-lg overflow-hidden">
        <textarea
          v-model="sqlQuery"
          class="w-full h-full p-4 bg-background text-foreground font-mono text-sm resize-none focus:outline-none focus:ring-2 focus:ring-ring"
          placeholder="Enter your SQL query here..."
          @keydown.ctrl.enter="executeQuery"
          @keydown.meta.enter="executeQuery"
        ></textarea>
      </div>

      <!-- Results or Error -->
      <div class="flex-1 overflow-auto">
        <div v-if="error" class="p-4 bg-destructive/10 text-destructive rounded-lg">
          <strong>Error:</strong> {{ error }}
        </div>

        <div v-else-if="!results" class="text-center py-12 text-muted-foreground">
          Execute a query to see results here
        </div>

        <div v-else class="space-y-4">
          <div class="flex items-center justify-between px-2">
            <div class="text-sm text-muted-foreground">
              {{ results.rows.length }} rows returned
            </div>
            <div v-if="executionTime" class="text-sm text-muted-foreground">
              Executed in {{ executionTime.toFixed(2) }}ms
            </div>
          </div>

          <div class="border border-border rounded-lg overflow-hidden">
            <div class="overflow-x-auto">
              <table class="w-full">
                <thead class="bg-muted">
                  <tr>
                    <th
                      v-for="col in results.columns"
                      :key="col"
                      class="px-4 py-3 text-left text-sm font-medium"
                    >
                      {{ col }}
                    </th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="(row, idx) in results.rows"
                    :key="idx"
                    class="border-t border-border hover:bg-secondary/50"
                  >
                    <td
                      v-for="col in results.columns"
                      :key="col"
                      class="px-4 py-3 text-sm font-mono"
                    >
                      {{ formatValue(row[col]) }}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </div>

      <div class="text-xs text-muted-foreground">
        Press Ctrl+Enter (Cmd+Enter on Mac) to execute query
      </div>
    </div>
  </AppLayout>
</template>

<style scoped>
</style>
