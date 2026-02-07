<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRoute } from 'vue-router'
import { useQuery, useMutation, useQueryClient } from '@tanstack/vue-query'
import AppLayout from '@/components/layout/AppLayout.vue'
import ConfirmDialog from '@/components/common/ConfirmDialog.vue'
import { useApi } from '@/composables/useApi'
import type { StorageFile } from '@/types'

const route = useRoute()
const { api } = useApi()
const queryClient = useQueryClient()

const bucketName = computed(() => route.params.name as string)
const showDeleteDialog = ref(false)
const selectedFile = ref<StorageFile | null>(null)
const uploadingFiles = ref<File[]>([])
const fileInput = ref<HTMLInputElement | null>(null)

// Fetch files in bucket
const { data: files, isLoading, error } = useQuery({
  queryKey: ['bucket-files', bucketName],
  queryFn: async () => {
    const { data } = await api!.get(`/storage/v1/buckets/${bucketName.value}/files`)
    return data as StorageFile[]
  },
  enabled: computed(() => !!bucketName.value),
})

// Upload file mutation
const uploadMutation = useMutation({
  mutationFn: async (file: File) => {
    const formData = new FormData()
    formData.append('file', file)
    const { data } = await api!.post(
      `/storage/v1/buckets/${bucketName.value}/files`,
      formData,
      {
        headers: { 'Content-Type': 'multipart/form-data' },
      }
    )
    return data
  },
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['bucket-files', bucketName.value] })
    uploadingFiles.value = []
  },
})

// Delete file mutation
const deleteMutation = useMutation({
  mutationFn: async (fileName: string) => {
    await api!.delete(`/storage/v1/buckets/${bucketName.value}/files/${fileName}`)
  },
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['bucket-files', bucketName.value] })
    showDeleteDialog.value = false
    selectedFile.value = null
  },
})

const formatFileSize = (bytes: number): string => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`
}

const handleFileSelect = (event: Event) => {
  const target = event.target as HTMLInputElement
  if (target.files) {
    uploadingFiles.value = Array.from(target.files)
    Array.from(target.files).forEach(file => {
      uploadMutation.mutate(file)
    })
  }
}

const handleDeleteClick = (file: StorageFile) => {
  selectedFile.value = file
  showDeleteDialog.value = true
}

const confirmDelete = () => {
  if (selectedFile.value) {
    deleteMutation.mutate(selectedFile.value.name)
  }
}

const downloadFile = async (fileName: string) => {
  try {
    const { data } = await api!.get(
      `/storage/v1/buckets/${bucketName.value}/files/${fileName}`,
      { responseType: 'blob' }
    )
    const url = window.URL.createObjectURL(new Blob([data]))
    const link = document.createElement('a')
    link.href = url
    link.setAttribute('download', fileName)
    document.body.appendChild(link)
    link.click()
    link.remove()
  } catch (err) {
    console.error('Download failed:', err)
  }
}
</script>

<template>
  <AppLayout>
    <div class="space-y-6">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-3xl font-bold">{{ bucketName }}</h1>
          <p class="text-sm text-muted-foreground mt-1">Storage Bucket</p>
        </div>
        <button
          @click="fileInput?.click()"
          class="px-4 py-2 rounded bg-primary text-primary-foreground hover:opacity-90"
        >
          📤 Upload Files
        </button>
        <input
          ref="fileInput"
          type="file"
          multiple
          class="hidden"
          @change="handleFileSelect"
        />
      </div>

      <div v-if="uploadMutation.isPending.value" class="p-4 bg-primary/10 text-primary rounded-lg">
        Uploading {{ uploadingFiles.length }} file(s)...
      </div>

      <div v-if="isLoading" class="text-center py-12">
        <p class="text-muted-foreground">Loading files...</p>
      </div>

      <div v-else-if="error" class="p-4 bg-destructive/10 text-destructive rounded-lg">
        Failed to load files: {{ error }}
      </div>

      <div v-else-if="!files || files.length === 0" class="text-center py-12">
        <p class="text-muted-foreground">No files in this bucket. Upload some to get started.</p>
      </div>

      <div v-else class="border border-border rounded-lg overflow-hidden">
        <table class="w-full">
          <thead class="bg-muted">
            <tr>
              <th class="px-4 py-3 text-left text-sm font-medium">Name</th>
              <th class="px-4 py-3 text-left text-sm font-medium">Size</th>
              <th class="px-4 py-3 text-left text-sm font-medium">Type</th>
              <th class="px-4 py-3 text-left text-sm font-medium">Modified</th>
              <th class="px-4 py-3 text-left text-sm font-medium">Actions</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="file in files"
              :key="file.name"
              class="border-t border-border hover:bg-secondary/50"
            >
              <td class="px-4 py-3 text-sm font-medium">{{ file.name }}</td>
              <td class="px-4 py-3 text-sm text-muted-foreground">{{ formatFileSize(file.size) }}</td>
              <td class="px-4 py-3 text-sm text-muted-foreground">{{ file.content_type }}</td>
              <td class="px-4 py-3 text-sm text-muted-foreground">
                {{ new Date(file.updated_at).toLocaleString() }}
              </td>
              <td class="px-4 py-3 text-sm">
                <div class="flex gap-2">
                  <button
                    @click="downloadFile(file.name)"
                    class="text-primary hover:underline"
                  >
                    Download
                  </button>
                  <button
                    @click="handleDeleteClick(file)"
                    class="text-destructive hover:underline"
                  >
                    Delete
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Delete Confirmation Dialog -->
    <ConfirmDialog
      v-model:open="showDeleteDialog"
      title="Delete File"
      :description="`Are you sure you want to delete ${selectedFile?.name}? This action cannot be undone.`"
      action-label="Delete"
      variant="destructive"
      @confirm="confirmDelete"
    />
  </AppLayout>
</template>

<style scoped>
</style>
