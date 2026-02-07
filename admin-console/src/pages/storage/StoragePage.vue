<script setup lang="ts">
import { ref } from 'vue'
import { useQuery, useMutation, useQueryClient } from '@tanstack/vue-query'
import { useRouter } from 'vue-router'
import AppLayout from '@/components/layout/AppLayout.vue'
import { useApi } from '@/composables/useApi'
import type { Bucket } from '@/types'

const { api } = useApi()
const router = useRouter()
const queryClient = useQueryClient()

const showCreateDialog = ref(false)
const newBucketName = ref('')
const newBucketPublic = ref(false)

// Fetch buckets
const { data: buckets, isLoading, error } = useQuery({
  queryKey: ['buckets'],
  queryFn: async () => {
    const { data } = await api!.get('/storage/v1/buckets')
    return data as Bucket[]
  },
})

// Create bucket mutation
const createMutation = useMutation({
  mutationFn: async (bucketData: { name: string; public: boolean }) => {
    const { data } = await api!.post('/storage/v1/buckets', bucketData)
    return data
  },
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['buckets'] })
    showCreateDialog.value = false
    newBucketName.value = ''
    newBucketPublic.value = false
  },
})

const navigateToBucket = (bucketName: string) => {
  router.push(`/storage/bucket/${bucketName}`)
}

const handleCreateBucket = () => {
  if (!newBucketName.value.trim()) return
  createMutation.mutate({
    name: newBucketName.value,
    public: newBucketPublic.value,
  })
}
</script>

<template>
  <AppLayout>
    <div class="space-y-6">
      <div class="flex items-center justify-between">
        <h1 class="text-3xl font-bold">Storage</h1>
        <button
          @click="showCreateDialog = true"
          class="px-4 py-2 rounded bg-primary text-primary-foreground hover:opacity-90"
        >
          + Create Bucket
        </button>
      </div>

      <div v-if="isLoading" class="text-center py-12">
        <p class="text-muted-foreground">Loading buckets...</p>
      </div>

      <div v-else-if="error" class="p-4 bg-destructive/10 text-destructive rounded-lg">
        Failed to load buckets: {{ error }}
      </div>

      <div v-else-if="!buckets || buckets.length === 0" class="text-center py-12">
        <p class="text-muted-foreground">No buckets found. Create one to get started.</p>
      </div>

      <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        <div
          v-for="bucket in buckets"
          :key="bucket.name"
          @click="navigateToBucket(bucket.name)"
          class="p-6 bg-card rounded-lg border border-border hover:border-primary cursor-pointer transition-all"
        >
          <div class="flex items-start justify-between mb-2">
            <h3 class="text-lg font-semibold">{{ bucket.name }}</h3>
            <span
              :class="[
                'text-xs px-2 py-1 rounded',
                bucket.public
                  ? 'bg-green-500/20 text-green-400'
                  : 'bg-yellow-500/20 text-yellow-400'
              ]"
            >
              {{ bucket.public ? 'Public' : 'Private' }}
            </span>
          </div>
          <p class="text-sm text-muted-foreground">
            Created {{ new Date(bucket.created_at).toLocaleDateString() }}
          </p>
        </div>
      </div>
    </div>

    <!-- Create Bucket Dialog -->
    <div
      v-if="showCreateDialog"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      @click="showCreateDialog = false"
    >
      <div
        class="bg-card border border-border rounded-lg shadow-lg max-w-md w-full mx-4 p-6"
        @click.stop
      >
        <h2 class="text-lg font-semibold mb-4">Create New Bucket</h2>
        
        <div class="space-y-4">
          <div>
            <label class="block text-sm font-medium mb-2">Bucket Name *</label>
            <input
              v-model="newBucketName"
              type="text"
              required
              placeholder="my-bucket"
              class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
            />
            <p class="text-xs text-muted-foreground mt-1">
              Lowercase letters, numbers, and hyphens only
            </p>
          </div>
          
          <div class="flex items-center gap-2">
            <input
              v-model="newBucketPublic"
              type="checkbox"
              id="public-checkbox"
              class="rounded border-input"
            />
            <label for="public-checkbox" class="text-sm">
              Make bucket public
            </label>
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
            @click="handleCreateBucket"
            :disabled="createMutation.isPending.value || !newBucketName.trim()"
            class="px-4 py-2 rounded bg-primary text-primary-foreground hover:opacity-90 disabled:opacity-50"
          >
            {{ createMutation.isPending.value ? 'Creating...' : 'Create Bucket' }}
          </button>
        </div>
      </div>
    </div>
  </AppLayout>
</template>

<style scoped>
</style>
