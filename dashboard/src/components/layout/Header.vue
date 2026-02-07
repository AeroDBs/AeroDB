<script setup lang="ts">
import { computed } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { useUIStore } from '@/stores/ui'
import { useRouter } from 'vue-router'

const authStore = useAuthStore()
const uiStore = useUIStore()
const router = useRouter()

const userEmail = computed(() => authStore.user?.email || 'Unknown')

const handleLogout = () => {
  authStore.signOut()
  router.push('/login')
}
</script>

<template>
  <header class="bg-card border-b border-border px-6 py-4 flex items-center justify-between">
    <div class="flex items-center gap-4">
      <h2 class="text-lg font-semibold">Admin Dashboard</h2>
    </div>
    
    <div class="flex items-center gap-4">
      <button
        @click="uiStore.toggleTheme"
        class="p-2 rounded hover:bg-secondary"
        :title="uiStore.theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'"
      >
        <span v-if="uiStore.theme === 'dark'">☀️</span>
        <span v-else>🌙</span>
      </button>
      
      <div class="flex items-center gap-2">
        <span class="text-sm text-muted-foreground">{{ userEmail }}</span>
        <button
          @click="handleLogout"
          class="px-4 py-2 rounded bg-destructive text-destructive-foreground hover:opacity-90 transition-opacity"
        >
          Logout
        </button>
      </div>
    </div>
  </header>
</template>

<style scoped>
</style>
