<script setup lang="ts">


const props = defineProps<{
  open: boolean
  title: string
  description: string
  actionLabel?: string
  variant?: 'default' | 'destructive'
}>()

const emit = defineEmits<{
  confirm: []
  cancel: []
  'update:open': [value: boolean]
}>()

const handleCancel = () => {
  emit('cancel')
  emit('update:open', false)
}

const handleConfirm = () => {
  emit('confirm')
  emit('update:open', false)
}
</script>

<template>
  <div
    v-if="open"
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
    @click="handleCancel"
  >
    <div
      class="bg-card border border-border rounded-lg shadow-lg max-w-md w-full mx-4 p-6"
      @click.stop
    >
      <h2 class="text-lg font-semibold mb-2">{{ title }}</h2>
      <p class="text-sm text-muted-foreground mb-6">{{ description }}</p>
      
      <div class="flex justify-end gap-3">
        <button
          @click="handleCancel"
          class="px-4 py-2 rounded border border-border hover:bg-secondary"
        >
          Cancel
        </button>
        <button
          @click="handleConfirm"
          :class="[
            'px-4 py-2 rounded',
            variant === 'destructive'
              ? 'bg-destructive text-destructive-foreground hover:opacity-90'
              : 'bg-primary text-primary-foreground hover:opacity-90'
          ]"
        >
          {{ actionLabel || 'Confirm' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
</style>
