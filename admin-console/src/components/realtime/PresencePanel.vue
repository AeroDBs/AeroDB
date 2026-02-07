<script setup lang="ts">
import { computed } from 'vue'
import type { PresenceUser } from '@/composables/useWebSocket'

const props = defineProps<{
    users: PresenceUser[]
    channel: string
}>()

const sortedUsers = computed(() => {
    return [...props.users].sort((a, b) => {
        // Online first, then by join time
        if (a.status !== b.status) {
            const order = { online: 0, away: 1, offline: 2 }
            return order[a.status] - order[b.status]
        }
        return new Date(b.online_at).getTime() - new Date(a.online_at).getTime()
    })
})

const getStatusColor = (status: string): string => {
    switch (status) {
        case 'online': return 'bg-green-500'
        case 'away': return 'bg-yellow-500'
        case 'offline': return 'bg-gray-500'
        default: return 'bg-gray-500'
    }
}

const formatTime = (timestamp: string): string => {
    const date = new Date(timestamp)
    const now = new Date()
    const diffMs = now.getTime() - date.getTime()
    const diffMins = Math.floor(diffMs / 60000)
    
    if (diffMins < 1) return 'Just now'
    if (diffMins < 60) return `${diffMins}m ago`
    if (diffMins < 1440) return `${Math.floor(diffMins / 60)}h ago`
    return date.toLocaleDateString()
}
</script>

<template>
    <div class="bg-card border border-border rounded-lg p-4">
        <div class="flex items-center justify-between mb-4">
            <h3 class="font-medium">Presence</h3>
            <span class="text-xs text-muted-foreground">{{ channel }}</span>
        </div>
        
        <div v-if="users.length === 0" class="text-center text-muted-foreground py-6">
            <p class="text-sm">No users in this channel</p>
        </div>
        
        <div v-else class="space-y-2">
            <div 
                v-for="user in sortedUsers" 
                :key="user.user_id"
                class="flex items-center gap-3 p-2 rounded-lg hover:bg-muted/50 transition-colors"
            >
                <!-- Avatar with status indicator -->
                <div class="relative">
                    <div class="w-8 h-8 rounded-full bg-muted flex items-center justify-center text-sm font-medium">
                        {{ (user.email || user.user_id).charAt(0).toUpperCase() }}
                    </div>
                    <div 
                        class="absolute -bottom-0.5 -right-0.5 w-3 h-3 rounded-full border-2 border-card"
                        :class="getStatusColor(user.status)"
                    ></div>
                </div>
                
                <!-- User info -->
                <div class="flex-1 min-w-0">
                    <p class="text-sm font-medium truncate">
                        {{ user.email || user.user_id.slice(0, 8) }}...
                    </p>
                    <p class="text-xs text-muted-foreground">
                        {{ formatTime(user.online_at) }}
                    </p>
                </div>
                
                <!-- Status badge -->
                <span 
                    class="text-xs px-2 py-0.5 rounded-full capitalize"
                    :class="{
                        'bg-green-500/10 text-green-500': user.status === 'online',
                        'bg-yellow-500/10 text-yellow-500': user.status === 'away',
                        'bg-gray-500/10 text-gray-500': user.status === 'offline',
                    }"
                >
                    {{ user.status }}
                </span>
            </div>
        </div>
        
        <div v-if="users.length > 0" class="mt-4 pt-4 border-t border-border">
            <div class="flex items-center justify-between text-xs text-muted-foreground">
                <span>{{ users.filter(u => u.status === 'online').length }} online</span>
                <span>{{ users.length }} total</span>
            </div>
        </div>
    </div>
</template>
