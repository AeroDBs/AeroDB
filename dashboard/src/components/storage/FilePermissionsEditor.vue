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
    (e: 'updated'): void
}>()

const isLoading = ref(false)
const isSaving = ref(false)
const error = ref<string | null>(null)

const isPublic = ref(false)
const allowedRoles = ref<string[]>([])
const newRole = ref('')
const ownerId = ref('')

// Load current permissions
const loadPermissions = async () => {
    isLoading.value = true
    error.value = null
    
    try {
        const perms = await storageService.getFilePermissions(props.bucketName, props.filePath)
        isPublic.value = perms.public
        allowedRoles.value = [...perms.allowed_roles]
        ownerId.value = perms.owner_id
    } catch (e) {
        error.value = 'Failed to load permissions'
    } finally {
        isLoading.value = false
    }
}

// Save permissions
const savePermissions = async () => {
    isSaving.value = true
    error.value = null
    
    try {
        await storageService.updateFilePermissions(props.bucketName, props.filePath, {
            public: isPublic.value,
            allowed_roles: allowedRoles.value,
        })
        emit('updated')
        close()
    } catch (e) {
        error.value = 'Failed to save permissions'
    } finally {
        isSaving.value = false
    }
}

const addRole = () => {
    if (newRole.value.trim() && !allowedRoles.value.includes(newRole.value.trim())) {
        allowedRoles.value.push(newRole.value.trim())
        newRole.value = ''
    }
}

const removeRole = (index: number) => {
    allowedRoles.value.splice(index, 1)
}

const close = () => {
    emit('update:open', false)
}

// Load on open
watch(() => props.open, (isOpen) => {
    if (isOpen) {
        loadPermissions()
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
            class="bg-card border border-border rounded-lg shadow-lg max-w-md w-full mx-4 p-6"
            @click.stop
        >
            <h2 class="text-lg font-semibold mb-1">File Permissions</h2>
            <p class="text-sm text-muted-foreground mb-4 font-mono truncate">{{ filePath }}</p>
            
            <div v-if="isLoading" class="text-center py-8 text-muted-foreground">
                Loading permissions...
            </div>
            
            <div v-else-if="error && isLoading" class="text-destructive text-sm mb-4">
                {{ error }}
            </div>
            
            <form v-else @submit.prevent="savePermissions" class="space-y-4">
                <!-- Public toggle -->
                <div class="flex items-center justify-between p-3 bg-muted/50 rounded-lg">
                    <div>
                        <p class="font-medium">Public Access</p>
                        <p class="text-sm text-muted-foreground">Anyone can access this file</p>
                    </div>
                    <label class="relative inline-flex items-center cursor-pointer">
                        <input type="checkbox" v-model="isPublic" class="sr-only peer" />
                        <div class="w-11 h-6 bg-muted rounded-full peer peer-checked:bg-primary after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:after:translate-x-full"></div>
                    </label>
                </div>
                
                <!-- Allowed Roles -->
                <div v-if="!isPublic" class="space-y-3">
                    <label class="block text-sm font-medium">Allowed Roles</label>
                    
                    <div v-if="allowedRoles.length > 0" class="flex flex-wrap gap-2">
                        <span
                            v-for="(role, index) in allowedRoles"
                            :key="role"
                            class="inline-flex items-center gap-1 px-2 py-1 bg-muted rounded text-sm"
                        >
                            {{ role }}
                            <button
                                type="button"
                                @click="removeRole(index)"
                                class="text-muted-foreground hover:text-destructive"
                            >
                                ✕
                            </button>
                        </span>
                    </div>
                    
                    <div class="flex gap-2">
                        <input
                            v-model="newRole"
                            type="text"
                            placeholder="Add role..."
                            class="flex-1 px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
                            @keypress.enter.prevent="addRole"
                        />
                        <button
                            type="button"
                            @click="addRole"
                            class="px-3 py-2 bg-muted hover:bg-muted/80 rounded-md"
                        >
                            Add
                        </button>
                    </div>
                </div>
                
                <!-- Owner -->
                <div class="text-sm text-muted-foreground">
                    Owner: <span class="font-mono">{{ ownerId || 'Unknown' }}</span>
                </div>
                
                <!-- Error -->
                <div v-if="error && !isLoading" class="p-3 bg-destructive/10 text-destructive rounded-lg text-sm">
                    {{ error }}
                </div>
                
                <!-- Actions -->
                <div class="flex justify-end gap-3 pt-2">
                    <button
                        type="button"
                        @click="close"
                        class="px-4 py-2 rounded border border-border hover:bg-secondary"
                    >
                        Cancel
                    </button>
                    <button
                        type="submit"
                        :disabled="isSaving"
                        class="px-4 py-2 rounded bg-primary text-primary-foreground hover:opacity-90 disabled:opacity-50"
                    >
                        {{ isSaving ? 'Saving...' : 'Save Permissions' }}
                    </button>
                </div>
            </form>
        </div>
    </div>
</template>
