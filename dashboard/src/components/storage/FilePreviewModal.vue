<script setup lang="ts">
import { ref, computed } from 'vue'
import type { StorageFile } from '@/types'

const props = defineProps<{
    file: StorageFile
    bucketName: string
}>()

const emit = defineEmits<{
    (e: 'close'): void
}>()

const isLoading = ref(true)
const error = ref<string | null>(null)

// Determine if file is previewable
const isImage = computed(() => {
    const imageTypes = ['image/jpeg', 'image/png', 'image/gif', 'image/webp', 'image/svg+xml']
    return imageTypes.includes(props.file.content_type || '')
})

const isVideo = computed(() => {
    const videoTypes = ['video/mp4', 'video/webm', 'video/ogg']
    return videoTypes.includes(props.file.content_type || '')
})

const isAudio = computed(() => {
    const audioTypes = ['audio/mpeg', 'audio/ogg', 'audio/wav', 'audio/webm']
    return audioTypes.includes(props.file.content_type || '')
})

const isPdf = computed(() => {
    return props.file.content_type === 'application/pdf'
})

const isText = computed(() => {
    const textTypes = ['text/plain', 'text/html', 'text/css', 'text/javascript', 'application/json', 'application/xml']
    return textTypes.includes(props.file.content_type || '') || 
           props.file.name.endsWith('.md') || 
           props.file.name.endsWith('.txt') ||
           props.file.name.endsWith('.json')
})

const isPreviewable = computed(() => {
    return isImage.value || isVideo.value || isAudio.value || isPdf.value || isText.value
})

// Generate preview URL
const previewUrl = computed(() => {
    return `/storage/v1/buckets/${props.bucketName}/files/${encodeURIComponent(props.file.name)}`
})

const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`
}

const handleLoad = () => {
    isLoading.value = false
}

const handleError = () => {
    isLoading.value = false
    error.value = 'Failed to load preview'
}
</script>

<template>
    <div
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/80"
        @click="emit('close')"
    >
        <div
            class="bg-card border border-border rounded-lg shadow-lg max-w-4xl max-h-[90vh] w-full mx-4 flex flex-col overflow-hidden"
            @click.stop
        >
            <!-- Header -->
            <div class="flex items-center justify-between p-4 border-b border-border">
                <div>
                    <h3 class="font-medium">{{ file.name }}</h3>
                    <p class="text-sm text-muted-foreground">
                        {{ formatFileSize(file.size) }} • {{ file.content_type || 'Unknown type' }}
                    </p>
                </div>
                <button @click="emit('close')" class="p-2 hover:bg-muted rounded-lg transition-colors">
                    ✕
                </button>
            </div>
            
            <!-- Preview Content -->
            <div class="flex-1 overflow-auto p-4 flex items-center justify-center min-h-[400px] bg-muted/30">
                <!-- Loading -->
                <div v-if="isLoading && isPreviewable" class="text-muted-foreground">
                    Loading preview...
                </div>
                
                <!-- Error -->
                <div v-if="error" class="text-destructive">
                    {{ error }}
                </div>
                
                <!-- Image Preview -->
                <img
                    v-if="isImage"
                    :src="previewUrl"
                    :alt="file.name"
                    class="max-w-full max-h-[70vh] object-contain"
                    @load="handleLoad"
                    @error="handleError"
                />
                
                <!-- Video Preview -->
                <video
                    v-else-if="isVideo"
                    :src="previewUrl"
                    controls
                    class="max-w-full max-h-[70vh]"
                    @loadeddata="handleLoad"
                    @error="handleError"
                />
                
                <!-- Audio Preview -->
                <div v-else-if="isAudio" class="text-center">
                    <div class="w-16 h-16 mx-auto mb-4 rounded-full bg-primary/10 flex items-center justify-center">
                        🎵
                    </div>
                    <audio
                        :src="previewUrl"
                        controls
                        class="w-full max-w-md"
                        @loadeddata="handleLoad"
                        @error="handleError"
                    />
                </div>
                
                <!-- PDF Preview -->
                <iframe
                    v-else-if="isPdf"
                    :src="previewUrl"
                    class="w-full h-[70vh] border-0"
                    @load="handleLoad"
                    @error="handleError"
                />
                
                <!-- Text/Code Preview (placeholder - would need fetch) -->
                <div v-else-if="isText" class="w-full h-full flex flex-col">
                    <div class="text-center text-muted-foreground py-8">
                        <p>Text file preview</p>
                        <p class="text-sm mt-2">Download to view contents</p>
                    </div>
                </div>
                
                <!-- Not Previewable -->
                <div v-else class="text-center">
                    <div class="w-20 h-20 mx-auto mb-4 rounded-lg bg-muted flex items-center justify-center text-3xl">
                        📄
                    </div>
                    <p class="text-muted-foreground">
                        Preview not available for this file type
                    </p>
                    <p class="text-sm text-muted-foreground mt-1">
                        {{ file.content_type || 'Unknown type' }}
                    </p>
                </div>
            </div>
            
            <!-- Footer -->
            <div class="flex items-center justify-between p-4 border-t border-border bg-muted/30">
                <div class="text-sm text-muted-foreground">
                    Last modified: {{ new Date(file.updated_at).toLocaleString() }}
                </div>
                <div class="flex gap-2">
                    <a
                        :href="previewUrl"
                        :download="file.name"
                        class="px-4 py-2 rounded bg-primary text-primary-foreground hover:opacity-90"
                    >
                        Download
                    </a>
                </div>
            </div>
        </div>
    </div>
</template>
