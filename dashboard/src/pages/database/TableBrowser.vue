<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRoute } from 'vue-router'
import { useQuery } from '@tanstack/vue-query'
import AppLayout from '@/components/layout/AppLayout.vue'
import DataTable from '@/components/common/DataTable.vue'
import FilterBuilder from '@/components/common/FilterBuilder.vue'
import { useApi } from '@/composables/useApi'
import type { Filter, TableRow } from '@/types'

const route = useRoute()
const { api } = useApi()

const tableName = computed(() => route.params.name as string)
const page = ref(0)
const pageSize = ref(20)
const filters = ref<Filter[]>([])
const availableFields = ref<string[]>([])

// Fetch table schema
const { data: schema } = useQuery({
  queryKey: ['table-schema', tableName],
  queryFn: async () => {
    const { data } = await api!.get(`/rest/v1/_schema/tables/${tableName.value}`)
    availableFields.value = data.columns?.map((col: { name: string }) => col.name) || []
    return data
  },
  enabled: computed(() => !!tableName.value),
})

// Build query params from filters
const buildQueryParams = () => {
  const params: Record<string, string> = {
    limit: String(pageSize.value),
    offset: String(page.value * pageSize.value),
  }
  
  filters.value.forEach((filter, idx) => {
    if (filter.field && filter.value) {
      params[`filter[${idx}][field]`] = filter.field
      params[`filter[${idx}][op]`] = filter.operator
      params[`filter[${idx}][value]`] = String(filter.value)
    }
  })
  
  return params
}

// Fetch table data
const {
  data: tableData,
  isLoading,
  error,
  refetch,
} = useQuery({
  queryKey: ['table-data', tableName, page, filters],
  queryFn: async () => {
    const params = buildQueryParams()
    const { data } = await api!.get(`/rest/v1/${tableName.value}`, { params })
    return data as { rows: TableRow[]; total: number }
  },
  enabled: computed(() => !!tableName.value),
})

// Generate columns from first row or schema
const columns = computed(() => {
  if (!tableData.value?.rows || tableData.value.rows.length === 0) {
    return schema.value?.columns?.map((col: { name: string }) => ({
      key: col.name,
      header: col.name,
      sortable: true,
    })) || []
  }
  
  const firstRow = tableData.value.rows[0]
  return Object.keys(firstRow).map((key) => ({
    key,
    header: key,
    sortable: true,
  }))
})

const handlePageChange = (newPage: number) => {
  page.value = newPage
}

const handleFiltersChange = (newFilters: Filter[]) => {
  filters.value = newFilters
  page.value = 0 // Reset to first page when filters change
}

const handleRefresh = () => {
  refetch()
}
</script>

<template>
  <AppLayout>
    <div class="space-y-6">
      <div class="flex items-center justify-between">
        <h1 class="text-3xl font-bold">{{ tableName }}</h1>
        <button
          @click="handleRefresh"
          class="px-4 py-2 rounded border border-border hover:bg-secondary"
          title="Refresh data"
        >
          🔄 Refresh
        </button>
      </div>

      <FilterBuilder
        :available-fields="availableFields"
        @change="handleFiltersChange"
      />

      <div v-if="error" class="p-4 bg-destructive/10 text-destructive rounded-lg">
        Failed to load data: {{ error }}
      </div>

      <DataTable
        :data="tableData?.rows || []"
        :columns="columns"
        :loading="isLoading"
        :pagination="{
          total: tableData?.total || 0,
          page,
          pageSize,
          onPageChange: handlePageChange,
        }"
      />
    </div>
  </AppLayout>
</template>

<style scoped>
</style>
