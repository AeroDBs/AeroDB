<script setup lang="ts">
import { computed } from 'vue'
import type { Message } from '@/composables/useWebSocket'

const props = defineProps<{
    messages: Message[]
    filter?: {
        channel?: string
        event?: string
    }
}>()

const emit = defineEmits<{
    (e: 'clear'): void
}>()

const filteredMessages = computed(() => {
    let msgs = props.messages
    
    if (props.filter?.channel) {
        msgs = msgs.filter(m => m.channel.includes(props.filter!.channel!))
    }
    if (props.filter?.event) {
        msgs = msgs.filter(m => m.event.includes(props.filter!.event!))
    }
    
    return msgs
})

const formatTime = (timestamp: string): string => {
    const date = new Date(timestamp)
    return date.toLocaleTimeString()
}

const formatPayload = (payload: unknown): string => {
    try {
        return JSON.stringify(payload, null, 2)
    } catch {
        return String(payload)
    }
}

const getEventColor = (event: string): string => {
    if (event.includes('INSERT') || event.includes('create')) return 'text-green-500'
    if (event.includes('UPDATE') || event.includes('update')) return 'text-yellow-500'
    if (event.includes('DELETE') || event.includes('delete')) return 'text-red-500'
    return 'text-blue-500'
}
</script>

<template>
    <div class="bg-card border border-border rounded-lg flex flex-col h-full">
        <div class="flex items-center justify-between p-4 border-b border-border">
            <h3 class="font-medium">Message Inspector</h3>
            <div class="flex items-center gap-2">
                <span class="text-xs text-muted-foreground">
                    {{ filteredMessages.length }} messages
                </span>
                <button 
                    @click="emit('clear')"
                    class="text-xs text-muted-foreground hover:text-foreground"
                >
                    Clear
                </button>
            </div>
        </div>
        
        <div class="flex-1 overflow-auto p-2">
            <div v-if="filteredMessages.length === 0" class="text-center text-muted-foreground py-8">
                <p class="text-sm">No messages yet</p>
                <p class="text-xs mt-1">Subscribe to a channel to see real-time events</p>
            </div>
            
            <div v-else class="space-y-2">
                <div 
                    v-for="msg in filteredMessages" 
                    :key="msg.id"
                    class="border border-border rounded-lg overflow-hidden"
                >
                    <!-- Message header -->
                    <div class="flex items-center gap-2 px-3 py-2 bg-muted/30">
                        <span 
                            class="text-xs font-mono font-medium"
                            :class="getEventColor(msg.event)"
                        >
                            {{ msg.event }}
                        </span>
                        <span class="text-xs text-muted-foreground">•</span>
                        <span class="text-xs font-mono text-muted-foreground">
                            {{ msg.channel }}
                        </span>
                        <span class="flex-1"></span>
                        <span class="text-xs text-muted-foreground">
                            {{ formatTime(msg.timestamp) }}
                        </span>
                    </div>
                    
                    <!-- Message payload -->
                    <div class="px-3 py-2 bg-background">
                        <pre class="text-xs font-mono whitespace-pre-wrap overflow-x-auto max-h-32">{{ formatPayload(msg.payload) }}</pre>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>
