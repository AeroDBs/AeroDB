<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { authService } from '@/services/auth'
import type { PasswordPolicy } from '@/types'

const router = useRouter()
const route = useRoute()

const token = ref('')
const password = ref('')
const confirmPassword = ref('')
const isSubmitting = ref(false)
const isSuccess = ref(false)
const error = ref('')
const passwordPolicy = ref<PasswordPolicy | null>(null)

// Extract token from URL on mount
onMounted(async () => {
    token.value = (route.query.token as string) || ''
    if (!token.value) {
        error.value = 'Invalid or missing reset token. Please request a new password reset.'
    }

    // Load password policy
    try {
        passwordPolicy.value = await authService.getPasswordPolicy()
    } catch {
        // Use defaults if policy fetch fails
        passwordPolicy.value = {
            min_length: 8,
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_symbols: false,
        }
    }
})

const passwordsMatch = computed(() => {
    return password.value === confirmPassword.value
})

const symbolPattern = /[!@#$%^&*(),.?":{}|<>]/

const passwordValidation = computed(() => {
    if (!passwordPolicy.value || !password.value) {
        return { valid: false, errors: [] }
    }

    const errors: string[] = []
    const p = password.value
    const policy = passwordPolicy.value

    if (p.length < policy.min_length) {
        errors.push(`At least ${policy.min_length} characters`)
    }
    if (policy.require_uppercase && !/[A-Z]/.test(p)) {
        errors.push('One uppercase letter')
    }
    if (policy.require_lowercase && !/[a-z]/.test(p)) {
        errors.push('One lowercase letter')
    }
    if (policy.require_numbers && !/[0-9]/.test(p)) {
        errors.push('One number')
    }
    if (policy.require_symbols && !symbolPattern.test(p)) {
        errors.push('One special character')
    }

    return { valid: errors.length === 0, errors }
})

const hasSymbol = computed(() => symbolPattern.test(password.value))

const canSubmit = computed(() => {
    return (
        token.value &&
        password.value &&
        confirmPassword.value &&
        passwordsMatch.value &&
        passwordValidation.value.valid &&
        !isSubmitting.value
    )
})

const handleSubmit = async () => {
    if (!canSubmit.value) return

    isSubmitting.value = true
    error.value = ''

    try {
        await authService.resetPassword(token.value, password.value)
        isSuccess.value = true
    } catch (err: unknown) {
        const message = err instanceof Error ? err.message : 'Failed to reset password'
        error.value = message
    } finally {
        isSubmitting.value = false
    }
}

const goToLogin = () => {
    router.push('/login')
}

// Expose functions used in template (suppresses vue-tsc unused warnings)
defineExpose({ handleSubmit, goToLogin })
</script>

<template>
    <div class="min-h-screen flex items-center justify-center bg-background p-4">
        <div class="w-full max-w-md">
            <!-- Logo/Brand -->
            <div class="text-center mb-8">
                <h1 class="text-3xl font-bold text-primary">AeroDB</h1>
                <p class="text-muted-foreground mt-2">Reset Password</p>
            </div>

            <!-- Success State -->
            <div v-if="isSuccess" class="bg-card border border-border rounded-lg p-8 shadow-lg">
                <div class="text-center">
                    <div class="w-16 h-16 mx-auto mb-4 rounded-full bg-green-500/10 flex items-center justify-center">
                        <svg class="w-8 h-8 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                        </svg>
                    </div>
                    <h2 class="text-xl font-semibold mb-2">Password Reset Successful</h2>
                    <p class="text-muted-foreground mb-6">
                        Your password has been updated. You can now log in with your new password.
                    </p>
                    <button
                        @click="goToLogin"
                        class="w-full px-4 py-3 rounded-lg bg-primary text-primary-foreground font-medium hover:opacity-90 transition-opacity"
                    >
                        Go to Login
                    </button>
                </div>
            </div>

            <!-- Form State -->
            <div v-else class="bg-card border border-border rounded-lg p-8 shadow-lg">
                <h2 class="text-xl font-semibold mb-2">Create New Password</h2>
                <p class="text-muted-foreground mb-6">
                    Enter your new password below.
                </p>

                <form @submit.prevent="handleSubmit" class="space-y-4">
                    <div>
                        <label for="password" class="block text-sm font-medium mb-2">
                            New Password
                        </label>
                        <input
                            id="password"
                            v-model="password"
                            type="password"
                            required
                            autocomplete="new-password"
                            placeholder="••••••••"
                            class="w-full px-4 py-3 bg-background border border-input rounded-lg focus:outline-none focus:ring-2 focus:ring-ring transition-all"
                        />
                        
                        <!-- Password Policy Display -->
                        <div v-if="passwordPolicy && password" class="mt-2 space-y-1">
                            <p class="text-xs text-muted-foreground">Password requirements:</p>
                            <ul class="text-xs space-y-1">
                                <li 
                                    :class="password.length >= passwordPolicy.min_length ? 'text-green-500' : 'text-muted-foreground'"
                                    class="flex items-center gap-1"
                                >
                                    <span v-if="password.length >= passwordPolicy.min_length">✓</span>
                                    <span v-else>○</span>
                                    At least {{ passwordPolicy.min_length }} characters
                                </li>
                                <li 
                                    v-if="passwordPolicy.require_uppercase"
                                    :class="/[A-Z]/.test(password) ? 'text-green-500' : 'text-muted-foreground'"
                                    class="flex items-center gap-1"
                                >
                                    <span v-if="/[A-Z]/.test(password)">✓</span>
                                    <span v-else>○</span>
                                    One uppercase letter
                                </li>
                                <li 
                                    v-if="passwordPolicy.require_lowercase"
                                    :class="/[a-z]/.test(password) ? 'text-green-500' : 'text-muted-foreground'"
                                    class="flex items-center gap-1"
                                >
                                    <span v-if="/[a-z]/.test(password)">✓</span>
                                    <span v-else>○</span>
                                    One lowercase letter
                                </li>
                                <li 
                                    v-if="passwordPolicy.require_numbers"
                                    :class="/[0-9]/.test(password) ? 'text-green-500' : 'text-muted-foreground'"
                                    class="flex items-center gap-1"
                                >
                                    <span v-if="/[0-9]/.test(password)">✓</span>
                                    <span v-else>○</span>
                                    One number
                                </li>
                                <li 
                                    v-if="passwordPolicy.require_symbols"
                                    :class="hasSymbol ? 'text-green-500' : 'text-muted-foreground'"
                                    class="flex items-center gap-1"
                                >
                                    <span v-if="hasSymbol">✓</span>
                                    <span v-else>○</span>
                                    One special character
                                </li>
                            </ul>
                        </div>
                    </div>

                    <div>
                        <label for="confirmPassword" class="block text-sm font-medium mb-2">
                            Confirm Password
                        </label>
                        <input
                            id="confirmPassword"
                            v-model="confirmPassword"
                            type="password"
                            required
                            autocomplete="new-password"
                            placeholder="••••••••"
                            class="w-full px-4 py-3 bg-background border border-input rounded-lg focus:outline-none focus:ring-2 focus:ring-ring transition-all"
                            :class="{ 'border-destructive': confirmPassword && !passwordsMatch }"
                        />
                        <p v-if="confirmPassword && !passwordsMatch" class="text-xs text-destructive mt-1">
                            Passwords do not match
                        </p>
                    </div>

                    <div v-if="error" class="p-3 bg-destructive/10 text-destructive rounded-lg text-sm">
                        {{ error }}
                    </div>

                    <button
                        type="submit"
                        :disabled="!canSubmit"
                        class="w-full px-4 py-3 rounded-lg bg-primary text-primary-foreground font-medium hover:opacity-90 transition-opacity disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        <span v-if="isSubmitting" class="flex items-center justify-center gap-2">
                            <svg class="animate-spin w-5 h-5" fill="none" viewBox="0 0 24 24">
                                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
                                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
                            </svg>
                            Resetting Password...
                        </span>
                        <span v-else>Reset Password</span>
                    </button>

                    <div class="text-center">
                        <button
                            type="button"
                            @click="goToLogin"
                            class="text-sm text-primary hover:underline"
                        >
                            ← Back to Login
                        </button>
                    </div>
                </form>
            </div>
        </div>
    </div>
</template>

<style scoped>
</style>
