<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useSetupStore } from '@/stores/setup'

const router = useRouter()
const setupStore = useSetupStore()

// Form data
const jwtExpiryHours = ref(24)
const refreshExpiryDays = ref(7)
const passwordMinLength = ref(8)
const requireUppercase = ref(true)
const requireNumber = ref(true)
const requireSpecial = ref(false)

const handleSubmit = async () => {
  try {
    await setupStore.saveAuth({
      jwt_expiry_hours: jwtExpiryHours.value,
      refresh_expiry_days: refreshExpiryDays.value,
      password_min_length: passwordMinLength.value,
      require_uppercase: requireUppercase.value,
      require_number: requireNumber.value,
      require_special: requireSpecial.value,
    })
    router.push('/setup/admin')
  } catch (err) {
    // Error is handled by store
  }
}

const goBack = () => {
  router.push('/setup/storage')
}
</script>

<template>
  <div>
    <!-- Header -->
    <div class="mb-8">
      <h2 class="text-2xl font-bold text-foreground mb-2">Authentication Settings</h2>
      <p class="text-muted-foreground">
        Configure JWT tokens and password requirements for your users.
      </p>
    </div>

    <!-- Form -->
    <form @submit.prevent="handleSubmit" class="space-y-8">
      <!-- JWT Settings -->
      <div class="space-y-4">
        <h3 class="text-lg font-semibold text-foreground">Token Settings</h3>
        
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <!-- JWT Expiry -->
          <div class="space-y-2">
            <label for="jwtExpiry" class="text-sm font-medium text-foreground">
              Access Token Expiry (hours)
            </label>
            <select
              id="jwtExpiry"
              v-model="jwtExpiryHours"
              class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            >
              <option :value="1">1 hour</option>
              <option :value="4">4 hours</option>
              <option :value="8">8 hours</option>
              <option :value="24">24 hours (default)</option>
              <option :value="72">3 days</option>
              <option :value="168">7 days</option>
            </select>
          </div>

          <!-- Refresh Expiry -->
          <div class="space-y-2">
            <label for="refreshExpiry" class="text-sm font-medium text-foreground">
              Refresh Token Expiry (days)
            </label>
            <select
              id="refreshExpiry"
              v-model="refreshExpiryDays"
              class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            >
              <option :value="1">1 day</option>
              <option :value="7">7 days (default)</option>
              <option :value="14">14 days</option>
              <option :value="30">30 days</option>
            </select>
          </div>
        </div>
      </div>

      <!-- Password Policy -->
      <div class="space-y-4">
        <h3 class="text-lg font-semibold text-foreground">Password Policy</h3>

        <div class="space-y-2">
          <label for="minLength" class="text-sm font-medium text-foreground">
            Minimum Password Length
          </label>
          <input
            id="minLength"
            v-model.number="passwordMinLength"
            type="number"
            min="6"
            max="32"
            class="flex h-10 w-32 rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
          />
        </div>

        <div class="space-y-3">
          <label class="text-sm font-medium text-foreground">Requirements</label>
          
          <label class="flex items-center gap-3 cursor-pointer">
            <input
              type="checkbox"
              v-model="requireUppercase"
              class="h-4 w-4 rounded border-input text-primary focus:ring-primary"
            />
            <span class="text-sm text-muted-foreground">Require uppercase letter (A-Z)</span>
          </label>

          <label class="flex items-center gap-3 cursor-pointer">
            <input
              type="checkbox"
              v-model="requireNumber"
              class="h-4 w-4 rounded border-input text-primary focus:ring-primary"
            />
            <span class="text-sm text-muted-foreground">Require number (0-9)</span>
          </label>

          <label class="flex items-center gap-3 cursor-pointer">
            <input
              type="checkbox"
              v-model="requireSpecial"
              class="h-4 w-4 rounded border-input text-primary focus:ring-primary"
            />
            <span class="text-sm text-muted-foreground">Require special character (!@#$%...)</span>
          </label>
        </div>
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
          :disabled="setupStore.loading"
          class="inline-flex items-center justify-center rounded-md text-sm font-medium ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 bg-primary text-primary-foreground hover:bg-primary/90 h-10 px-6"
        >
          <span v-if="setupStore.loading" class="mr-2">
            <svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
            </svg>
          </span>
          {{ setupStore.loading ? 'Saving...' : 'Continue' }}
        </button>
      </div>
    </form>
  </div>
</template>
