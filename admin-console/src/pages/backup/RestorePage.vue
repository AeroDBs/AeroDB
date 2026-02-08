<script setup lang="ts">
import { ref, computed } from 'vue'
import { useQuery, useMutation, useQueryClient } from '@tanstack/vue-query'
import AppLayout from '@/components/layout/AppLayout.vue'
import { useApi } from '@/composables/useApi'
import type { BackupInfo } from '@/types'

const { api } = useApi()
const queryClient = useQueryClient()

const selectedBackupId = ref('')
const targetDatabase = ref('')
const pointInTime = ref('')
const activeRestoreId = ref<string | null>(null)
const showWarning = ref(true)

// Fetch available backups
const { data: backups, isLoading: loadingBackups } = useQuery({
  queryKey: ['backups'],
  queryFn: async () => {
    const { data } = await api!.get('/backup/list')
    return data.backups as BackupInfo[]
  },
})

// Filter only completed backups for restore
const completedBackups = computed(() => {
  return backups.value?.filter(b => b.status === 'completed') || []
})

// Fetch restore status if there's an active restore
const { data: restoreStatus, isLoading: loadingStatus } = useQuery({
  queryKey: ['restore-status', activeRestoreId],
  queryFn: async () => {
    if (!activeRestoreId.value) return null
    const { data } = await api!.get(`/backup/restore/status/${activeRestoreId.value}`)
    return data
  },
  enabled: computed(() => !!activeRestoreId.value),
  refetchInterval: computed(() => {
    // Poll every 2 seconds if restore is in progress
    return restoreStatus.value?.status === 'in_progress' ? 2000 : false
  }),
})

// Fetch restore history
const { data: restoreHistory } = useQuery({
  queryKey: ['restore-history'],
  queryFn: async () => {
    const { data } = await api!.get('/backup/restore/history')
    return data || []
  },
})

// Restore mutation
const restoreMutation = useMutation({
  mutationFn: async (params: { backupId: string; targetDatabase?: string; pointInTime?: string }) => {
    const { data } = await api!.post(`/backup/${params.backupId}/restore`, {
      target_database: params.targetDatabase || undefined,
      point_in_time: params.pointInTime || undefined,
    })
    return data
  },
  onSuccess: (data) => {
    activeRestoreId.value = data.restore_id
    queryClient.invalidateQueries({ queryKey: ['restore-history'] })
    // Reset form
    selectedBackupId.value = ''
    targetDatabase.value = ''
    pointInTime.value = ''
    showWarning.value = false
  },
})

const handleRestore = () => {
  if (!selectedBackupId.value) {
    alert('Please select a backup to restore')
    return
  }
  
  if (!window.confirm('Are you sure you want to restore from this backup? This will overwrite existing data.')) {
    return
  }
  
  restoreMutation.mutate({
    backupId: selectedBackupId.value,
    targetDatabase: targetDatabase.value || undefined,
    pointInTime: pointInTime.value || undefined,
  })
}

const formatDate = (dateString: string) => {
  return new Date(dateString).toLocaleString()
}

const formatBytes = (bytes: number) => {
  if (bytes === 0) return '0 Bytes'
  const k = 1024
  const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i]
}

const getStatusColor = (status: string) => {
  const colors: Record<string, string> = {
    in_progress: 'text-blue-600',
    completed: 'text-green-600',
    failed: 'text-red-600',
  }
  return colors[status] || 'text-muted-foreground'
}

const getStatusIcon = (status: string) => {
  const icons: Record<string, string> = {
    in_progress: '⏳',
    completed: '✅',
    failed: '❌',
  }
  return icons[status] || '⚪'
}
</script>

<template>
  <AppLayout>
    <div class="space-y-6">
      <!-- Header -->
      <div>
        <h1 class="text-3xl font-bold">Restore from Backup</h1>
        <p class="text-muted-foreground mt-2">Restore your database from a previous backup</p>
      </div>

      <!-- Warning Banner -->
      <div v-if="showWarning" class="p-4 bg-yellow-500/10 border border-yellow-500/50 rounded-lg">
        <div class="flex items-start">
          <span class="text-2xl mr-3">⚠️</span>
          <div class="flex-1">
            <h3 class="font-semibold text-yellow-700 dark:text-yellow-400">Warning: Data Loss Risk</h3>
            <p class="text-sm text-yellow-600 dark:text-yellow-300 mt-1">
              Restoring from a backup will overwrite your current database. Make sure you have a recent backup
              before proceeding. This action cannot be undone.
            </p>
          </div>
          <button
            @click="showWarning = false"
            class="text-yellow-600 dark:text-yellow-400 hover:text-yellow-700 dark:hover:text-yellow-300"
          >
            ✕
          </button>
        </div>
      </div>

      <!-- Active Restore Status -->
      <div v-if="activeRestoreId && restoreStatus" class="p-6 bg-card border border-border rounded-lg">
        <h2 class="text-xl font-semibold mb-4">Active Restore Operation</h2>
        <div class="space-y-3">
          <div class="flex items-center justify-between">
            <span class="text-sm text-muted-foreground">Status:</span>
            <span :class="getStatusColor(restoreStatus.status)" class="font-medium">
              {{ getStatusIcon(restoreStatus.status) }} {{ restoreStatus.status }}
            </span>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-sm text-muted-foreground">Started:</span>
            <span class="text-sm">{{ formatDate(restoreStatus.started_at) }}</span>
          </div>
          <div v-if="restoreStatus.status === 'in_progress'" class="w-full bg-secondary rounded-full h-2">
            <div class="bg-primary h-2 rounded-full animate-pulse" style="width: 50%"></div>
          </div>
          <div v-if="restoreStatus.error" class="text-sm text-destructive mt-2">
            Error: {{ restoreStatus.error }}
          </div>
        </div>
      </div>

      <!-- Restore Form -->
      <div class="p-6 bg-card border border-border rounded-lg">
        <h2 class="text-xl font-semibold mb-4">Start New Restore</h2>
        
        <div class="space-y-4">
          <!-- Select Backup -->
          <div>
            <label class="block text-sm font-medium mb-2">Select Backup *</label>
            <select
              v-model="selectedBackupId"
              :disabled="loadingBackups"
              class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
            >
              <option value="">-- Select a backup --</option>
              <option
                v-for="backup in completedBackups"
                :key="backup.id"
                :value="backup.id"
              >
                {{ backup.name }} - {{ backup.backup_type }} - {{ formatDate(backup.created_at) }} ({{ formatBytes(backup.size_bytes) }})
              </option>
            </select>
            <p class="text-xs text-muted-foreground mt-1">
              {{ completedBackups.length }} completed backup(s) available
            </p>
          </div>

          <!-- Target Database (Optional) -->
          <div>
            <label class="block text-sm font-medium mb-2">Target Database (Optional)</label>
            <input
              v-model="targetDatabase"
              type="text"
              placeholder="Leave empty to restore to current database"
              class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
            />
            <p class="text-xs text-muted-foreground mt-1">
              Specify a different database name to restore to a new location
            </p>
          </div>

          <!-- Point in Time (Optional) -->
          <div>
            <label class="block text-sm font-medium mb-2">Point-in-Time Recovery (Optional)</label>
            <input
              v-model="pointInTime"
              type="datetime-local"
              class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
            />
            <p class="text-xs text-muted-foreground mt-1">
              Restore to a specific point in time (if supported by backup)
            </p>
          </div>

          <!-- Action Button -->
          <div class="flex justify-end pt-4">
            <button
              @click="handleRestore"
              :disabled="!selectedBackupId || restoreMutation.isPending.value"
              class="px-6 py-2 rounded bg-primary text-primary-foreground hover:opacity-90 disabled:opacity-50"
            >
              {{ restoreMutation.isPending.value ? 'Starting Restore...' : 'Start Restore' }}
            </button>
          </div>
        </div>
      </div>

      <!-- Restore History -->
      <div class="p-6 bg-card border border-border rounded-lg">
        <h2 class="text-xl font-semibold mb-4">Restore History</h2>
        
        <div v-if="restoreHistory && restoreHistory.length > 0" class="space-y-2">
          <div
            v-for="restore in restoreHistory"
            :key="restore.restore_id"
            class="p-3 border border-border rounded-md hover:bg-secondary/50"
          >
            <div class="flex items-center justify-between">
              <div class="flex-1">
                <div class="flex items-center gap-2">
                  <span :class="getStatusColor(restore.status)" class="font-medium">
                    {{ getStatusIcon(restore.status) }} {{ restore.status }}
                  </span>
                  <span class="text-xs text-muted-foreground">
                    Backup ID: {{ restore.backup_id.substring(0, 8) }}...
                  </span>
                </div>
                <div class="text-sm text-muted-foreground mt-1">
                  Started: {{ formatDate(restore.started_at) }}
                </div>
              </div>
            </div>
          </div>
        </div>
        
        <div v-else class="text-center text-muted-foreground py-8">
          No restore operations found
        </div>
      </div>
    </div>
  </AppLayout>
</template>

<style scoped>
</style>
