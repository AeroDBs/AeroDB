<script setup lang="ts">
import { computed } from 'vue'
import type { TableRow } from '@/types'

interface Column<T = TableRow> {
  key: string
  header: string
  sortable?: boolean
  cell?: (value: unknown, row: T) => string
}

const props = defineProps<{
  data: TableRow[]
  columns: Column[]
  loading?: boolean
  onRowClick?: (row: TableRow) => void
  pagination?: {
    total: number
    page: number
    pageSize: number
    onPageChange: (page: number) => void
  }
}>()

const emit = defineEmits<{
  sort: [key: string, direction: 'asc' | 'desc']
}>()

const formatCellValue = (value: unknown): string => {
  if (value === null) return 'NULL'
  if (value === undefined) return '-'
  if (typeof value === 'object') return JSON.stringify(value)
  return String(value)
}

const totalPages = computed(() => {
  if (!props.pagination) return 1
  return Math.ceil(props.pagination.total / props.pagination.pageSize)
})
</script>

<template>
  <div class="space-y-4">
    <div class="border border-border rounded-lg overflow-hidden">
      <div class="overflow-x-auto">
        <table class="w-full">
          <thead class="bg-muted">
            <tr>
              <th
                v-for="col in columns"
                :key="col.key"
                class="px-4 py-3 text-left text-sm font-medium"
              >
                <button
                  v-if="col.sortable"
                  @click="emit('sort', col.key, 'asc')"
                  class="hover:text-primary"
                >
                  {{ col.header }} ↕
                </button>
                <span v-else>{{ col.header }}</span>
              </th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="loading">
              <td :colspan="columns.length" class="px-4 py-8 text-center text-muted-foreground">
                Loading...
              </td>
            </tr>
            <tr v-else-if="!data || data.length === 0">
              <td :colspan="columns.length" class="px-4 py-8 text-center text-muted-foreground">
                No data available
              </td>
            </tr>
            <tr
              v-else
              v-for="(row, idx) in data"
              :key="idx"
              class="border-t border-border hover:bg-secondary/50 cursor-pointer transition-colors"
              @click="onRowClick?.(row)"
            >
              <td
                v-for="col in columns"
                :key="col.key"
                class="px-4 py-3 text-sm"
              >
                {{ col.cell ? col.cell(row[col.key], row) : formatCellValue(row[col.key]) }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Pagination -->
    <div v-if="pagination" class="flex items-center justify-between">
      <div class="text-sm text-muted-foreground">
        Showing {{ pagination.page * pagination.pageSize + 1 }} to 
        {{ Math.min((pagination.page + 1) * pagination.pageSize, pagination.total) }} of 
        {{ pagination.total }} results
      </div>
      
      <div class="flex gap-2">
        <button
          @click="pagination.onPageChange(pagination.page - 1)"
          :disabled="pagination.page === 0"
          class="px-3 py-1 rounded border border-border hover:bg-secondary disabled:opacity-50 disabled:cursor-not-allowed"
        >
          Previous
        </button>
        
        <span class="px-3 py-1">
          Page {{ pagination.page + 1 }} of {{ totalPages }}
        </span>
        
        <button
          @click="pagination.onPageChange(pagination.page + 1)"
          :disabled="pagination.page >= totalPages - 1"
          class="px-3 py-1 rounded border border-border hover:bg-secondary disabled:opacity-50 disabled:cursor-not-allowed"
        >
          Next
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
</style>
