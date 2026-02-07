<script setup lang="ts">
import { ref, computed } from 'vue'

const props = defineProps<{
    bucketName: string
    folderPath?: string
}>()

const emit = defineEmits<{
    (e: 'upload-complete', files: File[]): void
    (e: 'upload-error', error: string): void
}>()

const isDragging = ref(false)
const selectedFiles = ref<File[]>([])
const isUploading = ref(false)
const uploadProgress = ref({ current: 0, total: 0 })

const fileInput = ref<HTMLInputElement | null>(null)

const totalSize = computed(() => {
    return selectedFiles.value.reduce((sum, file) => sum + file.size, 0)
})

const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`
}

const handleDragEnter = (e: DragEvent) => {
    e.preventDefault()
    isDragging.value = true
}

const handleDragLeave = (e: DragEvent) => {
    e.preventDefault()
    isDragging.value = false
}

const handleDragOver = (e: DragEvent) => {
    e.preventDefault()
}

const handleDrop = (e: DragEvent) => {
    e.preventDefault()
    isDragging.value = false
    
    if (e.dataTransfer?.files) {
        addFiles(Array.from(e.dataTransfer.files))
    }
}

const handleFileSelect = (e: Event) => {
    const input = e.target as HTMLInputElement
    if (input.files) {
        addFiles(Array.from(input.files))
    }
}

const addFiles = (files: File[]) => {
    // Filter duplicates
    const newFiles = files.filter(
        file => !selectedFiles.value.some(existing => existing.name === file.name)
    )
    selectedFiles.value = [...selectedFiles.value, ...newFiles]
}

const removeFile = (index: number) => {
    selectedFiles.value.splice(index, 1)
}

const clearFiles = () => {
    selectedFiles.value = []
}

const startUpload = async () => {
    if (selectedFiles.value.length === 0) return
    
    isUploading.value = true
    uploadProgress.value = { current: 0, total: selectedFiles.value.length }
    
    try {
        // Emit files for parent to handle upload
        emit('upload-complete', [...selectedFiles.value])
        selectedFiles.value = []
    } catch (error) {
        emit('upload-error', String(error))
    } finally {
        isUploading.value = false
    }
}
</script>

<template>
    <div class="space-y-4">
        <!-- Drop Zone -->
        <div
            @dragenter="handleDragEnter"
            @dragleave="handleDragLeave"
            @dragover="handleDragOver"
            @drop="handleDrop"
            class="border-2 border-dashed rounded-lg p-8 text-center transition-colors cursor-pointer"
            :class="isDragging ? 'border-primary bg-primary/5' : 'border-border hover:border-muted-foreground'"
            @click="fileInput?.click()"
        >
            <input
                ref="fileInput"
                type="file"
                multiple
                class="hidden"
                @change="handleFileSelect"
            />
            
            <div class="text-4xl mb-4">📂</div>
            <p class="font-medium">
                {{ isDragging ? 'Drop files here' : 'Drag & drop files or click to browse' }}
            </p>
            <p class="text-sm text-muted-foreground mt-1">
                Upload multiple files at once
            </p>
        </div>

        <!-- Selected Files List -->
        <div v-if="selectedFiles.length > 0" class="space-y-3">
            <div class="flex items-center justify-between">
                <span class="text-sm text-muted-foreground">
                    {{ selectedFiles.length }} file(s) selected • {{ formatFileSize(totalSize) }} total
                </span>
                <button 
                    @click="clearFiles"
                    class="text-sm text-muted-foreground hover:text-foreground"
                >
                    Clear all
                </button>
            </div>

            <div class="max-h-48 overflow-auto space-y-2">
                <div
                    v-for="(file, index) in selectedFiles"
                    :key="file.name"
                    class="flex items-center justify-between p-3 bg-muted/50 rounded-lg"
                >
                    <div class="flex items-center gap-3 min-w-0">
                        <span class="text-lg">📄</span>
                        <div class="min-w-0">
                            <p class="font-medium text-sm truncate">{{ file.name }}</p>
                            <p class="text-xs text-muted-foreground">{{ formatFileSize(file.size) }}</p>
                        </div>
                    </div>
                    <button
                        @click="removeFile(index)"
                        class="p-1 text-muted-foreground hover:text-destructive"
                    >
                        ✕
                    </button>
                </div>
            </div>

            <!-- Upload Button -->
            <button
                @click="startUpload"
                :disabled="isUploading"
                class="w-full px-4 py-3 rounded-lg bg-primary text-primary-foreground font-medium hover:opacity-90 disabled:opacity-50"
            >
                <span v-if="isUploading" class="flex items-center justify-center gap-2">
                    <svg class="animate-spin w-5 h-5" fill="none" viewBox="0 0 24 24">
                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
                    </svg>
                    Uploading {{ uploadProgress.current }}/{{ uploadProgress.total }}...
                </span>
                <span v-else>Upload {{ selectedFiles.length }} file(s)</span>
            </button>
        </div>
    </div>
</template>
