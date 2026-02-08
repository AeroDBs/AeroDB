<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useQuery, useMutation, useQueryClient } from '@tanstack/vue-query'
import AppLayout from '@/components/layout/AppLayout.vue'
import DataTable from '@/components/common/DataTable.vue'
import ConfirmDialog from '@/components/common/ConfirmDialog.vue'
import { useApi } from '@/composables/useApi'
import type { BackupInfo, BackupSchedule, BackupStats } from '@/types'

const { api } = useApi()
const queryClient = useQueryClient()

const showCreateDialog = ref(false)
const showDeleteDialog = ref(false)
const showScheduleDialog = ref(false)
const selectedBackup = ref<BackupInfo | null>(null)

const newBackup = ref({
  name: '',
  backup_type: 'full',
  include_tables: [] as string[],
})

const scheduleForm = ref<BackupSchedule>({
  enabled: false,
  cron_expression: '0 0 * * *',
  retention_days: 30,
  backup_type: 'incremental',
})

// Fetch backups list
const { data: backups, isLoading, error } = useQuery({
  queryKey: ['backups'],
  queryFn: async () => {
    const { data } = await api!.get('/backup/list')
    return data.backups as BackupInfo[]
  },
  refetchInterval: 5000, // Refresh every 5s to update status
})

// Fetch backup statistics
const { data: stats } = useQuery({
  queryKey: ['backup-stats'],
  queryFn: async () => {
    const { data } = await api!.get('/backup/stats')
    return data as BackupStats
  },
})

// Fetch backup schedule
const { data: schedule } = useQuery({
  queryKey: ['backup-schedule'],
  queryFn: async () => {
    const { data } = await api!.get('/backup/schedule')
    return data as BackupSchedule
  },
})

// Load schedule into form when fetched
onMounted(() => {
  if (schedule.value) {
    scheduleForm.value = { ...schedule.value }
  }
})

// Create backup mutation
const createMutation = useMutation({
  mutationFn: async (backupData: typeof newBackup.value) => {
    const { data } = await api!.post('/backup/create', backupData)
    return data
  },
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['backups'] })
    queryClient.invalidateQueries({ queryKey: ['backup-stats'] })
    showCreateDialog.value = false
    resetNewBackup()
  },
})

// Delete backup mutation
const deleteMutation = useMutation({
  mutationFn: async (backupId: string) => {
    await api!.delete(`/backup/${backupId}`)
  },
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['backups'] })
    queryClient.invalidateQueries({ queryKey: ['backup-stats'] })
    showDeleteDialog.value = false
    selectedBackup.value = null
  },
})

// Update schedule mutation
const updateScheduleMutation = useMutation({
  mutationFn: async (scheduleData: BackupSchedule) => {
    const { data } = await api!.patch('/backup/schedule', scheduleData)
    return data
  },
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['backup-schedule'] })
    showScheduleDialog.value = false
  },
})

const resetNewBackup = () => {
  newBackup.value = {
    name: '',
    backup_type: 'full',
    include_tables: [],
  }
}

const handleCreateBackup = () => {
  createMutation.mutate(newBackup.value)
}

const handleDeleteClick = (backup: BackupInfo) => {
  selectedBackup.value = backup
  showDeleteDialog.value = true
}

const confirmDelete = () => {
  if (selectedBackup.value) {
    deleteMutation.mutate(selectedBackup.value.id)
  }
}

const handleDownload = async (backup: BackupInfo) => {
  try {
    const { data } = await api!.get(`/backup/${backup.id}/download`, {
      responseType: 'blob',
    })
    const url = window.URL.createObjectURL(new Blob([data]))
    const link = document.createElement('a')
    link.href = url
    link.setAttribute('download', `backup-${backup.name}-${backup.id}.tar.gz`)
    document.body.appendChild(link)
    link.click()
    link.remove()
  } catch (err) {
    console.error('Download failed:', err)
  }
}

const openScheduleDialog = () => {
  if (schedule.value) {
    scheduleForm.value = { ...schedule.value }
  }
  showScheduleDialog.value = true
}

const handleUpdateSchedule = () => {
  updateScheduleMutation.mutate(scheduleForm.value)
}

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

const columns = [
  { key: 'name', header: 'Name', sortable: true },
  { key: 'backup_type', header: 'Type', sortable: true },
  { 
    key: 'size_bytes', 
    header: 'Size', 
    sortable: true,
    cell: (value: number) => formatBytes(value)
  },
  { 
    key: 'status', 
    header: 'Status', 
    sortable: true,
    cell: (value: string) => {
      const colors: Record<string, string> = {
        completed: 'text-green-600',
        in_progress: 'text-blue-600',
        failed: 'text-red-600',
      }
      return `<span class="${colors[value] || ''}">${value}</span>`
    }
  },
  { 
    key: 'created_at', 
    header: 'Created', 
    sortable: true,
    cell: (value: string) => formatDate(value)
  },
  {
    key: 'actions',
    header: 'Actions',
    cell: (_value: unknown, row: BackupInfo) => {
      const downloadBtn = row.status === 'completed' 
        ? `<button class="text-primary hover:underline mr-3" onclick="window.dispatchEvent(new CustomEvent('download-backup', { detail: '${row.id}' }))">Download</button>`
        : ''
      return `${downloadBtn}<button class="text-destructive hover:underline" onclick="window.dispatchEvent(new CustomEvent('delete-backup', { detail: '${row.id}' }))">Delete</button>`
    },
  },
]

// Handle custom events from table
if (typeof window !== 'undefined') {
  window.addEventListener('delete-backup', ((event: CustomEvent) => {
    const backupId = event.detail
    const backup = backups.value?.find(b => b.id === backupId)
    if (backup) {
      handleDeleteClick(backup)
    }
  }) as EventListener)

  window.addEventListener('download-backup', ((event: CustomEvent) => {
    const backupId = event.detail
    const backup = backups.value?.find(b => b.id === backupId)
    if (backup) {
      handleDownload(backup)
    }
  }) as EventListener)
}
</script>

<template>
  <AppLayout>
    <div class="space-y-6">
      <!-- Header -->
      <div class="flex items-center justify-between">
        <h1 class="text-3xl font-bold">Backups</h1>
        <button
          @click="showCreateDialog = true"
          class="px-4 py-2 rounded bg-primary text-primary-foreground hover:opacity-90"
        >
          + Create Backup
        </button>
      </div>

      <!-- Statistics Cards -->
      <div v-if="stats" class="grid grid-cols-1 md:grid-cols-4 gap-4">
        <div class="p-4 bg-card border border-border rounded-lg">
          <div class="text-sm text-muted-foreground">Total Backups</div>
          <div class="text-2xl font-bold">{{ stats.total_backups }}</div>
        </div>
        <div class="p-4 bg-card border border-border rounded-lg">
          <div class="text-sm text-muted-foreground">Total Size</div>
          <div class="text-2xl font-bold">{{ formatBytes(stats.total_size_bytes) }}</div>
        </div>
        <div class="p-4 bg-card border border-border rounded-lg">
          <div class="text-sm text-muted-foreground">Last Backup</div>
          <div class="text-2xl font-bold">{{ stats.last_backup_at ? formatDate(stats.last_backup_at) : 'Never' }}</div>
        </div>
        <div class="p-4 bg-card border border-border rounded-lg">
          <div class="text-sm text-muted-foreground">Failed (24h)</div>
          <div class="text-2xl font-bold text-destructive">{{ stats.failed_backups_24h }}</div>
        </div>
      </div>

      <!-- Backup Schedule Card -->
      <div v-if="schedule" class="p-6 bg-card border border-border rounded-lg">
        <div class="flex items-center justify-between mb-4">
          <h2 class="text-xl font-semibold">Backup Schedule</h2>
          <button
            @click="openScheduleDialog"
            class="px-3 py-1 text-sm rounded border border-border hover:bg-secondary"
          >
            Edit Schedule
          </button>
        </div>
        <div class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
          <div>
            <span class="text-muted-foreground">Status:</span>
            <span :class="schedule.enabled ? 'text-green-600 ml-2' : 'text-muted-foreground ml-2'">
              {{ schedule.enabled ? 'Enabled' : 'Disabled' }}
            </span>
          </div>
          <div>
            <span class="text-muted-foreground">Schedule:</span>
            <span class="ml-2">{{ schedule.cron_expression }}</span>
          </div>
          <div>
            <span class="text-muted-foreground">Retention:</span>
            <span class="ml-2">{{ schedule.retention_days }} days</span>
          </div>
          <div>
            <span class="text-muted-foreground">Type:</span>
            <span class="ml-2">{{ schedule.backup_type }}</span>
          </div>
        </div>
      </div>

      <!-- Error Display -->
      <div v-if="error" class="p-4 bg-destructive/10 text-destructive rounded-lg">
        Failed to load backups: {{ error }}
      </div>

      <!-- Backups Table -->
      <DataTable
        :data="(backups || []) as any"
        :columns="columns as any"
        :loading="isLoading"
      />
    </div>

    <!-- Create Backup Dialog -->
    <div
      v-if="showCreateDialog"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      @click="showCreateDialog = false"
    >
      <div
        class="bg-card border border-border rounded-lg shadow-lg max-w-md w-full mx-4 p-6"
        @click.stop
      >
        <h2 class="text-lg font-semibold mb-4">Create New Backup</h2>
        
        <div class="space-y-4">
          <div>
            <label class="block text-sm font-medium mb-2">Backup Name *</label>
            <input
              v-model="newBackup.name"
              type="text"
              required
              placeholder="e.g., daily-backup-2024"
              class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
            />
          </div>
          
          <div>
            <label class="block text-sm font-medium mb-2">Backup Type</label>
            <select
              v-model="newBackup.backup_type"
              class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
            >
              <option value="full">Full Backup</option>
              <option value="incremental">Incremental</option>
            </select>
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
            @click="handleCreateBackup"
            :disabled="createMutation.isPending.value || !newBackup.name"
            class="px-4 py-2 rounded bg-primary text-primary-foreground hover:opacity-90 disabled:opacity-50"
          >
            {{ createMutation.isPending.value ? 'Creating...' : 'Create Backup' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Schedule Dialog -->
    <div
      v-if="showScheduleDialog"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      @click="showScheduleDialog = false"
    >
      <div
        class="bg-card border border-border rounded-lg shadow-lg max-w-md w-full mx-4 p-6"
        @click.stop
      >
        <h2 class="text-lg font-semibold mb-4">Edit Backup Schedule</h2>
        
        <div class="space-y-4">
          <div class="flex items-center">
            <input
              v-model="scheduleForm.enabled"
              type="checkbox"
              id="schedule-enabled"
              class="mr-2"
            />
            <label for="schedule-enabled" class="text-sm font-medium">Enable Automatic Backups</label>
          </div>
          
          <div>
            <label class="block text-sm font-medium mb-2">Cron Expression</label>
            <input
              v-model="scheduleForm.cron_expression"
              type="text"
              placeholder="0 0 * * * (Daily at midnight)"
              class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
            />
            <p class="text-xs text-muted-foreground mt-1">Example: 0 0 * * * = Daily at midnight</p>
          </div>
          
          <div>
            <label class="block text-sm font-medium mb-2">Retention Days</label>
            <input
              v-model.number="scheduleForm.retention_days"
              type="number"
              min="1"
              max="365"
              class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
            />
          </div>
          
          <div>
            <label class="block text-sm font-medium mb-2">Backup Type</label>
            <select
              v-model="scheduleForm.backup_type"
              class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
            >
              <option value="full">Full</option>
              <option value="incremental">Incremental</option>
            </select>
          </div>
        </div>
        
        <div class="flex justify-end gap-3 mt-6">
          <button
            @click="showScheduleDialog = false"
            class="px-4 py-2 rounded border border-border hover:bg-secondary"
          >
            Cancel
          </button>
          <button
            @click="handleUpdateSchedule"
            :disabled="updateScheduleMutation.isPending.value"
            class="px-4 py-2 rounded bg-primary text-primary-foreground hover:opacity-90 disabled:opacity-50"
          >
            {{ updateScheduleMutation.isPending.value ? 'Saving...' : 'Save Schedule' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Delete Confirmation Dialog -->
    <ConfirmDialog
      v-model:open="showDeleteDialog"
      title="Delete Backup"
      :description="`Are you sure you want to delete ${selectedBackup?.name}? This action cannot be undone.`"
      action-label="Delete"
      variant="destructive"
      @confirm="confirmDelete"
    />
  </AppLayout>
</template>

<style scoped>
</style>
