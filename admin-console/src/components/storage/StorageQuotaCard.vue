<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
    usedBytes: number
    limitBytes: number
    bucketsCount?: number
    filesCount?: number
}>()

const usagePercent = computed(() => {
    if (props.limitBytes === 0) return 0
    return Math.min(100, (props.usedBytes / props.limitBytes) * 100)
})

const usageLevel = computed(() => {
    if (usagePercent.value >= 90) return 'critical'
    if (usagePercent.value >= 75) return 'warning'
    return 'normal'
})

const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`
}
</script>

<template>
    <div class="bg-card border border-border rounded-lg p-4">
        <div class="flex items-center justify-between mb-4">
            <h3 class="font-medium">Storage Usage</h3>
            <span 
                class="text-xs px-2 py-1 rounded-full"
                :class="{
                    'bg-green-500/10 text-green-500': usageLevel === 'normal',
                    'bg-yellow-500/10 text-yellow-500': usageLevel === 'warning',
                    'bg-red-500/10 text-red-500': usageLevel === 'critical',
                }"
            >
                {{ usagePercent.toFixed(1) }}% used
            </span>
        </div>
        
        <!-- Progress bar -->
        <div class="h-3 bg-muted rounded-full overflow-hidden mb-3">
            <div 
                class="h-full transition-all duration-500"
                :class="{
                    'bg-green-500': usageLevel === 'normal',
                    'bg-yellow-500': usageLevel === 'warning',
                    'bg-red-500': usageLevel === 'critical',
                }"
                :style="{ width: `${usagePercent}%` }"
            ></div>
        </div>
        
        <!-- Details -->
        <div class="flex items-center justify-between text-sm">
            <span class="text-muted-foreground">
                {{ formatBytes(usedBytes) }} of {{ formatBytes(limitBytes) }}
            </span>
            <span class="text-muted-foreground">
                {{ formatBytes(limitBytes - usedBytes) }} remaining
            </span>
        </div>
        
        <!-- Stats -->
        <div v-if="bucketsCount !== undefined || filesCount !== undefined" class="grid grid-cols-2 gap-4 mt-4 pt-4 border-t border-border">
            <div v-if="bucketsCount !== undefined" class="text-center">
                <p class="text-2xl font-bold">{{ bucketsCount }}</p>
                <p class="text-sm text-muted-foreground">Buckets</p>
            </div>
            <div v-if="filesCount !== undefined" class="text-center">
                <p class="text-2xl font-bold">{{ filesCount.toLocaleString() }}</p>
                <p class="text-sm text-muted-foreground">Files</p>
            </div>
        </div>
    </div>
</template>
