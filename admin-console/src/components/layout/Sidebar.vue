<script setup lang="ts">
import { RouterLink } from 'vue-router'
import { useUIStore } from '@/stores/ui'

defineProps<{
  open: boolean
}>()

const uiStore = useUIStore()

const navItems = [
  { to: '/', label: 'Home', icon: '🏠' },
  { to: '/database', label: 'Database', icon: '🗄️' },
  { to: '/auth', label: 'Auth', icon: '👥' },
  { to: '/storage', label: 'Storage', icon: '📦' },
  { to: '/functions', label: 'Functions', icon: '⚙️' },
  { to: '/realtime', label: 'Real-Time', icon: '⚡' },
  { to: '/cluster', label: 'Cluster', icon: '🖥️' },
  { to: '/backup', label: 'Backup', icon: '💾' },
  { to: '/snapshots', label: 'Snapshots', icon: '📸' },
  { to: '/logs', label: 'Logs', icon: '📋' },
  { to: '/metrics', label: 'Metrics', icon: '📊' },
]
</script>

<template>
  <aside 
    :class="[
      'bg-card border-r border-border transition-all duration-200 flex flex-col',
      open ? 'w-64' : 'w-16'
    ]"
  >
    <div class="p-4 border-b border-border flex items-center justify-between">
      <div v-if="open" class="flex items-center gap-2">
        <img src="/logo.svg" alt="AeroDB" class="w-8 h-8" />
        <h1 class="text-xl font-bold">AeroDB</h1>
      </div>
      <button
        @click="uiStore.toggleSidebar"
        class="p-2 rounded hover:bg-secondary"
        :title="open ? 'Collapse sidebar' : 'Expand sidebar'"
      >
        <span v-if="open">-</span>
        <span v-else">☰</span>
      </button>
    </div>
    
    <nav class="flex-1 p-4 space-y-2 overflow-y-auto">
      <RouterLink
        v-for="item in navItems"
        :key="item.to"
        :to="item.to"
        class="flex items-center gap-3 px-3 py-2 rounded hover:bg-secondary transition-colors"
        active-class="bg-secondary"
      >
        <span class="text-xl">{{ item.icon }}</span>
        <span v-if="open" class="text-sm">{{ item.label }}</span>
      </RouterLink>
    </nav>
  </aside>
</template>

<style scoped>
</style>
