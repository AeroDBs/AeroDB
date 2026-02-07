<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useSetupStore } from '@/stores/setup'

const router = useRouter()
const setupStore = useSetupStore()

const confirmed = ref(false)

const handleSubmit = () => {
  if (confirmed.value) {
    router.push('/setup/complete')
  }
}

const goBack = () => {
  router.push('/setup/admin')
}
</script>

<template>
  <div>
    <!-- Header -->
    <div class="mb-8">
      <h2 class="text-2xl font-bold text-foreground mb-2">Review Configuration</h2>
      <p class="text-muted-foreground">
        Please review your configuration before finalizing the setup.
      </p>
    </div>

    <!-- Configuration Summary -->
    <div class="space-y-6 mb-8">
      <!-- Storage Config -->
      <div class="bg-card border border-border rounded-lg overflow-hidden">
        <div class="bg-muted/50 px-4 py-3 border-b border-border flex items-center gap-2">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-primary">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
          </svg>
          <h3 class="font-semibold text-foreground">Storage</h3>
          <span v-if="setupStore.storageConfigured" class="ml-auto text-xs bg-green-500/10 text-green-600 px-2 py-0.5 rounded-full">
            ✓ Configured
          </span>
        </div>
        <div class="p-4 text-sm space-y-2">
          <div class="flex justify-between">
            <span class="text-muted-foreground">Data Directory</span>
            <code class="bg-muted px-2 py-0.5 rounded text-xs">{{ setupStore.storageConfig?.data_dir || '/var/lib/aerodb/data' }}</code>
          </div>
          <div class="flex justify-between">
            <span class="text-muted-foreground">WAL Directory</span>
            <code class="bg-muted px-2 py-0.5 rounded text-xs">{{ setupStore.storageConfig?.wal_dir || '/var/lib/aerodb/wal' }}</code>
          </div>
          <div class="flex justify-between">
            <span class="text-muted-foreground">Snapshot Directory</span>
            <code class="bg-muted px-2 py-0.5 rounded text-xs">{{ setupStore.storageConfig?.snapshot_dir || '/var/lib/aerodb/snapshots' }}</code>
          </div>
        </div>
      </div>

      <!-- Auth Config -->
      <div class="bg-card border border-border rounded-lg overflow-hidden">
        <div class="bg-muted/50 px-4 py-3 border-b border-border flex items-center gap-2">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-primary">
            <rect width="18" height="11" x="3" y="11" rx="2" ry="2"/>
            <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
          </svg>
          <h3 class="font-semibold text-foreground">Authentication</h3>
          <span v-if="setupStore.authConfigured" class="ml-auto text-xs bg-green-500/10 text-green-600 px-2 py-0.5 rounded-full">
            ✓ Configured
          </span>
        </div>
        <div class="p-4 text-sm space-y-2">
          <div class="flex justify-between">
            <span class="text-muted-foreground">Access Token Expiry</span>
            <span class="text-foreground">{{ setupStore.authConfig?.jwt_expiry_hours || 24 }} hours</span>
          </div>
          <div class="flex justify-between">
            <span class="text-muted-foreground">Refresh Token Expiry</span>
            <span class="text-foreground">{{ setupStore.authConfig?.refresh_expiry_days || 7 }} days</span>
          </div>
          <div class="flex justify-between">
            <span class="text-muted-foreground">Min Password Length</span>
            <span class="text-foreground">{{ setupStore.authConfig?.password_min_length || 8 }} characters</span>
          </div>
        </div>
      </div>

      <!-- Admin Config -->
      <div class="bg-card border border-border rounded-lg overflow-hidden">
        <div class="bg-muted/50 px-4 py-3 border-b border-border flex items-center gap-2">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-primary">
            <path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2"/>
            <circle cx="12" cy="7" r="4"/>
          </svg>
          <h3 class="font-semibold text-foreground">Admin User</h3>
          <span v-if="setupStore.adminCreated" class="ml-auto text-xs bg-green-500/10 text-green-600 px-2 py-0.5 rounded-full">
            ✓ Created
          </span>
        </div>
        <div class="p-4 text-sm">
          <div class="flex justify-between">
            <span class="text-muted-foreground">Admin Email</span>
            <span class="text-foreground">{{ setupStore.adminEmail || '-' }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Confirmation -->
    <div class="bg-muted/50 border border-border rounded-lg p-4 mb-6">
      <label class="flex items-start gap-3 cursor-pointer">
        <input
          type="checkbox"
          v-model="confirmed"
          class="h-4 w-4 mt-0.5 rounded border-input text-primary focus:ring-primary"
        />
        <span class="text-sm text-muted-foreground">
          I confirm that the above configuration is correct and I'm ready to complete the setup.
          <strong class="text-foreground">This action cannot be undone.</strong>
        </span>
      </label>
    </div>

    <!-- Actions -->
    <div class="flex justify-between">
      <button
        type="button"
        @click="goBack"
        class="inline-flex items-center justify-center rounded-md text-sm font-medium h-10 px-4 border border-input bg-background hover:bg-accent hover:text-accent-foreground"
      >
        Back
      </button>
      <button
        @click="handleSubmit"
        :disabled="!confirmed"
        class="inline-flex items-center justify-center rounded-md text-sm font-medium ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 bg-primary text-primary-foreground hover:bg-primary/90 h-10 px-6"
      >
        Complete Setup
      </button>
    </div>
  </div>
</template>
