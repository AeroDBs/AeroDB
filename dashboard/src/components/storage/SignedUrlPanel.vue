<script setup lang="ts">
import { ref, watch } from 'vue'
import { storageService } from '@/services/storage'

const props = defineProps<{
    open: boolean
    bucketName: string
    filePath: string
}>()

const emit = defineEmits<{
    (e: 'update:open', value: boolean): void
}>()

const isLoading = ref(false)
const isCreating = ref(false)
const error = ref<string | null>(null)

const signedUrls = ref<Array<{
    id: string
    url: string
    expires_at: string
    created_at: string
}>>([])

const expiresInHours = ref(24)
const newUrlResult = ref<{ url: string } | null>(null)

// Load signed URLs
const loadSignedUrls = async () => {
    isLoading.value = true
    error.value = null
    
    try {
        signedUrls.value = await storageService.listSignedUrls(props.bucketName, props.filePath)
    } catch (e) {
        error.value = 'Failed to load signed URLs'
    } finally {
        isLoading.value = false
    }
}

// Create new signed URL
const createSignedUrl = async () => {
    isCreating.value = true
    newUrlResult.value = null
    
    try {
        const expiresInSeconds = expiresInHours.value * 3600
        const result = await storageService.createSignedUrl(props.bucketName, props.filePath, expiresInSeconds)
        newUrlResult.value = { url: result.url }
        await loadSignedUrls()
    } catch (e) {
        error.value = 'Failed to create signed URL'
    } finally {
        isCreating.value = false
    }
}

// Revoke a signed URL
const revokeUrl = async (urlId: string) => {
    try {
        await storageService.revokeSignedUrl(props.bucketName, props.filePath, urlId)
        signedUrls.value = signedUrls.value.filter(u => u.id !== urlId)
    } catch (e) {
        error.value = 'Failed to revoke URL'
    }
}

const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text)
}

const formatExpiry = (timestamp: string): string => {
    const date = new Date(timestamp)
    const now = new Date()
    const diffMs = date.getTime() - now.getTime()
    
    if (diffMs < 0) return 'Expired'
    
    const diffHours = Math.floor(diffMs / (1000 * 60 * 60))
    if (diffHours < 1) return 'Less than 1 hour'
    if (diffHours < 24) return `${diffHours} hours`
    return `${Math.floor(diffHours / 24)} days`
}

const close = () => {
    emit('update:open', false)
    newUrlResult.value = null
}

// Load on open
watch(() => props.open, (isOpen) => {
    if (isOpen) {
        loadSignedUrls()
    }
})
</script>

<template>
    <div
        v-if="open"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
        @click="close"
    >
        <div
            class="bg-card border border-border rounded-lg shadow-lg max-w-lg w-full mx-4 p-6"
            @click.stop
        >
            <h2 class="text-lg font-semibold mb-1">Signed URLs</h2>
            <p class="text-sm text-muted-foreground mb-4 font-mono truncate">{{ filePath }}</p>
            
            <!-- Create New URL -->
            <div class="bg-muted/50 rounded-lg p-4 mb-4">
                <h3 class="font-medium mb-3">Create Signed URL</h3>
                
                <div v-if="newUrlResult" class="mb-4 p-3 bg-green-500/10 border border-green-500/30 rounded-lg">
                    <p class="text-sm text-green-500 mb-2">URL created successfully!</p>
                    <div class="flex items-center gap-2">
                        <input
                            :value="newUrlResult.url"
                            readonly
                            class="flex-1 px-2 py-1 bg-background border border-input rounded text-xs font-mono"
                        />
                        <button
                            @click="copyToClipboard(newUrlResult.url)"
                            class="px-2 py-1 bg-primary text-primary-foreground text-xs rounded hover:opacity-90"
                        >
                            Copy
                        </button>
                    </div>
                </div>
                
                <div class="flex items-center gap-3">
                    <div class="flex items-center gap-2">
                        <label class="text-sm">Expires in:</label>
                        <select
                            v-model="expiresInHours"
                            class="px-2 py-1 bg-background border border-input rounded-md text-sm"
                        >
                            <option :value="1">1 hour</option>
                            <option :value="24">24 hours</option>
                            <option :value="168">7 days</option>
                            <option :value="720">30 days</option>
                        </select>
                    </div>
                    <button
                        @click="createSignedUrl"
                        :disabled="isCreating"
                        class="px-4 py-2 bg-primary text-primary-foreground rounded hover:opacity-90 disabled:opacity-50 text-sm"
                    >
                        {{ isCreating ? 'Creating...' : 'Generate URL' }}
                    </button>
                </div>
            </div>
            
            <!-- Existing URLs -->
            <div>
                <h3 class="font-medium mb-3">Active URLs</h3>
                
                <div v-if="isLoading" class="text-center py-4 text-muted-foreground text-sm">
                    Loading...
                </div>
                
                <div v-else-if="signedUrls.length === 0" class="text-center py-4 text-muted-foreground text-sm">
                    No active signed URLs
                </div>
                
                <div v-else class="space-y-2 max-h-48 overflow-auto">
                    <div
                        v-for="url in signedUrls"
                        :key="url.id"
                        class="flex items-center justify-between p-3 bg-background border border-border rounded-lg"
                    >
                        <div class="min-w-0 flex-1">
                            <p class="text-xs font-mono text-muted-foreground truncate">
                                {{ url.url.slice(0, 50) }}...
                            </p>
                            <p class="text-xs text-muted-foreground mt-1">
                                Expires: {{ formatExpiry(url.expires_at) }}
                            </p>
                        </div>
                        <div class="flex items-center gap-2 ml-2">
                            <button
                                @click="copyToClipboard(url.url)"
                                class="text-xs text-primary hover:underline"
                            >
                                Copy
                            </button>
                            <button
                                @click="revokeUrl(url.id)"
                                class="text-xs text-destructive hover:underline"
                            >
                                Revoke
                            </button>
                        </div>
                    </div>
                </div>
            </div>
            
            <!-- Error -->
            <div v-if="error" class="mt-4 p-3 bg-destructive/10 text-destructive rounded-lg text-sm">
                {{ error }}
            </div>
            
            <!-- Close -->
            <div class="flex justify-end mt-4">
                <button @click="close" class="px-4 py-2 bg-muted hover:bg-muted/80 rounded">
                    Close
                </button>
            </div>
        </div>
    </div>
</template>
