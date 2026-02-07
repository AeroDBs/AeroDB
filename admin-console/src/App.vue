<script setup lang="ts">
import { RouterView } from 'vue-router'
import { onMounted } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { useSetupStore } from '@/stores/setup'

const authStore = useAuthStore()
const setupStore = useSetupStore()

onMounted(async () => {
  // Check setup status first (required before any other initialization)
  await setupStore.fetchStatus()
  
  // Initialize auth store (fetch user if token exists)
  if (setupStore.isReady) {
    authStore.initialize()
  }
})
</script>

<template>
  <RouterView />
</template>

<style scoped>
</style>

