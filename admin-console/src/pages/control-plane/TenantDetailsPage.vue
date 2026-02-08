<script setup lang="ts">
import { computed } from 'vue'
import { useQuery } from '@tanstack/vue-query'
import { useRoute, useRouter } from 'vue-router'
import AppLayout from '@/components/layout/AppLayout.vue'
import MetricsChart from '@/components/common/MetricsChart.vue'
import { controlPlaneService } from '@/services/controlPlane'

const route = useRoute()
const router = useRouter()
const tenantId = computed(() => route.params.id as string)

// Fetch tenant details
const { data: tenant, isLoading } = useQuery({
  queryKey: ['tenant', tenantId],
  queryFn: async () => {
    return await controlPlaneService.getTenant(tenantId.value)
  },
})

// Fetch tenant usage
const { data: usage } = useQuery({
  queryKey: ['tenant-usage', tenantId],
  queryFn: async () => {
    return await controlPlaneService.getTenantUsage(tenantId.value)
  },
})

// Fetch tenant quota
const { data: quotaData } = useQuery({
  queryKey: ['tenant-quota', tenantId],
  queryFn: async () => {
    return await controlPlaneService.getTenantQuota(tenantId.value)
  },
})

// Fetch tenant invoices
const { data: invoicesData } = useQuery({
  queryKey: ['tenant-invoices', tenantId],
  queryFn: async () => {
    return await controlPlaneService.listTenantInvoices(tenantId.value, { limit: 10 })
  },
})

const formatBytes = (bytes: number) => {
  if (bytes === 0) return '0 Bytes'
  const k = 1024
  const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i]
}

const formatDate = (dateString: string) => {
  return new Date(dateString).toLocaleString()
}

const formatCurrency = (cents: number, currency: string = 'USD') => {
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: currency,
  }).format(cents / 100)
}

const calculateUsagePercent = (used: number, limit: number) => {
  if (limit === 0) return 0
  return Math.min((used / limit) * 100, 100)
}

const getUsageColor = (percent: number) => {
  if (percent >= 90) return 'bg-red-500'
  if (percent >= 70) return 'bg-yellow-500'
  return 'bg-green-500'
}

const getPlanBadgeColor = (plan: string) => {
  const colors: Record<string, string> = {
    free: 'bg-gray-500/10 text-gray-600 dark:text-gray-400',
    pro: 'bg-blue-500/10 text-blue-600 dark:text-blue-400',
    enterprise: 'bg-purple-500/10 text-purple-600 dark:text-purple-400',
  }
  return colors[plan] || ''
}

const getStatusBadgeColor = (status: string) => {
  const colors: Record<string, string> = {
    active: 'bg-green-500/10 text-green-600 dark:text-green-400',
    suspended: 'bg-yellow-500/10 text-yellow-600 dark:text-yellow-400',
    deleted: 'bg-red-500/10 text-red-600 dark:text-red-400',
  }
  return colors[status] || ''
}

const goBack = () => {
  router.push('/control-plane')
}
</script>

<template>
  <AppLayout>
    <div class="space-y-6">
      <!-- Header -->
      <div class="flex items-center gap-4">
        <button
          @click="goBack"
          class="px-3 py-1 rounded border border-border hover:bg-secondary"
        >
          ← Back
        </button>
        <div class="flex-1">
          <h1 class="text-3xl font-bold">{{ tenant?.name || 'Loading...' }}</h1>
          <p class="text-muted-foreground mt-1">Tenant Details</p>
        </div>
      </div>

      <!-- Loading State -->
      <div v-if="isLoading" class="text-center py-12">
        <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary mx-auto"></div>
        <p class="text-muted-foreground mt-4">Loading tenant details...</p>
      </div>

      <!-- Content -->
      <template v-else-if="tenant">
        <!-- Overview Section -->
        <div class="p-6 bg-card border border-border rounded-lg">
          <h2 class="text-xl font-semibold mb-4">Overview</h2>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <span class="text-sm text-muted-foreground">Tenant ID:</span>
              <p class="font-mono text-sm">{{ tenant.id }}</p>
            </div>
            <div>
              <span class="text-sm text-muted-foreground">Admin Email:</span>
              <p>{{ tenant.admin_email }}</p>
            </div>
            <div>
              <span class="text-sm text-muted-foreground">Plan:</span>
              <p>
                <span :class="getPlanBadgeColor(tenant.plan)" class="px-2 py-1 rounded-full text-xs">
                  {{ tenant.plan.toUpperCase() }}
                </span>
              </p>
            </div>
            <div>
              <span class="text-sm text-muted-foreground">Status:</span>
              <p>
                <span :class="getStatusBadgeColor(tenant.status)" class="px-2 py-1 rounded-full text-xs">
                  {{ tenant.status }}
                </span>
              </p>
            </div>
            <div>
              <span class="text-sm text-muted-foreground">Created:</span>
              <p class="text-sm">{{ formatDate(tenant.created_at) }}</p>
            </div>
            <div>
              <span class="text-sm text-muted-foreground">Isolation Model:</span>
              <p class="text-sm">{{ tenant.isolation_model }}</p>
            </div>
          </div>
        </div>

        <!-- Usage Metrics -->
        <div v-if="usage" class="p-6 bg-card border border-border rounded-lg">
          <h2 class="text-xl font-semibold mb-4">Usage Metrics</h2>
          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            <div class="p-4 border border-border rounded-lg">
              <div class="text-sm text-muted-foreground">API Requests</div>
              <div class="text-2xl font-bold">{{ usage.api_requests.toLocaleString() }}</div>
            </div>
            <div class="p-4 border border-border rounded-lg">
              <div class="text-sm text-muted-foreground">Storage Used</div>
              <div class="text-2xl font-bold">{{ formatBytes(usage.storage_bytes) }}</div>
            </div>
            <div class="p-4 border border-border rounded-lg">
              <div class="text-sm text-muted-foreground">Database Rows</div>
              <div class="text-2xl font-bold">{{ usage.database_rows.toLocaleString() }}</div>
            </div>
            <div class="p-4 border border-border rounded-lg">
              <div class="text-sm text-muted-foreground">Active Connections</div>
              <div class="text-2xl font-bold">{{ usage.active_connections }}</div>
            </div>
          </div>
          <p class="text-xs text-muted-foreground mt-4">
            Period: {{ formatDate(usage.period_start) }} - {{ formatDate(usage.period_end) }}
          </p>
        </div>

        <!-- Quota Management -->
        <div v-if="quotaData" class="p-6 bg-card border border-border rounded-lg">
          <h2 class="text-xl font-semibold mb-4">Quota & Limits</h2>
          <div class="space-y-4">
            <!-- API Requests Quota -->
            <div>
              <div class="flex justify-between text-sm mb-1">
                <span>API Requests / Month</span>
                <span class="text-muted-foreground">
                  {{ quotaData.usage.api_requests.toLocaleString() }} /
                  {{ quotaData.quota.api_requests_per_month.toLocaleString() }}
                </span>
              </div>
              <div class="w-full bg-secondary rounded-full h-2">
                <div
                  :class="getUsageColor(calculateUsagePercent(quotaData.usage.api_requests, quotaData.quota.api_requests_per_month))"
                  class="h-2 rounded-full transition-all"
                  :style="{ width: `${calculateUsagePercent(quotaData.usage.api_requests, quotaData.quota.api_requests_per_month)}%` }"
                ></div>
              </div>
            </div>

            <!-- Storage Quota -->
            <div>
              <div class="flex justify-between text-sm mb-1">
                <span>Storage</span>
                <span class="text-muted-foreground">
                  {{ formatBytes(quotaData.usage.storage_bytes) }} /
                  {{ formatBytes(quotaData.quota.storage_bytes) }}
                </span>
              </div>
              <div class="w-full bg-secondary rounded-full h-2">
                <div
                  :class="getUsageColor(calculateUsagePercent(quotaData.usage.storage_bytes, quotaData.quota.storage_bytes))"
                  class="h-2 rounded-full transition-all"
                  :style="{ width: `${calculateUsagePercent(quotaData.usage.storage_bytes, quotaData.quota.storage_bytes)}%` }"
                ></div>
              </div>
            </div>

            <!-- Connections -->
            <div>
              <div class="flex justify-between text-sm mb-1">
                <span>Connections</span>
                <span class="text-muted-foreground">
                  {{ quotaData.usage.connections }} / {{ quotaData.quota.max_connections }}
                </span>
              </div>
              <div class="w-full bg-secondary rounded-full h-2">
                <div
                  :class="getUsageColor(calculateUsagePercent(quotaData.usage.connections, quotaData.quota.max_connections))"
                  class="h-2 rounded-full transition-all"
                  :style="{ width: `${calculateUsagePercent(quotaData.usage.connections, quotaData.quota.max_connections)}%` }"
                ></div>
              </div>
            </div>
          </div>
        </div>

        <!-- Invoices -->
        <div v-if="invoicesData" class="p-6 bg-card border border-border rounded-lg">
          <h2 class="text-xl font-semibold mb-4">Recent Invoices</h2>
          <div v-if="invoicesData.invoices.length > 0" class="space-y-3">
            <div
              v-for="invoice in invoicesData.invoices"
              :key="invoice.id"
              class="p-4 border border-border rounded-lg flex items-center justify-between"
            >
              <div>
                <div class="font-medium">{{ formatDate(invoice.period_start) }} - {{ formatDate(invoice.period_end) }}</div>
                <div class="text-sm text-muted-foreground">
                  Status: <span :class="invoice.status === 'paid' ? 'text-green-600' : 'text-yellow-600'">{{ invoice.status }}</span>
                </div>
              </div>
              <div class="text-right">
                <div class="text-lg font-bold">{{ formatCurrency(invoice.amount_cents, invoice.currency) }}</div>
                <div class="text-xs text-muted-foreground">{{ invoice.line_items.length }} item(s)</div>
              </div>
            </div>
          </div>
          <div v-else class="text-center text-muted-foreground py-4">
            No invoices found
          </div>
        </div>
      </template>
    </div>
  </AppLayout>
</template>

<style scoped>
</style>
