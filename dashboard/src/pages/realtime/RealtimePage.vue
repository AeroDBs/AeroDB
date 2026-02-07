<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useQuery, useMutation, useQueryClient } from '@tanstack/vue-query'
import AppLayout from '@/components/layout/AppLayout.vue'
import PresencePanel from '@/components/realtime/PresencePanel.vue'
import MessageInspector from '@/components/realtime/MessageInspector.vue'
import { realtimeService } from '@/services/realtime'
import { useWebSocket } from '@/composables/useWebSocket'

const queryClient = useQueryClient()

// WebSocket state
const { 
    connected, 
    connecting,
    messages, 
    subscriptions: wsSubscriptions,
    error: wsError, 
    reconnectAttempts,
    connect, 
    disconnect,
    subscribe,
    unsubscribe,
    send,
    trackPresence,
    clearMessages,
    getPresence,
} = useWebSocket()

// UI State
const activeTab = ref<'subscriptions' | 'broadcast' | 'presence'>('subscriptions')
const newChannelName = ref('')
const newChannelFilter = ref('')
const broadcastChannel = ref('')
const broadcastEvent = ref('')
const broadcastPayload = ref('{}')
const selectedChannel = ref<string | null>(null)

// Fetch active subscriptions from server
const { data: serverSubscriptions, isLoading, error } = useQuery({
    queryKey: ['subscriptions'],
    queryFn: () => realtimeService.getSubscriptions(),
    refetchInterval: 10000, // Refresh every 10s
})

// Disconnect subscription mutation
const disconnectSubMutation = useMutation({
    mutationFn: (subscriptionId: string) => realtimeService.disconnectSubscription(subscriptionId),
    onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: ['subscriptions'] })
    },
})

// Broadcast mutation
const broadcastMutation = useMutation({
    mutationFn: (data: { channel: string; event: string; payload: unknown }) => 
        realtimeService.broadcast(data.channel, data.event, data.payload),
})

// Toggle connection
const toggleConnection = () => {
    if (connected.value || connecting.value) {
        disconnect()
    } else {
        connect()
    }
}

// Subscribe to a channel
const handleSubscribe = () => {
    if (!newChannelName.value.trim()) return
    
    let filter: Record<string, unknown> | undefined
    if (newChannelFilter.value.trim()) {
        try {
            filter = JSON.parse(newChannelFilter.value)
        } catch {
            // Invalid JSON, ignore filter
        }
    }
    
    subscribe(newChannelName.value, filter)
    trackPresence(newChannelName.value)
    selectedChannel.value = newChannelName.value
    newChannelName.value = ''
    newChannelFilter.value = ''
}

// Unsubscribe from a channel
const handleUnsubscribe = (channel: string) => {
    unsubscribe(channel)
    if (selectedChannel.value === channel) {
        selectedChannel.value = wsSubscriptions.value[0]?.channel || null
    }
}

// Send broadcast
const handleBroadcast = () => {
    if (!broadcastChannel.value || !broadcastEvent.value) return
    
    let payload: unknown = {}
    try {
        payload = JSON.parse(broadcastPayload.value)
    } catch {
        payload = { message: broadcastPayload.value }
    }
    
    // Use WebSocket if connected, otherwise HTTP
    if (connected.value) {
        send(broadcastChannel.value, broadcastEvent.value, payload)
    } else {
        broadcastMutation.mutate({
            channel: broadcastChannel.value,
            event: broadcastEvent.value,
            payload,
        })
    }
}

// Presence for selected channel
const presenceUsers = computed(() => {
    if (!selectedChannel.value) return []
    return getPresence(selectedChannel.value)
})

// Auto-connect on mount
onMounted(() => {
    connect()
})


onUnmounted(() => {
    disconnect()
})

const formatFilter = (filter: unknown): string => {
    if (!filter) return '-'
    try {
        return JSON.stringify(filter)
    } catch {
        return String(filter)
    }
}
</script>

<template>
    <AppLayout>
        <div class="space-y-6">
            <!-- Header -->
            <div class="flex items-center justify-between">
                <h1 class="text-3xl font-bold">Realtime</h1>
                <div class="flex items-center gap-4">
                    <!-- Connection status -->
                    <div 
                        :class="[
                            'px-3 py-1 rounded-full text-sm flex items-center gap-2',
                            connected ? 'bg-green-500/20 text-green-400' : 
                            connecting ? 'bg-yellow-500/20 text-yellow-400' :
                            'bg-red-500/20 text-red-400'
                        ]"
                    >
                        <span v-if="connecting" class="animate-pulse">●</span>
                        <span v-else>{{ connected ? '●' : '○' }}</span>
                        {{ connecting ? 'Connecting...' : connected ? 'Connected' : 'Disconnected' }}
                        <span v-if="reconnectAttempts > 0" class="text-xs">
                            (retry {{ reconnectAttempts }})
                        </span>
                    </div>
                    
                    <button
                        @click="toggleConnection"
                        :class="[
                            'px-4 py-2 rounded font-medium transition-colors',
                            connected || connecting
                                ? 'bg-destructive text-destructive-foreground hover:opacity-90'
                                : 'bg-primary text-primary-foreground hover:opacity-90'
                        ]"
                    >
                        {{ connected || connecting ? 'Disconnect' : 'Connect' }}
                    </button>
                </div>
            </div>

            <!-- Error banner -->
            <div v-if="wsError" class="p-4 bg-destructive/10 text-destructive rounded-lg flex items-center gap-2">
                <span>⚠️</span>
                <span>{{ wsError }}</span>
            </div>

            <!-- Tabs -->
            <div class="border-b border-border">
                <nav class="flex gap-4">
                    <button 
                        @click="activeTab = 'subscriptions'"
                        class="px-4 py-2 font-medium transition-colors border-b-2"
                        :class="activeTab === 'subscriptions' ? 'border-primary text-primary' : 'border-transparent text-muted-foreground hover:text-foreground'"
                    >
                        Subscriptions
                    </button>
                    <button 
                        @click="activeTab = 'broadcast'"
                        class="px-4 py-2 font-medium transition-colors border-b-2"
                        :class="activeTab === 'broadcast' ? 'border-primary text-primary' : 'border-transparent text-muted-foreground hover:text-foreground'"
                    >
                        Broadcast
                    </button>
                    <button 
                        @click="activeTab = 'presence'"
                        class="px-4 py-2 font-medium transition-colors border-b-2"
                        :class="activeTab === 'presence' ? 'border-primary text-primary' : 'border-transparent text-muted-foreground hover:text-foreground'"
                    >
                        Presence
                    </button>
                </nav>
            </div>

            <!-- Subscriptions Tab -->
            <div v-if="activeTab === 'subscriptions'" class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                <!-- Left: Channel Management -->
                <div class="space-y-4">
                    <!-- Subscribe to new channel -->
                    <div class="bg-card border border-border rounded-lg p-4">
                        <h3 class="font-medium mb-4">Subscribe to Channel</h3>
                        <form @submit.prevent="handleSubscribe" class="space-y-3">
                            <div>
                                <label class="block text-sm text-muted-foreground mb-1">Channel Name</label>
                                <input
                                    v-model="newChannelName"
                                    type="text"
                                    placeholder="e.g., public:messages or users:123"
                                    class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring font-mono"
                                />
                            </div>
                            <div>
                                <label class="block text-sm text-muted-foreground mb-1">Filter (optional JSON)</label>
                                <input
                                    v-model="newChannelFilter"
                                    type="text"
                                    placeholder='{"table": "posts", "event": "INSERT"}'
                                    class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring font-mono text-sm"
                                />
                            </div>
                            <button
                                type="submit"
                                :disabled="!newChannelName.trim() || !connected"
                                class="w-full px-4 py-2 rounded bg-primary text-primary-foreground hover:opacity-90 disabled:opacity-50"
                            >
                                Subscribe
                            </button>
                        </form>
                    </div>

                    <!-- Local subscriptions -->
                    <div class="bg-card border border-border rounded-lg p-4">
                        <h3 class="font-medium mb-4">Your Subscriptions</h3>
                        <div v-if="wsSubscriptions.length === 0" class="text-center py-4 text-muted-foreground">
                            No active subscriptions
                        </div>
                        <div v-else class="space-y-2">
                            <div 
                                v-for="sub in wsSubscriptions" 
                                :key="sub.channel"
                                class="flex items-center justify-between p-3 rounded-lg border border-border bg-background"
                                :class="{ 'ring-2 ring-primary': selectedChannel === sub.channel }"
                            >
                                <div @click="selectedChannel = sub.channel" class="flex-1 cursor-pointer">
                                    <p class="font-mono text-sm">{{ sub.channel }}</p>
                                    <p v-if="sub.filter" class="text-xs text-muted-foreground">
                                        {{ formatFilter(sub.filter) }}
                                    </p>
                                </div>
                                <button
                                    @click="handleUnsubscribe(sub.channel)"
                                    class="text-xs text-destructive hover:underline"
                                >
                                    Unsubscribe
                                </button>
                            </div>
                        </div>
                    </div>

                    <!-- Server-side subscriptions -->
                    <div class="bg-card border border-border rounded-lg p-4">
                        <h3 class="font-medium mb-4">All Active Connections</h3>
                        <div v-if="isLoading" class="text-center py-4 text-muted-foreground">
                            Loading...
                        </div>
                        <div v-else-if="error" class="text-destructive text-sm">
                            Failed to load subscriptions
                        </div>
                        <div v-else-if="!serverSubscriptions?.length" class="text-center py-4 text-muted-foreground">
                            No active connections
                        </div>
                        <div v-else class="overflow-x-auto">
                            <table class="w-full text-sm">
                                <thead>
                                    <tr class="text-left text-muted-foreground">
                                        <th class="pb-2">User</th>
                                        <th class="pb-2">Channel</th>
                                        <th class="pb-2"></th>
                                    </tr>
                                </thead>
                                <tbody class="divide-y divide-border">
                                    <tr v-for="sub in serverSubscriptions" :key="sub.id">
                                        <td class="py-2 font-mono text-xs">{{ sub.user_id.slice(0, 8) }}...</td>
                                        <td class="py-2 font-mono">{{ sub.channel }}</td>
                                        <td class="py-2">
                                            <button
                                                @click="disconnectSubMutation.mutate(sub.id)"
                                                class="text-xs text-destructive hover:underline"
                                            >
                                                Disconnect
                                            </button>
                                        </td>
                                    </tr>
                                </tbody>
                            </table>
                        </div>
                    </div>
                </div>

                <!-- Right: Message Inspector -->
                <div class="h-[600px]">
                    <MessageInspector 
                        :messages="messages"
                        :filter="selectedChannel ? { channel: selectedChannel } : undefined"
                        @clear="clearMessages"
                    />
                </div>
            </div>

            <!-- Broadcast Tab -->
            <div v-if="activeTab === 'broadcast'" class="max-w-lg">
                <div class="bg-card border border-border rounded-lg p-6">
                    <h3 class="font-medium mb-4">Send Broadcast Message</h3>
                    <form @submit.prevent="handleBroadcast" class="space-y-4">
                        <div>
                            <label class="block text-sm font-medium mb-2">Channel</label>
                            <input
                                v-model="broadcastChannel"
                                type="text"
                                required
                                placeholder="public:announcements"
                                class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring font-mono"
                            />
                        </div>
                        <div>
                            <label class="block text-sm font-medium mb-2">Event Type</label>
                            <input
                                v-model="broadcastEvent"
                                type="text"
                                required
                                placeholder="new_message"
                                class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
                            />
                        </div>
                        <div>
                            <label class="block text-sm font-medium mb-2">Payload (JSON)</label>
                            <textarea
                                v-model="broadcastPayload"
                                rows="4"
                                placeholder='{"message": "Hello world!"}'
                                class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring font-mono text-sm"
                            ></textarea>
                        </div>
                        <button
                            type="submit"
                            :disabled="!broadcastChannel || !broadcastEvent"
                            class="w-full px-4 py-2 rounded bg-primary text-primary-foreground hover:opacity-90 disabled:opacity-50"
                        >
                            {{ broadcastMutation.isPending.value ? 'Sending...' : 'Send Broadcast' }}
                        </button>
                    </form>
                </div>
            </div>

            <!-- Presence Tab -->
            <div v-if="activeTab === 'presence'" class="grid grid-cols-1 lg:grid-cols-3 gap-6">
                <!-- Channel selector -->
                <div class="bg-card border border-border rounded-lg p-4">
                    <h3 class="font-medium mb-4">Channels</h3>
                    <div v-if="wsSubscriptions.length === 0" class="text-center py-4 text-muted-foreground text-sm">
                        Subscribe to a channel to view presence
                    </div>
                    <div v-else class="space-y-2">
                        <button
                            v-for="sub in wsSubscriptions"
                            :key="sub.channel"
                            @click="selectedChannel = sub.channel"
                            class="w-full p-3 text-left rounded-lg border transition-colors"
                            :class="selectedChannel === sub.channel ? 'border-primary bg-primary/5' : 'border-border hover:bg-muted/50'"
                        >
                            <span class="font-mono text-sm">{{ sub.channel }}</span>
                        </button>
                    </div>
                </div>

                <!-- Presence panel -->
                <div class="lg:col-span-2">
                    <PresencePanel
                        v-if="selectedChannel"
                        :users="presenceUsers"
                        :channel="selectedChannel"
                    />
                    <div v-else class="bg-card border border-border rounded-lg p-8 text-center text-muted-foreground">
                        Select a channel to view presence
                    </div>
                </div>
            </div>
        </div>
    </AppLayout>
</template>
