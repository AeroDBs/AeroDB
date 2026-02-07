<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useSetupStore } from '@/stores/setup'

const router = useRouter()
const setupStore = useSetupStore()

// Form data
const email = ref('')
const password = ref('')
const confirmPassword = ref('')
const showPassword = ref(false)

// Validation
const passwordsMatch = computed(() => password.value === confirmPassword.value)
const isValidEmail = computed(() => /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email.value))
const isValidPassword = computed(() => password.value.length >= 8)
const canSubmit = computed(() => 
  isValidEmail.value && 
  isValidPassword.value && 
  passwordsMatch.value
)

const handleSubmit = async () => {
  if (!canSubmit.value) return

  try {
    await setupStore.createAdmin({
      email: email.value,
      password: password.value,
      confirm_password: confirmPassword.value,
    })
    router.push('/setup/review')
  } catch (err) {
    // Error is handled by store
  }
}

const goBack = () => {
  router.push('/setup/auth')
}
</script>

<template>
  <div>
    <!-- Header -->
    <div class="mb-8">
      <h2 class="text-2xl font-bold text-foreground mb-2">Create Admin Account</h2>
      <p class="text-muted-foreground">
        Set up your administrator account. This will be the super-admin with full access.
      </p>
    </div>

    <!-- Warning -->
    <div class="bg-amber-500/10 border border-amber-500/20 rounded-lg p-4 mb-6">
      <div class="flex gap-3">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-amber-500 flex-shrink-0 mt-0.5">
          <path d="m21.73 18-8-14a2 2 0 0 0-3.46 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/>
          <path d="M12 9v4"/>
          <path d="M12 17h.01"/>
        </svg>
        <div class="text-sm">
          <p class="font-medium text-amber-600 dark:text-amber-400 mb-1">Important</p>
          <p class="text-amber-600/80 dark:text-amber-400/80">
            This creates a super-admin account and can only be done once during setup.
            Make sure to remember these credentials!
          </p>
        </div>
      </div>
    </div>

    <!-- Form -->
    <form @submit.prevent="handleSubmit" class="space-y-6">
      <!-- Email -->
      <div class="space-y-2">
        <label for="email" class="text-sm font-medium text-foreground">
          Admin Email
        </label>
        <input
          id="email"
          v-model="email"
          type="email"
          required
          placeholder="admin@example.com"
          class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
          :class="{ 'border-destructive': email && !isValidEmail }"
        />
        <p v-if="email && !isValidEmail" class="text-xs text-destructive">
          Please enter a valid email address
        </p>
      </div>

      <!-- Password -->
      <div class="space-y-2">
        <label for="password" class="text-sm font-medium text-foreground">
          Password
        </label>
        <div class="relative">
          <input
            id="password"
            v-model="password"
            :type="showPassword ? 'text' : 'password'"
            required
            placeholder="••••••••"
            class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 pr-10 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            :class="{ 'border-destructive': password && !isValidPassword }"
          />
          <button
            type="button"
            @click="showPassword = !showPassword"
            class="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
          >
            <svg v-if="!showPassword" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z"/>
              <circle cx="12" cy="12" r="3"/>
            </svg>
            <svg v-else xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M9.88 9.88a3 3 0 1 0 4.24 4.24"/>
              <path d="M10.73 5.08A10.43 10.43 0 0 1 12 5c7 0 10 7 10 7a13.16 13.16 0 0 1-1.67 2.68"/>
              <path d="M6.61 6.61A13.526 13.526 0 0 0 2 12s3 7 10 7a9.74 9.74 0 0 0 5.39-1.61"/>
              <line x1="2" x2="22" y1="2" y2="22"/>
            </svg>
          </button>
        </div>
        <p v-if="password && !isValidPassword" class="text-xs text-destructive">
          Password must be at least 8 characters
        </p>
      </div>

      <!-- Confirm Password -->
      <div class="space-y-2">
        <label for="confirmPassword" class="text-sm font-medium text-foreground">
          Confirm Password
        </label>
        <input
          id="confirmPassword"
          v-model="confirmPassword"
          :type="showPassword ? 'text' : 'password'"
          required
          placeholder="••••••••"
          class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
          :class="{ 'border-destructive': confirmPassword && !passwordsMatch }"
        />
        <p v-if="confirmPassword && !passwordsMatch" class="text-xs text-destructive">
          Passwords do not match
        </p>
      </div>

      <!-- Actions -->
      <div class="flex justify-between pt-4">
        <button
          type="button"
          @click="goBack"
          class="inline-flex items-center justify-center rounded-md text-sm font-medium h-10 px-4 border border-input bg-background hover:bg-accent hover:text-accent-foreground"
        >
          Back
        </button>
        <button
          type="submit"
          :disabled="!canSubmit || setupStore.loading"
          class="inline-flex items-center justify-center rounded-md text-sm font-medium ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 bg-primary text-primary-foreground hover:bg-primary/90 h-10 px-6"
        >
          <span v-if="setupStore.loading" class="mr-2">
            <svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
            </svg>
          </span>
          {{ setupStore.loading ? 'Creating...' : 'Continue' }}
        </button>
      </div>
    </form>
  </div>
</template>
