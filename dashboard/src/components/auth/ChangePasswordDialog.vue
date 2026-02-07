<script setup lang="ts">
import { ref, computed } from 'vue'
import { authService } from '@/services/auth'
import type { PasswordPolicy } from '@/types'

const props = defineProps<{
    open: boolean
    userId?: string
}>()

const emit = defineEmits<{
    (e: 'update:open', value: boolean): void
    (e: 'success'): void
}>()

const currentPassword = ref('')
const newPassword = ref('')
const confirmPassword = ref('')
const isSubmitting = ref(false)
const error = ref('')
const passwordPolicy = ref<PasswordPolicy | null>(null)

// Load password policy on mount
const loadPolicy = async () => {
    try {
        passwordPolicy.value = await authService.getPasswordPolicy()
    } catch {
        passwordPolicy.value = {
            min_length: 8,
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_symbols: false,
        }
    }
}

// Watch for dialog open to load policy
import { watch } from 'vue'
watch(() => props.open, (isOpen) => {
    if (isOpen) {
        loadPolicy()
        resetForm()
    }
})

const passwordsMatch = computed(() => newPassword.value === confirmPassword.value)

const passwordValidation = computed(() => {
    if (!passwordPolicy.value || !newPassword.value) {
        return { valid: false, errors: [] }
    }

    const errors: string[] = []
    const p = newPassword.value
    const policy = passwordPolicy.value

    if (p.length < policy.min_length) errors.push(`At least ${policy.min_length} characters`)
    if (policy.require_uppercase && !/[A-Z]/.test(p)) errors.push('One uppercase letter')
    if (policy.require_lowercase && !/[a-z]/.test(p)) errors.push('One lowercase letter')
    if (policy.require_numbers && !/[0-9]/.test(p)) errors.push('One number')
    if (policy.require_symbols && !/[!@#$%^&*(),.?":{}|<>]/.test(p)) errors.push('One special character')

    return { valid: errors.length === 0, errors }
})

const canSubmit = computed(() => {
    return (
        currentPassword.value &&
        newPassword.value &&
        confirmPassword.value &&
        passwordsMatch.value &&
        passwordValidation.value.valid &&
        !isSubmitting.value
    )
})

const resetForm = () => {
    currentPassword.value = ''
    newPassword.value = ''
    confirmPassword.value = ''
    error.value = ''
}

const closeDialog = () => {
    emit('update:open', false)
    resetForm()
}

const handleSubmit = async () => {
    if (!canSubmit.value) return

    isSubmitting.value = true
    error.value = ''

    try {
        await authService.changePassword(currentPassword.value, newPassword.value)
        emit('success')
        closeDialog()
    } catch (err: unknown) {
        const message = err instanceof Error ? err.message : 'Failed to change password'
        error.value = message
    } finally {
        isSubmitting.value = false
    }
}
</script>

<template>
    <div
        v-if="open"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
        @click="closeDialog"
    >
        <div
            class="bg-card border border-border rounded-lg shadow-lg max-w-md w-full mx-4 p-6"
            @click.stop
        >
            <h2 class="text-lg font-semibold mb-4">Change Password</h2>

            <form @submit.prevent="handleSubmit" class="space-y-4">
                <div>
                    <label class="block text-sm font-medium mb-2">Current Password</label>
                    <input
                        v-model="currentPassword"
                        type="password"
                        required
                        autocomplete="current-password"
                        class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
                    />
                </div>

                <div>
                    <label class="block text-sm font-medium mb-2">New Password</label>
                    <input
                        v-model="newPassword"
                        type="password"
                        required
                        autocomplete="new-password"
                        class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
                    />
                    
                    <!-- Password Policy -->
                    <div v-if="passwordPolicy && newPassword" class="mt-2 space-y-1">
                        <ul class="text-xs space-y-1">
                            <li :class="newPassword.length >= passwordPolicy.min_length ? 'text-green-500' : 'text-muted-foreground'" class="flex items-center gap-1">
                                <span>{{ newPassword.length >= passwordPolicy.min_length ? '✓' : '○' }}</span>
                                At least {{ passwordPolicy.min_length }} characters
                            </li>
                            <li v-if="passwordPolicy.require_uppercase" :class="/[A-Z]/.test(newPassword) ? 'text-green-500' : 'text-muted-foreground'" class="flex items-center gap-1">
                                <span>{{ /[A-Z]/.test(newPassword) ? '✓' : '○' }}</span>
                                One uppercase letter
                            </li>
                            <li v-if="passwordPolicy.require_lowercase" :class="/[a-z]/.test(newPassword) ? 'text-green-500' : 'text-muted-foreground'" class="flex items-center gap-1">
                                <span>{{ /[a-z]/.test(newPassword) ? '✓' : '○' }}</span>
                                One lowercase letter
                            </li>
                            <li v-if="passwordPolicy.require_numbers" :class="/[0-9]/.test(newPassword) ? 'text-green-500' : 'text-muted-foreground'" class="flex items-center gap-1">
                                <span>{{ /[0-9]/.test(newPassword) ? '✓' : '○' }}</span>
                                One number
                            </li>
                        </ul>
                    </div>
                </div>

                <div>
                    <label class="block text-sm font-medium mb-2">Confirm New Password</label>
                    <input
                        v-model="confirmPassword"
                        type="password"
                        required
                        autocomplete="new-password"
                        class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
                        :class="{ 'border-destructive': confirmPassword && !passwordsMatch }"
                    />
                    <p v-if="confirmPassword && !passwordsMatch" class="text-xs text-destructive mt-1">
                        Passwords do not match
                    </p>
                </div>

                <div v-if="error" class="p-3 bg-destructive/10 text-destructive rounded-lg text-sm">
                    {{ error }}
                </div>

                <div class="flex justify-end gap-3 pt-2">
                    <button
                        type="button"
                        @click="closeDialog"
                        class="px-4 py-2 rounded border border-border hover:bg-secondary"
                    >
                        Cancel
                    </button>
                    <button
                        type="submit"
                        :disabled="!canSubmit"
                        class="px-4 py-2 rounded bg-primary text-primary-foreground hover:opacity-90 disabled:opacity-50"
                    >
                        {{ isSubmitting ? 'Changing...' : 'Change Password' }}
                    </button>
                </div>
            </form>
        </div>
    </div>
</template>
