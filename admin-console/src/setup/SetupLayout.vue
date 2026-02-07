<script setup lang="ts">
import { computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useSetupStore } from '@/stores/setup'

const router = useRouter()
const route = useRoute()
const setupStore = useSetupStore()

// Current step based on route
const currentStepIndex = computed(() => {
  const stepPath = route.path
  const index = setupStore.steps.findIndex(s => s.path === stepPath)
  return index >= 0 ? index : 0
})

// Progress percentage
const progressPercent = computed(() => {
  return ((currentStepIndex.value + 1) / setupStore.steps.length) * 100
})

// Navigation
const canGoBack = computed(() => currentStepIndex.value > 0)
const canGoNext = computed(() => currentStepIndex.value < setupStore.steps.length - 1)

const goBack = () => {
  if (canGoBack.value) {
    const prevStep = setupStore.steps[currentStepIndex.value - 1]
    router.push(prevStep.path)
  }
}

const goNext = () => {
  if (canGoNext.value) {
    const nextStep = setupStore.steps[currentStepIndex.value + 1]
    router.push(nextStep.path)
  }
}
</script>

<template>
  <div class="min-h-screen flex flex-col bg-background">
    <!-- Header -->
    <header class="border-b border-border bg-card py-4">
      <div class="max-w-4xl mx-auto px-6 flex items-center gap-4">
        <div class="w-10 h-10 bg-primary/10 rounded-lg flex items-center justify-center">
          <span class="text-2xl">✈️</span>
        </div>
        <div>
          <h1 class="text-xl font-bold text-foreground">AeroDB Setup</h1>
          <p class="text-sm text-muted-foreground">First-time configuration wizard</p>
        </div>
      </div>
    </header>

    <!-- Progress Bar -->
    <div class="bg-card border-b border-border">
      <div class="max-w-4xl mx-auto px-6 py-4">
        <!-- Step indicators -->
        <div class="flex items-center justify-between mb-2">
          <template v-for="(step, index) in setupStore.steps" :key="step.id">
            <div class="flex items-center">
              <div
                class="w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium transition-colors"
                :class="{
                  'bg-primary text-primary-foreground': index <= currentStepIndex,
                  'bg-muted text-muted-foreground': index > currentStepIndex
                }"
              >
                {{ step.id }}
              </div>
              <span
                class="ml-2 text-sm hidden sm:inline"
                :class="{
                  'text-foreground font-medium': index === currentStepIndex,
                  'text-muted-foreground': index !== currentStepIndex
                }"
              >
                {{ step.name }}
              </span>
            </div>
            <div
              v-if="index < setupStore.steps.length - 1"
              class="flex-1 h-0.5 mx-2"
              :class="{
                'bg-primary': index < currentStepIndex,
                'bg-muted': index >= currentStepIndex
              }"
            />
          </template>
        </div>
        <!-- Progress bar -->
        <div class="h-1 bg-muted rounded-full overflow-hidden">
          <div
            class="h-full bg-primary transition-all duration-300 ease-out"
            :style="{ width: `${progressPercent}%` }"
          />
        </div>
      </div>
    </div>

    <!-- Error Display -->
    <div v-if="setupStore.error" class="bg-destructive/10 border-b border-destructive/20">
      <div class="max-w-4xl mx-auto px-6 py-3 flex items-center justify-between">
        <div class="flex items-center gap-2 text-destructive">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="10"/>
            <line x1="12" x2="12" y1="8" y2="12"/>
            <line x1="12" x2="12.01" y1="16" y2="16"/>
          </svg>
          <span class="text-sm font-medium">{{ setupStore.error }}</span>
        </div>
        <button
          @click="setupStore.clearError()"
          class="text-destructive hover:text-destructive/80"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M18 6 6 18"/>
            <path d="m6 6 12 12"/>
          </svg>
        </button>
      </div>
    </div>

    <!-- Main Content -->
    <main class="flex-1 py-8">
      <div class="max-w-4xl mx-auto px-6">
        <RouterView v-slot="{ Component }">
          <transition name="fade" mode="out-in">
            <component :is="Component" @next="goNext" @back="goBack" />
          </transition>
        </RouterView>
      </div>
    </main>

    <!-- Footer -->
    <footer class="border-t border-border bg-card py-4">
      <div class="max-w-4xl mx-auto px-6 flex items-center justify-between">
        <p class="text-sm text-muted-foreground">
          Step {{ currentStepIndex + 1 }} of {{ setupStore.steps.length }}
        </p>
        <div class="flex items-center gap-3">
          <button
            v-if="canGoBack && currentStepIndex !== 5"
            @click="goBack"
            class="inline-flex items-center justify-center rounded-md text-sm font-medium h-9 px-4 border border-input bg-background hover:bg-accent hover:text-accent-foreground"
          >
            Back
          </button>
          <!-- Next button is handled by individual step components -->
        </div>
      </div>
    </footer>
  </div>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
