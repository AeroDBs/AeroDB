<script setup lang="ts">
import { ref } from 'vue'
import { useQuery, useMutation, useQueryClient } from '@tanstack/vue-query'
import AppLayout from '@/components/layout/AppLayout.vue'
import DataTable from '@/components/common/DataTable.vue'
import ConfirmDialog from '@/components/common/ConfirmDialog.vue'
import { useApi } from '@/composables/useApi'
import type { User } from '@/types'

const { api } = useApi()
const queryClient = useQueryClient()

const showAddDialog = ref(false)
const showDeleteDialog = ref(false)
const selectedUser = ref<User | null>(null)

const newUser = ref({
  email: '',
  password: '',
  name: '',
  role: 'user',
})

// Fetch users
const { data: users, isLoading, error } = useQuery({
  queryKey: ['users'],
  queryFn: async () => {
    const { data } = await api!.get('/auth/users')
    return data as User[]
  },
})

// Create user mutation
const createMutation = useMutation({
  mutationFn: async (userData: typeof newUser.value) => {
    const { data } = await api!.post('/auth/users', userData)
    return data
  },
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['users'] })
    showAddDialog.value = false
    resetNewUser()
  },
})

// Delete user mutation
const deleteMutation = useMutation({
  mutationFn: async (userId: string) => {
    await api!.delete(`/auth/users/${userId}`)
  },
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['users'] })
    showDeleteDialog.value = false
    selectedUser.value = null
  },
})

const resetNewUser = () => {
  newUser.value = {
    email: '',
    password: '',
    name: '',
    role: 'user',
  }
}

const handleAddUser = () => {
  createMutation.mutate(newUser.value)
}

const handleDeleteClick = (user: User) => {
  selectedUser.value = user
  showDeleteDialog.value = true
}

const confirmDelete = () => {
  if (selectedUser.value) {
    deleteMutation.mutate(selectedUser.value.id)
  }
}

const columns = [
  { key: 'email', header: 'Email', sortable: true },
  { key: 'name', header: 'Name', sortable: true },
  { key: 'role', header: 'Role', sortable: true },
  { key: 'created_at', header: 'Created', sortable: true },
  { key: 'last_login', header: 'Last Login', sortable: true },
  {
    key: 'actions',
    header: 'Actions',
    cell: (_value: unknown, row: User) => {
      return `<button class="text-destructive hover:underline" onclick="window.dispatchEvent(new CustomEvent('delete-user', { detail: '${row.id}' }))">Delete</button>`
    },
  },
]

// Handle delete button clicks from table
if (typeof window !== 'undefined') {
  window.addEventListener('delete-user', ((event: CustomEvent) => {
    const userId = event.detail
    const user = users.value?.find(u => u.id === userId)
    if (user) {
      handleDeleteClick(user)
    }
  }) as EventListener)
}
</script>

<template>
  <AppLayout>
    <div class="space-y-6">
      <div class="flex items-center justify-between">
        <h1 class="text-3xl font-bold">Users</h1>
        <button
          @click="showAddDialog = true"
          class="px-4 py-2 rounded bg-primary text-primary-foreground hover:opacity-90"
        >
          + Add User
        </button>
      </div>

      <div v-if="error" class="p-4 bg-destructive/10 text-destructive rounded-lg">
        Failed to load users: {{ error }}
      </div>

      <DataTable
        :data="(users || []) as any"
        :columns="columns as any"
        :loading="isLoading"
      />
    </div>

    <!-- Add User Dialog -->
    <div
      v-if="showAddDialog"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      @click="showAddDialog = false"
    >
      <div
        class="bg-card border border-border rounded-lg shadow-lg max-w-md w-full mx-4 p-6"
        @click.stop
      >
        <h2 class="text-lg font-semibold mb-4">Add New User</h2>
        
        <div class="space-y-4">
          <div>
            <label class="block text-sm font-medium mb-2">Email *</label>
            <input
              v-model="newUser.email"
              type="email"
              required
              class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
            />
          </div>
          
          <div>
            <label class="block text-sm font-medium mb-2">Password *</label>
            <input
              v-model="newUser.password"
              type="password"
              required
              class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
            />
          </div>
          
          <div>
            <label class="block text-sm font-medium mb-2">Name</label>
            <input
              v-model="newUser.name"
              type="text"
              class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
            />
          </div>
          
          <div>
            <label class="block text-sm font-medium mb-2">Role</label>
            <select
              v-model="newUser.role"
              class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
            >
              <option value="user">User</option>
              <option value="admin">Admin</option>
            </select>
          </div>
        </div>
        
        <div class="flex justify-end gap-3 mt-6">
          <button
            @click="showAddDialog = false"
            class="px-4 py-2 rounded border border-border hover:bg-secondary"
          >
            Cancel
          </button>
          <button
            @click="handleAddUser"
            :disabled="createMutation.isPending.value"
            class="px-4 py-2 rounded bg-primary text-primary-foreground hover:opacity-90 disabled:opacity-50"
          >
            {{ createMutation.isPending.value ? 'Creating...' : 'Create User' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Delete Confirmation Dialog -->
    <ConfirmDialog
      v-model:open="showDeleteDialog"
      title="Delete User"
      :description="`Are you sure you want to delete ${selectedUser?.email}? This action cannot be undone.`"
      action-label="Delete"
      variant="destructive"
      @confirm="confirmDelete"
    />
  </AppLayout>
</template>

<style scoped>
</style>
