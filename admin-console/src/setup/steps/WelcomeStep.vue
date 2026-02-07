<script setup lang="ts">
import { useRouter } from 'vue-router'

const router = useRouter()

const startSetup = () => {
  router.push('/setup/storage')
}
</script>

<template>
  <div class="text-center py-8">
    <!-- Hero Icon -->
    <div class="w-24 h-24 bg-primary/10 rounded-full flex items-center justify-center mx-auto mb-6">
      <span class="text-5xl">✈️</span>
    </div>

    <!-- Welcome Message -->
    <h1 class="text-3xl font-bold text-foreground mb-4">
      Welcome to AeroDB
    </h1>
    <p class="text-lg text-muted-foreground max-w-xl mx-auto mb-8">
      Let's configure your database in just a few steps. This wizard will guide you through
      setting up storage, authentication, and creating your admin account.
    </p>

    <!-- What We'll Configure -->
    <div class="bg-card border border-border rounded-lg p-6 max-w-lg mx-auto mb-8 text-left">
      <h3 class="font-semibold text-foreground mb-4">What we'll configure:</h3>
      <ul class="space-y-3">
        <li class="flex items-center gap-3 text-muted-foreground">
          <div class="w-6 h-6 bg-primary/10 rounded-full flex items-center justify-center flex-shrink-0">
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-primary">
              <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
            </svg>
          </div>
          <span>Storage directories for data, WAL, and snapshots</span>
        </li>
        <li class="flex items-center gap-3 text-muted-foreground">
          <div class="w-6 h-6 bg-primary/10 rounded-full flex items-center justify-center flex-shrink-0">
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-primary">
              <rect width="18" height="11" x="3" y="11" rx="2" ry="2"/>
              <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
            </svg>
          </div>
          <span>Authentication and security settings</span>
        </li>
        <li class="flex items-center gap-3 text-muted-foreground">
          <div class="w-6 h-6 bg-primary/10 rounded-full flex items-center justify-center flex-shrink-0">
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-primary">
              <path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2"/>
              <circle cx="12" cy="7" r="4"/>
            </svg>
          </div>
          <span>Your first administrator account</span>
        </li>
      </ul>
    </div>

    <!-- Time Estimate -->
    <p class="text-sm text-muted-foreground mb-6">
      ⏱️ This will take about 2-3 minutes
    </p>

    <!-- Start Button -->
    <button
      @click="startSetup"
      class="inline-flex items-center justify-center rounded-md text-sm font-medium ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 bg-primary text-primary-foreground hover:bg-primary/90 h-11 px-8 shadow-lg"
    >
      Let's Get Started
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="ml-2">
        <path d="M5 12h14"/>
        <path d="m12 5 7 7-7 7"/>
      </svg>
    </button>
  </div>
</template>
