<script setup lang="ts">
import { ref } from 'vue'
import { useQuery, useMutation, useQueryClient } from '@tanstack/vue-query'
import { useRouter } from 'vue-router'
import AppLayout from '@/components/layout/AppLayout.vue'
import DataTable from '@/components/common/DataTable.vue'
import ConfirmDialog from '@/components/common/ConfirmDialog.vue'
import { controlPlaneService } from '@/services/controlPlane'
import type { Tenant } from '@/types'

const router = useRouter()
const queryClient = useQueryClient()

const showCreateDialog = ref(false)
const showDeleteDialog = ref(false)
const selectedTenant = ref<Tenant | null>(null)

const newTenant = ref({
  name: '',
  admin_email: '',
  plan: 'free' as 'free' | 'pro' | 'enterprise',
  isolation_model: 'schema' as 'schema' | 'database',
})

// Fetch tenants
const { data, isLoading, error } = useQuery({
  queryKey: ['tenants'],
  queryFn: async () => {
    return await controlPlaneService.listTenants()
  },
})

// Create tenant mutation
const createMutation = useMutation({
  mutationFn: async (tenantData: typeof newTenant.value) => {
    return await controlPlaneService.createTenant(tenantData)
  },
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['tenants'] })
    showCreateDialog.value = false
    resetNewTenant()
  },
})

// Delete tenant mutation
const deleteMutation = useMutation({
  mutationFn: async (tenantId: string) => {
    await controlPlaneService.deleteTenant(tenantId)
  },
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['tenants'] })
    showDeleteDialog.value = false
    selectedTenant.value = null
  },
})

// Suspend tenant mutation
const suspendMutation = useMutation({
  mutationFn: async (tenantId: string) => {
    return await controlPlaneService.suspendTenant(tenantId)
  },
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['tenants'] })
  },
})

// Activate tenant mutation
const activateMutation = useMutation({
  mutationFn: async (tenantId: string) => {
    return await controlPlaneService.activateTenant(tenantId)
  },
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['tenants'] })
  },
})

const resetNewTenant = () => {
  newTenant.value = {
    name: '',
    admin_email: '',
    plan: 'free',
    isolation_model: 'schema',
  }
}

const handleCreateTenant = () => {
  createMutation.mutate(newTenant.value)
}

const handleDeleteClick = (tenant: Tenant) => {
  selectedTenant.value = tenant
  showDeleteDialog.value = true
}

const confirmDelete = () => {
  if (selectedTenant.value) {
    deleteMutation.mutate(selectedTenant.value.id)
  }
}

const handleViewDetails = (tenant: Tenant) => {
  router.push(`/control-plane/tenants/${tenant.id}`)
}

const handleToggleStatus = (tenant: Tenant) => {
  if (tenant.status === 'active') {
    if (confirm(`Suspend tenant "${tenant.name}"? They will lose access immediately.`)) {
      suspendMutation.mutate(tenant.id)
    }
  } else if (tenant.status === 'suspended') {
    activateMutation.mutate(tenant.id)
  }
}

const formatDate = (dateString: string) => {
  return new Date(dateString).toLocaleString()
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

const columns = [
  { 
    key: 'name', 
    header: 'Name', 
    sortable: true,
    cell: (_value: unknown, row: Tenant) => {
      return `<button class="text-primary hover:underline font-medium" onclick="window.dispatchEvent(new CustomEvent('view-tenant', { detail: '${row.id}' }))">${row.name}</button>`
    }
  },
  { key: 'admin_email', header: 'Admin Email', sortable: true },
  {
    key: 'plan',
    header: 'Plan',
    sortable: true,
    cell: (value: string) => {
      const color = getPlanBadgeColor(value)
      return `<span class="px-2 py-1 rounded-full text-xs ${color}">${value.toUpperCase()}</span>`
    },
  },
  {
    key: 'status',
    header: 'Status',
    sortable: true,
    cell: (value: string) => {
      const color = getStatusBadgeColor(value)
      return `<span class="px-2 py-1 rounded-full text-xs ${color}">${value}</span>`
    },
  },
  {
    key: 'created_at',
    header: 'Created',
    sortable: true,
    cell: (value: string) => formatDate(value),
  },
  {
    key: 'actions',
    header: 'Actions',
    cell: (_value: unknown, row: Tenant) => {
      const toggleBtn =
        row.status === 'active'
          ? `<button class="text-yellow-600 hover:underline mr-3" onclick="window.dispatchEvent(new CustomEvent('toggle-status', { detail: '${row.id}' }))">Suspend</button>`
          : row.status === 'suspended'
          ? `<button class="text-green-600 hover:underline mr-3" onclick="window.dispatchEvent(new CustomEvent('toggle-status', { detail: '${row.id}' }))">Activate</button>`
          : ''
      return `${toggleBtn}<button class="text-destructive hover:underline" onclick="window.dispatchEvent(new CustomEvent('delete-tenant', { detail: '${row.id}' }))">Delete</button>`
    },
  },
]

// Handle custom events from table
if (typeof window !== 'undefined') {
  window.addEventListener('view-tenant', ((event: CustomEvent) => {
    const tenantId = event.detail
    const tenant = data.value?.tenants.find((t) => t.id === tenantId)
    if (tenant) {
      handleViewDetails(tenant)
    }
  }) as EventListener)

  window.addEventListener('delete-tenant', ((event: CustomEvent) => {
    const tenantId = event.detail
    const tenant = data.value?.tenants.find((t) => t.id === tenantId)
    if (tenant) {
      handleDeleteClick(tenant)
    }
  }) as EventListener)

  window.addEventListener('toggle-status', ((event: CustomEvent) => {
    const tenantId = event.detail
    const tenant = data.value?.tenants.find((t) => t.id === tenantId)
    if (tenant) {
      handleToggleStatus(tenant)
    }
  }) as EventListener)
}
</script>

<template>
  <AppLayout>
    <div class="space-y-6">
      <!-- Header -->
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-3xl font-bold">Tenant Management</h1>
          <p class="text-muted-foreground mt-1">
            Manage multi-tenant organizations and their resources
          </p>
        </div>
        <button
          @click="showCreateDialog = true"
          class="px-4 py-2 rounded bg-primary text-primary-foreground hover:opacity-90"
        >
          + Create Tenant
        </button>
      </div>

      <!-- Statistics Cards -->
      <div v-if="data" class="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div class="p-4 bg-card border border-border rounded-lg">
          <div class="text-sm text-muted-foreground">Total Tenants</div>
          <div class="text-2xl font-bold">{{ data.total }}</div>
        </div>
        <div class="p-4 bg-card border border-border rounded-lg">
          <div class="text-sm text-muted-foreground">Active Tenants</div>
          <div class="text-2xl font-bold text-green-600">
            {{ data.tenants.filter((t) => t.status === 'active').length }}
          </div>
        </div>
        <div class="p-4 bg-card border border-border rounded-lg">
          <div class="text-sm text-muted-foreground">Suspended</div>
          <div class="text-2xl font-bold text-yellow-600">
            {{ data.tenants.filter((t) => t.status === 'suspended').length }}
          </div>
        </div>
      </div>

      <!-- Error Display -->
      <div v-if="error" class="p-4 bg-destructive/10 text-destructive rounded-lg">
        Failed to load tenants: {{ error }}
      </div>

      <!-- Tenants Table -->
      <DataTable
        :data="(data?.tenants || []) as any"
        :columns="columns as any"
        :loading="isLoading"
      />
    </div>

    <!-- Create Tenant Dialog -->
    <div
      v-if="showCreateDialog"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      @click="showCreateDialog = false"
    >
      <div
        class="bg-card border border-border rounded-lg shadow-lg max-w-md w-full mx-4 p-6"
        @click.stop
      >
        <h2 class="text-lg font-semibold mb-4">Create New Tenant</h2>

        <div class="space-y-4">
          <div>
            <label class="block text-sm font-medium mb-2">Tenant Name *</label>
            <input
              v-model="newTenant.name"
              type="text"
              required
              placeholder="e.g., Acme Corporation"
              class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
            />
          </div>

          <div>
            <label class="block text-sm font-medium mb-2">Admin Email *</label>
            <input
              v-model="newTenant.admin_email"
              type="email"
              required
              placeholder="admin@example.com"
              class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
            />
          </div>

          <div>
            <label class="block text-sm font-medium mb-2">Plan</label>
            <select
              v-model="newTenant.plan"
              class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
            >
              <option value="free">Free</option>
              <option value="pro">Pro</option>
              <option value="enterprise">Enterprise</option>
            </select>
          </div>

          <div>
            <label class="block text-sm font-medium mb-2">Isolation Model</label>
            <select
              v-model="newTenant.isolation_model"
              class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
            >
              <option value="schema">Schema-per-Tenant (Shared DB)</option>
              <option value="database">Database-per-Tenant</option>
            </select>
            <p class="text-xs text-muted-foreground mt-1">
              Schema isolation is faster but less isolated. Database isolation provides complete separation.
            </p>
          </div>
        </div>

        <div class="flex justify-end gap-3 mt-6">
          <button
            @click="showCreateDialog = false"
            class="px-4 py-2 rounded border border-border hover:bg-secondary"
          >
            Cancel
          </button>
          <button
            @click="handleCreateTenant"
            :disabled="
              createMutation.isPending.value || !newTenant.name || !newTenant.admin_email
            "
            class="px-4 py-2 rounded bg-primary text-primary-foreground hover:opacity-90 disabled:opacity-50"
          >
            {{ createMutation.isPending.value ? 'Creating...' : 'Create Tenant' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Delete Confirmation Dialog -->
    <ConfirmDialog
      v-model:open="showDeleteDialog"
      title="Delete Tenant"
      :description="`Are you sure you want to delete ${selectedTenant?.name}? This will permanently delete all tenant data and cannot be undone.`"
      action-label="Delete Permanently"
      variant="destructive"
      @confirm="confirmDelete"
    />
  </AppLayout>
</template>

<style scoped>
</style>
