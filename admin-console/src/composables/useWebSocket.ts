import { ref, onUnmounted } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { realtimeService } from '@/services/realtime'

export interface Message {
    id: string
    channel: string
    event: string
    payload: unknown
    timestamp: string
}

export interface PresenceUser {
    user_id: string
    email?: string
    online_at: string
    status: 'online' | 'away' | 'offline'
}

export interface ChannelSubscription {
    channel: string
    filter?: Record<string, unknown>
}

export function useWebSocket(autoConnect = false) {
    const authStore = useAuthStore()

    const ws = ref<WebSocket | null>(null)
    const connected = ref(false)
    const connecting = ref(false)
    const messages = ref<Message[]>([])
    const events = ref<Array<{ type: string; payload: unknown; timestamp: string }>>([])
    const subscriptions = ref<ChannelSubscription[]>([])
    const presence = ref<Map<string, PresenceUser[]>>(new Map())
    const error = ref<string | null>(null)
    const reconnectAttempts = ref(0)
    const maxReconnectAttempts = 5

    let reconnectTimeout: ReturnType<typeof setTimeout> | null = null
    let pingInterval: ReturnType<typeof setInterval> | null = null

    const connect = () => {
        if (ws.value?.readyState === WebSocket.OPEN || connecting.value) {
            return
        }

        connecting.value = true
        error.value = null

        const wsUrl = realtimeService.getWebSocketUrl()
        const token = authStore.accessToken

        try {
            // Append auth token as query param
            const url = token ? `${wsUrl}?token=${token}` : wsUrl
            ws.value = new WebSocket(url)

            ws.value.onopen = () => {
                connected.value = true
                connecting.value = false
                reconnectAttempts.value = 0
                error.value = null

                // Start ping interval
                pingInterval = setInterval(() => {
                    if (ws.value?.readyState === WebSocket.OPEN) {
                        ws.value.send(JSON.stringify({ type: 'ping' }))
                    }
                }, 30000)

                // Resubscribe to channels
                subscriptions.value.forEach(sub => {
                    sendSubscribe(sub.channel, sub.filter)
                })
            }

            ws.value.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data)
                    handleMessage(data)

                    // Also store in events for backward compatibility
                    events.value.unshift({
                        ...data,
                        timestamp: new Date().toISOString(),
                    })
                    if (events.value.length > 100) {
                        events.value = events.value.slice(0, 100)
                    }
                } catch (e) {
                    console.error('Failed to parse WebSocket message:', e)
                }
            }

            ws.value.onerror = () => {
                error.value = 'WebSocket connection error'
            }

            ws.value.onclose = (event) => {
                connected.value = false
                connecting.value = false

                if (pingInterval) {
                    clearInterval(pingInterval)
                    pingInterval = null
                }

                // Attempt reconnection
                if (reconnectAttempts.value < maxReconnectAttempts && !event.wasClean) {
                    const delay = Math.min(1000 * Math.pow(2, reconnectAttempts.value), 30000)
                    reconnectAttempts.value++
                    reconnectTimeout = setTimeout(connect, delay)
                }
            }
        } catch (e) {
            connecting.value = false
            error.value = String(e)
        }
    }

    const disconnect = () => {
        if (reconnectTimeout) {
            clearTimeout(reconnectTimeout)
            reconnectTimeout = null
        }
        if (pingInterval) {
            clearInterval(pingInterval)
            pingInterval = null
        }
        if (ws.value) {
            ws.value.close()
            ws.value = null
        }
        connected.value = false
        reconnectAttempts.value = maxReconnectAttempts // Prevent auto-reconnect
    }

    const handleMessage = (data: { type: string; channel?: string; event?: string; payload?: unknown; users?: PresenceUser[] }) => {
        switch (data.type) {
            case 'pong':
                // Heartbeat response
                break

            case 'message':
            case 'broadcast':
            case 'db_change':
                if (data.channel && data.event) {
                    const msg: Message = {
                        id: `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
                        channel: data.channel,
                        event: data.event,
                        payload: data.payload,
                        timestamp: new Date().toISOString(),
                    }
                    messages.value = [msg, ...messages.value].slice(0, 100)
                }
                break

            case 'presence_state':
                if (data.channel && data.users) {
                    presence.value.set(data.channel, data.users)
                }
                break

            case 'presence_join':
            case 'presence_leave':
                if (data.channel) {
                    sendCommand('presence_state', { channel: data.channel })
                }
                break

            case 'subscribed':
                break

            case 'error':
                error.value = String(data.payload || 'Unknown error')
                break
        }
    }

    const sendCommand = (type: string, payload: Record<string, unknown>) => {
        if (ws.value?.readyState === WebSocket.OPEN) {
            ws.value.send(JSON.stringify({ type, ...payload }))
        }
    }

    const sendSubscribe = (channel: string, filter?: Record<string, unknown>) => {
        sendCommand('subscribe', { channel, filter })
    }

    const subscribe = (channel: string, filter?: Record<string, unknown>) => {
        const existing = subscriptions.value.find(s => s.channel === channel)
        if (!existing) {
            subscriptions.value.push({ channel, filter })
        }

        if (connected.value) {
            sendSubscribe(channel, filter)
        }
    }

    const unsubscribe = (channel: string) => {
        subscriptions.value = subscriptions.value.filter(s => s.channel !== channel)
        sendCommand('unsubscribe', { channel })
        presence.value.delete(channel)
    }

    const send = (channel: string, event: string, payload: unknown) => {
        sendCommand('broadcast', { channel, event, payload })
    }

    const trackPresence = (channel: string, metadata?: Record<string, unknown>) => {
        sendCommand('track', { channel, ...metadata })
    }

    const clearMessages = () => {
        messages.value = []
        events.value = []
    }

    const getPresence = (channel: string): PresenceUser[] => {
        return presence.value.get(channel) || []
    }

    // Auto connect if specified
    if (autoConnect) {
        connect()
    }

    // Cleanup on unmount
    onUnmounted(() => {
        disconnect()
    })

    return {
        connected,
        connecting,
        messages,
        events,
        subscriptions,
        presence,
        error,
        reconnectAttempts,
        connect,
        disconnect,
        subscribe,
        unsubscribe,
        send,
        trackPresence,
        clearMessages,
        getPresence,
    }
}
