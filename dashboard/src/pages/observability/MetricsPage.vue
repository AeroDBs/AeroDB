<script setup lang="ts">
import { ref, computed } from 'vue'
import { useQuery } from '@tanstack/vue-query'
import AppLayout from '@/components/layout/AppLayout.vue'
import { useApi } from '@/composables/useApi'
import type { MetricDataPoint } from '@/types'

const { api } = useApi()

const timeRange = ref('1h')
const autoRefresh = ref(false)

const timeRanges = [
  { value: '5m', label: 'Last 5 minutes' },
  { value: '15m', label: 'Last 15 minutes' },
  { value: '1h', label: 'Last hour' },
  { value: '6h', label: 'Last 6 hours' },
  { value: '24h', label: 'Last 24 hours' },
]

// Fetch metrics
const { data: metrics, isLoading, error, refetch } = useQuery({
  queryKey: ['metrics', timeRange],
  queryFn: async () => {
    const { data } = await api!.get('/observability/metrics', {
      params: { timeRange: timeRange.value }
    })
    return data as {
      queries_per_sec: MetricDataPoint[]
      latency_p95: MetricDataPoint[]
      latency_p99: MetricDataPoint[]
      error_rate: MetricDataPoint[]
      active_connections: MetricDataPoint[]
    }
  },
  refetchInterval: computed(() => autoRefresh.value ? 5000 : false),
})

const formatValue = (value: number, decimals = 2) => {
  return value.toFixed(decimals)
}

const getLatestValue = (dataPoints?: MetricDataPoint[]) => {
  if (!dataPoints || dataPoints.length === 0) return '—'
  return formatValue(dataPoints[dataPoints.length - 1].value)
}

const getAverageValue = (dataPoints?: MetricDataPoint[]) => {
  if (!dataPoints || dataPoints.length === 0) return '—'
  const sum = dataPoints.reduce((acc, point) => acc + point.value, 0)
  return formatValue(sum / dataPoints.length)
}
</script>

<template>
  <AppLayout>
    <div class="space-y-6">
      <div class="flex items-center justify-between">
        <h1 class="text-3xl font-bold">Metrics</h1>
        <div class="flex items-center gap-4">
          <select
            v-model="timeRange"
            class="px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
          >
            <option v-for="range in timeRanges" :key="range.value" :value="range.value">
              {{ range.label }}
            </option>
          </select>
          
          <label class="flex items-center gap-2">
            <input
              v-model="autoRefresh"
              type="checkbox"
              class="rounded border-input"
            />
            <span class="text-sm">Auto-refresh (5s)</span>
          </label>
          
          <button
            @click="refetch()"
            class="px-4 py-2 rounded border border-border hover:bg-secondary"
          >
            🔄 Refresh
          </button>
        </div>
      </div>

      <div v-if="isLoading" class="text-center py-12">
        <p class="text-muted-foreground">Loading metrics...</p>
      </div>

      <div v-else-if="error" class="p-4 bg-destructive/10 text-destructive rounded-lg">
        Failed to load metrics: {{ error }}
      </div>

      <div v-else-if="!metrics" class="text-center py-12">
        <p class="text-muted-foreground">No metrics available</p>
      </div>

      <div v-else class="space-y-6">
        <!-- Summary Cards -->
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          <div class="p-6 bg-card border border-border rounded-lg">
            <div class="text-sm text-muted-foreground mb-1">Queries/sec</div>
            <div class="text-3xl font-bold">{{ getLatestValue(metrics.queries_per_sec) }}</div>
            <div class="text-xs text-muted-foreground mt-2">
              Avg: {{ getAverageValue(metrics.queries_per_sec) }}
            </div>
          </div>
          
          <div class="p-6 bg-card border border-border rounded-lg">
            <div class="text-sm text-muted-foreground mb-1">Latency (p95)</div>
            <div class="text-3xl font-bold">{{ getLatestValue(metrics.latency_p95) }} <span class="text-base">ms</span></div>
            <div class="text-xs text-muted-foreground mt-2">
              Avg: {{ getAverageValue(metrics.latency_p95) }} ms
            </div>
          </div>
          
          <div class="p-6 bg-card border border-border rounded-lg">
            <div class="text-sm text-muted-foreground mb-1">Latency (p99)</div>
            <div class="text-3xl font-bold">{{ getLatestValue(metrics.latency_p99) }} <span class="text-base">ms</span></div>
            <div class="text-xs text-muted-foreground mt-2">
              Avg: {{ getAverageValue(metrics.latency_p99) }} ms
            </div>
          </div>
          
          <div class="p-6 bg-card border border-border rounded-lg">
            <div class="text-sm text-muted-foreground mb-1">Error Rate</div>
            <div class="text-3xl font-bold text-destructive">{{ getLatestValue(metrics.error_rate) }}<span class="text-base">%</span></div>
            <div class="text-xs text-muted-foreground mt-2">
              Avg: {{ getAverageValue(metrics.error_rate) }}%
            </div>
          </div>
        </div>

        <!-- Simple Data Visualization -->
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <div class="p-6 bg-card border border-border rounded-lg">
            <h3 class="text-lg font-semibold mb-4">Queries Per Second</h3>
            <div class="space-y-2">
              <div
                v-for="(point, idx) in (metrics.queries_per_sec || []).slice(-10)"
                :key="idx"
                class="flex items-center gap-4"
              >
                <span class="text-xs text-muted-foreground w-24">{{ new Date(point.timestamp).toLocaleTimeString() }}</span>
                <div class="flex-1 bg-secondary rounded-full h-4">
                  <div
                    class="bg-primary h-4 rounded-full"
                    :style="{ width: `${(point.value / Math.max(...(metrics.queries_per_sec || []).map(p => p.value))) * 100}%` }"
                  ></div>
                </div>
                <span class="text-sm font-mono w-16 text-right">{{ formatValue(point.value) }}</span>
              </div>
            </div>
          </div>

          <div class="p-6 bg-card border border-border rounded-lg">
            <h3 class="text-lg font-semibold mb-4">Active Connections</h3>
            <div class="space-y-2">
              <div
                v-for="(point, idx) in (metrics.active_connections || []).slice(-10)"
                :key="idx"
                class="flex items-center gap-4"
              >
                <span class="text-xs text-muted-foreground w-24">{{ new Date(point.timestamp).toLocaleTimeString() }}</span>
                <div class="flex-1 bg-secondary rounded-full h-4">
                  <div
                    class="bg-green-500 h-4 rounded-full"
                    :style="{ width: `${(point.value / Math.max(...(metrics.active_connections || []).map(p => p.value))) * 100}%` }"
                  ></div>
                </div>
                <span class="text-sm font-mono w-16 text-right">{{ formatValue(point.value, 0) }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </AppLayout>
</template>

<style scoped>
</style>
