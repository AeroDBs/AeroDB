<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { observabilityService } from '@/services/observability'

type Alert = {
    id: string
    name: string
    metric: string
    condition: 'gt' | 'lt' | 'eq'
    threshold: number
    duration_minutes: number
    enabled: boolean
    notification_channels: string[]
    last_triggered_at?: string
}

const alerts = ref<Alert[]>([])
const alertHistory = ref<Array<{
    alert_id: string
    alert_name: string
    triggered_at: string
    value: number
    resolved_at?: string
}>>([])

const loading = ref(false)
const showCreateDialog = ref(false)

// New alert form
const newAlert = ref({
    name: '',
    metric: '',
    condition: 'gt' as const,
    threshold: 0,
    duration_minutes: 5,
    notification_channels: [] as string[],
})

const metrics = ref<Array<{ name: string; description: string }>>([])
const channels = ['email', 'slack', 'webhook']

onMounted(async () => {
    loading.value = true
    try {
        const [alertsData, historyData, metricsData] = await Promise.all([
            observabilityService.getAlerts(),
            observabilityService.getAlertHistory({ limit: 20 }),
            observabilityService.getMetricNames(),
        ])
        alerts.value = alertsData
        alertHistory.value = historyData
        metrics.value = metricsData
    } finally {
        loading.value = false
    }
})

const createAlert = async () => {
    try {
        await observabilityService.createAlert(newAlert.value)
        alerts.value = await observabilityService.getAlerts()
        showCreateDialog.value = false
        resetForm()
    } catch (e) {
        console.error('Failed to create alert', e)
    }
}

const toggleAlert = async (alert: Alert) => {
    await observabilityService.updateAlert(alert.id, { enabled: !alert.enabled })
    alert.enabled = !alert.enabled
}

const deleteAlert = async (alertId: string) => {
    if (!confirm('Delete this alert?')) return
    await observabilityService.deleteAlert(alertId)
    alerts.value = alerts.value.filter(a => a.id !== alertId)
}

const resetForm = () => {
    newAlert.value = {
        name: '',
        metric: '',
        condition: 'gt',
        threshold: 0,
        duration_minutes: 5,
        notification_channels: [],
    }
}

const conditionLabels = {
    gt: 'Greater than',
    lt: 'Less than',
    eq: 'Equals',
}

const formatTimestamp = (ts: string) => new Date(ts).toLocaleString()
</script>

<template>
    <div class="h-full flex flex-col">
        <div class="flex items-center justify-between p-4 border-b border-border">
            <h2 class="font-semibold">Alert Rules</h2>
            <button
                @click="showCreateDialog = true"
                class="px-3 py-1 text-sm bg-primary text-primary-foreground rounded hover:opacity-90"
            >
                + New Alert
            </button>
        </div>
        
        <div class="flex-1 overflow-auto p-4 space-y-6">
            <!-- Alert Rules -->
            <div v-if="loading" class="text-center text-muted-foreground py-8">
                Loading alerts...
            </div>
            
            <div v-else-if="alerts.length === 0" class="text-center text-muted-foreground py-8">
                No alert rules configured
            </div>
            
            <div v-else class="space-y-3">
                <div
                    v-for="alert in alerts"
                    :key="alert.id"
                    class="p-4 bg-card border border-border rounded-lg"
                >
                    <div class="flex items-start justify-between">
                        <div>
                            <div class="flex items-center gap-2">
                                <h3 class="font-medium">{{ alert.name }}</h3>
                                <span
                                    class="text-xs px-2 py-0.5 rounded-full"
                                    :class="alert.enabled ? 'bg-green-500/10 text-green-500' : 'bg-muted text-muted-foreground'"
                                >
                                    {{ alert.enabled ? 'Active' : 'Disabled' }}
                                </span>
                            </div>
                            <p class="text-sm text-muted-foreground mt-1 font-mono">
                                {{ alert.metric }} {{ conditionLabels[alert.condition] }} {{ alert.threshold }}
                                for {{ alert.duration_minutes }}m
                            </p>
                            <p v-if="alert.last_triggered_at" class="text-xs text-muted-foreground mt-1">
                                Last triggered: {{ formatTimestamp(alert.last_triggered_at) }}
                            </p>
                        </div>
                        <div class="flex items-center gap-2">
                            <button
                                @click="toggleAlert(alert)"
                                class="text-sm px-2 py-1 border border-border rounded hover:bg-muted"
                            >
                                {{ alert.enabled ? 'Disable' : 'Enable' }}
                            </button>
                            <button
                                @click="deleteAlert(alert.id)"
                                class="text-sm px-2 py-1 text-destructive border border-destructive/30 rounded hover:bg-destructive/10"
                            >
                                Delete
                            </button>
                        </div>
                    </div>
                </div>
            </div>
            
            <!-- Alert History -->
            <div v-if="alertHistory.length > 0">
                <h3 class="font-medium mb-3">Recent Triggers</h3>
                <div class="space-y-2">
                    <div
                        v-for="event in alertHistory"
                        :key="`${event.alert_id}-${event.triggered_at}`"
                        class="flex items-center justify-between p-3 bg-muted/50 rounded-lg text-sm"
                    >
                        <div class="flex items-center gap-3">
                            <span :class="event.resolved_at ? 'text-green-500' : 'text-yellow-500'">
                                {{ event.resolved_at ? '✓' : '⚠️' }}
                            </span>
                            <span>{{ event.alert_name }}</span>
                            <span class="text-muted-foreground">Value: {{ event.value }}</span>
                        </div>
                        <span class="text-muted-foreground">
                            {{ formatTimestamp(event.triggered_at) }}
                        </span>
                    </div>
                </div>
            </div>
        </div>
        
        <!-- Create Alert Dialog -->
        <div
            v-if="showCreateDialog"
            class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
            @click="showCreateDialog = false"
        >
            <div class="bg-card border border-border rounded-lg p-6 max-w-md w-full mx-4" @click.stop>
                <h2 class="text-lg font-semibold mb-4">Create Alert Rule</h2>
                
                <form @submit.prevent="createAlert" class="space-y-4">
                    <div>
                        <label class="block text-sm font-medium mb-1">Name</label>
                        <input
                            v-model="newAlert.name"
                            type="text"
                            required
                            placeholder="High CPU Usage"
                            class="w-full px-3 py-2 bg-background border border-input rounded-md"
                        />
                    </div>
                    
                    <div>
                        <label class="block text-sm font-medium mb-1">Metric</label>
                        <select
                            v-model="newAlert.metric"
                            required
                            class="w-full px-3 py-2 bg-background border border-input rounded-md"
                        >
                            <option value="" disabled>Select metric</option>
                            <option v-for="m in metrics" :key="m.name" :value="m.name">
                                {{ m.name }} - {{ m.description }}
                            </option>
                        </select>
                    </div>
                    
                    <div class="grid grid-cols-2 gap-3">
                        <div>
                            <label class="block text-sm font-medium mb-1">Condition</label>
                            <select v-model="newAlert.condition" class="w-full px-3 py-2 bg-background border border-input rounded-md">
                                <option value="gt">Greater than</option>
                                <option value="lt">Less than</option>
                                <option value="eq">Equals</option>
                            </select>
                        </div>
                        <div>
                            <label class="block text-sm font-medium mb-1">Threshold</label>
                            <input
                                v-model.number="newAlert.threshold"
                                type="number"
                                required
                                class="w-full px-3 py-2 bg-background border border-input rounded-md"
                            />
                        </div>
                    </div>
                    
                    <div>
                        <label class="block text-sm font-medium mb-1">Duration (minutes)</label>
                        <input
                            v-model.number="newAlert.duration_minutes"
                            type="number"
                            min="1"
                            required
                            class="w-full px-3 py-2 bg-background border border-input rounded-md"
                        />
                    </div>
                    
                    <div>
                        <label class="block text-sm font-medium mb-1">Notification Channels</label>
                        <div class="flex gap-3">
                            <label v-for="ch in channels" :key="ch" class="flex items-center gap-1">
                                <input type="checkbox" :value="ch" v-model="newAlert.notification_channels" />
                                {{ ch }}
                            </label>
                        </div>
                    </div>
                    
                    <div class="flex justify-end gap-3 pt-2">
                        <button type="button" @click="showCreateDialog = false" class="px-4 py-2 border border-border rounded">
                            Cancel
                        </button>
                        <button type="submit" class="px-4 py-2 bg-primary text-primary-foreground rounded">
                            Create Alert
                        </button>
                    </div>
                </form>
            </div>
        </div>
    </div>
</template>
