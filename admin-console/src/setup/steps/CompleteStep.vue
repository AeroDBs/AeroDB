<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useSetupStore } from '@/stores/setup'

const router = useRouter()
const setupStore = useSetupStore()

const completing = ref(true)
const success = ref(false)
const errorOccurred = ref(false)

onMounted(async () => {
  // Start the completion process automatically
  try {
    await setupStore.completeSetup()
    success.value = true
  } catch (err) {
    errorOccurred.value = true
  } finally {
    completing.value = false
  }
})

const goToDashboard = () => {
  router.push('/')
}

const retrySetup = () => {
  completing.value = true
  errorOccurred.value = false
  setupStore.completeSetup()
    .then(() => {
      success.value = true
    })
    .catch(() => {
      errorOccurred.value = true
    })
    .finally(() => {
      completing.value = false
    })
}
</script>

<template>
  <div class="text-center py-8">
    <!-- Completing State -->
    <template v-if="completing">
      <div class="w-20 h-20 mx-auto mb-6 relative">
        <div class="absolute inset-0 border-4 border-muted rounded-full"></div>
        <div class="absolute inset-0 border-4 border-primary border-t-transparent rounded-full animate-spin"></div>
      </div>
      <h2 class="text-2xl font-bold text-foreground mb-2">Completing Setup...</h2>
      <p class="text-muted-foreground">
        Please wait while we finalize your configuration.
      </p>
    </template>

    <!-- Success State -->
    <template v-else-if="success">
      <div class="w-20 h-20 bg-green-500/10 rounded-full flex items-center justify-center mx-auto mb-6">
        <svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-green-500">
          <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/>
          <polyline points="22 4 12 14.01 9 11.01"/>
        </svg>
      </div>

      <h2 class="text-2xl font-bold text-foreground mb-2">Setup Complete! 🎉</h2>
      <p class="text-muted-foreground max-w-md mx-auto mb-8">
        AeroDB is now configured and ready to use. You can start building your application!
      </p>

      <!-- Quick Stats -->
      <div class="grid grid-cols-3 gap-4 max-w-lg mx-auto mb-8">
        <div class="bg-card border border-border rounded-lg p-4">
          <div class="text-2xl font-bold text-primary">✓</div>
          <div class="text-sm text-muted-foreground mt-1">Storage</div>
        </div>
        <div class="bg-card border border-border rounded-lg p-4">
          <div class="text-2xl font-bold text-primary">✓</div>
          <div class="text-sm text-muted-foreground mt-1">Auth</div>
        </div>
        <div class="bg-card border border-border rounded-lg p-4">
          <div class="text-2xl font-bold text-primary">✓</div>
          <div class="text-sm text-muted-foreground mt-1">Admin</div>
        </div>
      </div>

      <button
        @click="goToDashboard"
        class="inline-flex items-center justify-center rounded-md text-sm font-medium ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 bg-primary text-primary-foreground hover:bg-primary/90 h-11 px-8 shadow-lg"
      >
        Go to Dashboard
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="ml-2">
          <path d="M5 12h14"/>
          <path d="m12 5 7 7-7 7"/>
        </svg>
      </button>
    </template>

    <!-- Error State -->
    <template v-else-if="errorOccurred">
      <div class="w-20 h-20 bg-destructive/10 rounded-full flex items-center justify-center mx-auto mb-6">
        <svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-destructive">
          <circle cx="12" cy="12" r="10"/>
          <line x1="15" x2="9" y1="9" y2="15"/>
          <line x1="9" x2="15" y1="9" y2="15"/>
        </svg>
      </div>

      <h2 class="text-2xl font-bold text-foreground mb-2">Setup Failed</h2>
      <p class="text-muted-foreground max-w-md mx-auto mb-4">
        There was an error completing the setup. Please try again.
      </p>
      <p v-if="setupStore.error" class="text-sm text-destructive mb-8">
        {{ setupStore.error }}
      </p>

      <button
        @click="retrySetup"
        class="inline-flex items-center justify-center rounded-md text-sm font-medium ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 bg-primary text-primary-foreground hover:bg-primary/90 h-10 px-6"
      >
        Retry
      </button>
    </template>
  </div>
</template>
