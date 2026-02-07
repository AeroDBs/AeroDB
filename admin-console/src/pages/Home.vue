<script setup lang="ts">
import AppLayout from '@/components/layout/AppLayout.vue'
</script>

<template>
  <AppLayout>
    <div class="space-y-6">
      <h1 class="text-3xl font-bold">Dashboard</h1>
      
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <div class="p-6 bg-card rounded-lg border border-border">
          <h3 class="text-lg font-semibold mb-2">Database</h3>
          <p class="text-sm text-muted-foreground">Browse tables and execute SQL queries</p>
        </div>
        
        <div class="p-6 bg-card rounded-lg border border-border">
          <h3 class="text-lg font-semibold mb-2">Authentication</h3>
          <p class="text-sm text-muted-foreground">Manage users and sessions</p>
        </div>
        
        <div class="p-6 bg-card rounded-lg border border-border">
          <h3 class="text-lg font-semibold mb-2">Storage</h3>
          <p class="text-sm text-muted-foreground">Browse files and buckets</p>
        </div>
        
        <div class="p-6 bg-card rounded-lg border border-border">
          <h3 class="text-lg font-semibold mb-2">Real-Time</h3>
          <p class="text-sm text-muted-foreground">Monitor subscriptions and events</p>
        </div>
        
        <div class="p-6 bg-card rounded-lg border border-border">
          <h3 class="text-lg font-semibold mb-2">Cluster</h3>
          <p class="text-sm text-muted-foreground">View topology and replication</p>
        </div>
        
        <div class="p-6 bg-card rounded-lg border border-border">
          <h3 class="text-lg font-semibold mb-2">Logs</h3>
          <p class="text-sm text-muted-foreground">View system and audit logs</p>
        </div>
        
        <div class="p-6 bg-card rounded-lg border border-border">
          <h3 class="text-lg font-semibold mb-2">Metrics</h3>
          <p class="text-sm text-muted-foreground">Performance and health metrics</p>
        </div>
      </div>
    </div>
  </AppLayout>
</template>

<style scoped>
</style>
