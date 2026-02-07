<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useSetupStore } from '@/stores/setup'

const router = useRouter()
const setupStore = useSetupStore()

// Form data
const dataDir = ref('/var/lib/aerodb/data')
const walDir = ref('/var/lib/aerodb/wal')
const snapshotDir = ref('/var/lib/aerodb/snapshots')

// Validation state
const validating = ref(false)

const handleSubmit = async () => {
  validating.value = true
  
  try {
    await setupStore.saveStorage({
      data_dir: dataDir.value,
      wal_dir: walDir.value,
      snapshot_dir: snapshotDir.value,
    })
    router.push('/setup/auth')
  } catch (err) {
    // Error is handled by store
  } finally {
    validating.value = false
  }
}

const goBack = () => {
  router.push('/setup/welcome')
}
</script>

<template>
  <div>
    <!-- Header -->
    <div class="mb-8">
      <h2 class="text-2xl font-bold text-foreground mb-2">Storage Configuration</h2>
      <p class="text-muted-foreground">
        Configure where AeroDB will store your data. These directories will be created if they don't exist.
      </p>
    </div>

    <!-- Form -->
    <form @submit.prevent="handleSubmit" class="space-y-6">
      <!-- Data Directory -->
      <div class="space-y-2">
        <label for="dataDir" class="text-sm font-medium text-foreground">
          Data Directory
        </label>
        <input
          id="dataDir"
          v-model="dataDir"
          type="text"
          required
          placeholder="/var/lib/aerodb/data"
          class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
        />
        <p class="text-xs text-muted-foreground">
          Primary storage for your database files
        </p>
      </div>

      <!-- WAL Directory -->
      <div class="space-y-2">
        <label for="walDir" class="text-sm font-medium text-foreground">
          Write-Ahead Log (WAL) Directory
        </label>
        <input
          id="walDir"
          v-model="walDir"
          type="text"
          required
          placeholder="/var/lib/aerodb/wal"
          class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
        />
        <p class="text-xs text-muted-foreground">
          Transaction logs for durability and recovery
        </p>
      </div>

      <!-- Snapshot Directory -->
      <div class="space-y-2">
        <label for="snapshotDir" class="text-sm font-medium text-foreground">
          Snapshot Directory
        </label>
        <input
          id="snapshotDir"
          v-model="snapshotDir"
          type="text"
          required
          placeholder="/var/lib/aerodb/snapshots"
          class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
        />
        <p class="text-xs text-muted-foreground">
          Point-in-time snapshots and backups
        </p>
      </div>

      <!-- Info Box -->
      <div class="bg-muted/50 border border-border rounded-lg p-4">
        <div class="flex gap-3">
          <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-primary flex-shrink-0 mt-0.5">
            <circle cx="12" cy="12" r="10"/>
            <path d="M12 16v-4"/>
            <path d="M12 8h.01"/>
          </svg>
          <div class="text-sm text-muted-foreground">
            <p class="font-medium text-foreground mb-1">Permissions Required</p>
            <p>The server process must have read/write access to these directories. They will be created automatically if they don't exist.</p>
          </div>
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
          {{ setupStore.loading ? 'Validating...' : 'Continue' }}
        </button>
      </div>
    </form>
  </div>
</template>
