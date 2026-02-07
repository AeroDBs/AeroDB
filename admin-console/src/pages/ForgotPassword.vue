<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { authService } from '@/services/auth'

const router = useRouter()

const email = ref('')
const isSubmitting = ref(false)
const isSubmitted = ref(false)
const error = ref('')

const isValidEmail = computed(() => {
    return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email.value)
})

const handleSubmit = async () => {
    if (!isValidEmail.value) {
        error.value = 'Please enter a valid email address'
        return
    }

    isSubmitting.value = true
    error.value = ''

    try {
        await authService.forgotPassword(email.value)
        isSubmitted.value = true
    } catch (err: unknown) {
        const message = err instanceof Error ? err.message : 'Failed to send reset email'
        error.value = message
    } finally {
        isSubmitting.value = false
    }
}

const goToLogin = () => {
    router.push('/login')
}
</script>

<template>
    <div class="min-h-screen flex items-center justify-center bg-background p-4">
        <div class="w-full max-w-md">
            <!-- Logo/Brand -->
            <div class="text-center mb-8">
                <h1 class="text-3xl font-bold text-primary">AeroDB</h1>
                <p class="text-muted-foreground mt-2">Password Recovery</p>
            </div>

            <!-- Success State -->
            <div v-if="isSubmitted" class="bg-card border border-border rounded-lg p-8 shadow-lg">
                <div class="text-center">
                    <div class="w-16 h-16 mx-auto mb-4 rounded-full bg-green-500/10 flex items-center justify-center">
                        <svg class="w-8 h-8 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                        </svg>
                    </div>
                    <h2 class="text-xl font-semibold mb-2">Check Your Email</h2>
                    <p class="text-muted-foreground mb-6">
                        If an account exists for <strong>{{ email }}</strong>, we've sent password reset instructions.
                    </p>
                    <button
                        @click="goToLogin"
                        class="w-full px-4 py-3 rounded-lg bg-primary text-primary-foreground font-medium hover:opacity-90 transition-opacity"
                    >
                        Back to Login
                    </button>
                </div>
            </div>

            <!-- Form State -->
            <div v-else class="bg-card border border-border rounded-lg p-8 shadow-lg">
                <h2 class="text-xl font-semibold mb-2">Forgot Password?</h2>
                <p class="text-muted-foreground mb-6">
                    Enter your email address and we'll send you instructions to reset your password.
                </p>

                <form @submit.prevent="handleSubmit" class="space-y-4">
                    <div>
                        <label for="email" class="block text-sm font-medium mb-2">
                            Email Address
                        </label>
                        <input
                            id="email"
                            v-model="email"
                            type="email"
                            required
                            autocomplete="email"
                            placeholder="you@example.com"
                            class="w-full px-4 py-3 bg-background border border-input rounded-lg focus:outline-none focus:ring-2 focus:ring-ring transition-all"
                            :class="{ 'border-destructive': error }"
                        />
                    </div>

                    <div v-if="error" class="p-3 bg-destructive/10 text-destructive rounded-lg text-sm">
                        {{ error }}
                    </div>

                    <button
                        type="submit"
                        :disabled="isSubmitting || !email"
                        class="w-full px-4 py-3 rounded-lg bg-primary text-primary-foreground font-medium hover:opacity-90 transition-opacity disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        <span v-if="isSubmitting" class="flex items-center justify-center gap-2">
                            <svg class="animate-spin w-5 h-5" fill="none" viewBox="0 0 24 24">
                                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
                                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
                            </svg>
                            Sending...
                        </span>
                        <span v-else>Send Reset Instructions</span>
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
