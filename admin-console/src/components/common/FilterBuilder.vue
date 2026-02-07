<script setup lang="ts">
import { ref } from 'vue'
import type { Filter } from '@/types'

const props = defineProps<{
  availableFields?: string[]
}>()

const emit = defineEmits<{
  change: [filters: Filter[]]
}>()

const filters = ref<Filter[]>([])

const operators = [
  { value: 'eq', label: '=' },
  { value: 'gt', label: '>' },
  { value: 'lt', label: '<' },
  { value: 'gte', label: '>=' },
  { value: 'lte', label: '<=' },
  { value: 'like', label: 'LIKE' },
  { value: 'in', label: 'IN' },
]

const addFilter = () => {
  filters.value.push({
    field: '',
    operator: 'eq',
    value: '',
  })
  emitChange()
}

const removeFilter = (index: number) => {
  filters.value.splice(index, 1)
  emitChange()
}

const updateFilter = (index: number, updates: Partial<Filter>) => {
  filters.value[index] = { ...filters.value[index], ...updates }
  emitChange()
}

const emitChange = () => {
  emit('change', filters.value)
}

const clearFilters = () => {
  filters.value = []
  emitChange()
}
</script>

<template>
  <div class="space-y-3 p-4 border border-border rounded-lg bg-card">
    <div class="flex items-center justify-between">
      <h3 class="text-sm font-semibold">Filters</h3>
      <div class="flex gap-2">
        <button
          v-if="filters.length > 0"
          @click="clearFilters"
          class="px-3 py-1 text-sm rounded border border-border hover:bg-secondary"
        >
          Clear All
        </button>
        <button
          @click="addFilter"
          class="px-3 py-1 text-sm rounded bg-primary text-primary-foreground hover:opacity-90"
        >
          + Add Filter
        </button>
      </div>
    </div>

    <div v-if="filters.length === 0" class="text-sm text-muted-foreground text-center py-4">
      No filters applied. Click "Add Filter" to filter results.
    </div>

    <div v-else class="space-y-2">
      <div
        v-for="(filter, index) in filters"
        :key="index"
        class="flex gap-2 items-center"
      >
        <input
          v-model="filter.field"
          @input="updateFilter(index, { field: filter.field })"
          type="text"
          placeholder="Field name"
          class="flex-1 px-3 py-2 bg-background border border-input rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-ring"
          :list="availableFields ? `fields-${index}` : undefined"
        />
        <datalist v-if="availableFields" :id="`fields-${index}`">
          <option v-for="field in availableFields" :key="field" :value="field" />
        </datalist>

        <select
          v-model="filter.operator"
          @change="updateFilter(index, { operator: filter.operator as Filter['operator'] })"
          class="px-3 py-2 bg-background border border-input rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-ring"
        >
          <option v-for="op in operators" :key="op.value" :value="op.value">
            {{ op.label }}
          </option>
        </select>

        <input
          v-model="filter.value"
          @input="updateFilter(index, { value: filter.value })"
          type="text"
          placeholder="Value"
          class="flex-1 px-3 py-2 bg-background border border-input rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-ring"
        />

        <button
          @click="removeFilter(index)"
          class="p-2 rounded hover:bg-destructive/10 text-destructive"
          title="Remove filter"
        >
          ✕
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
</style>
