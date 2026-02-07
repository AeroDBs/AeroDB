<script setup lang="ts">

import { useQuery } from '@tanstack/vue-query'
import { useRouter } from 'vue-router'
import AppLayout from '@/components/layout/AppLayout.vue'
import { useApi } from '@/composables/useApi'

const { api } = useApi()
const router = useRouter()

// Fetch list of collections/tables
const { data: collections, isLoading, error } = useQuery({
  queryKey: ['collections'],
  queryFn: async () => {
    const { data } = await api!.get('/rest/v1/_schema/tables')
    return data as { name: string; row_count?: number }[]
  },
})

const navigateToTable = (tableName: string) => {
  router.push(`/database/table/${tableName}`)
}
</script>

<template>
  <AppLayout>
    <div class="space-y-6">
      <div class="flex items-center justify-between">
        <h1 class="text-3xl font-bold">Database</h1>
        <button
          @click="router.push('/database/sql')"
          class="px-4 py-2 rounded bg-primary text-primary-foreground hover:opacity-90"
        >
          SQL Console
        </button>
      </div>

      <div v-if="isLoading" class="text-center py-12">
        <p class="text-muted-foreground">Loading tables...</p>
      </div>

      <div v-else-if="error" class="p-4 bg-destructive/10 text-destructive rounded-lg">
        Failed to load tables: {{ error }}
      </div>

      <div v-else-if="!collections || collections.length === 0" class="text-center py-12">
        <p class="text-muted-foreground">No tables found</p>
      </div>

      <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        <div
          v-for="table in collections"
          :key="table.name"
          @click="navigateToTable(table.name)"
          class="p-6 bg-card rounded-lg border border-border hover:border-primary cursor-pointer transition-all"
        >
          <h3 class="text-lg font-semibold mb-2">{{ table.name }}</h3>
          <p class="text-sm text-muted-foreground">
            {{ table.row_count !== undefined ? `${table.row_count} rows` : 'Table' }}
          </p>
        </div>
      </div>
    </div>
  </AppLayout>
</template>

<style scoped>
</style>
