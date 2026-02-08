<template>
    <div class="min-h-screen bg-background p-8">
        <div class="max-w-4xl mx-auto space-y-8">
            <!-- Header -->
            <div>
                <h1 class="text-3xl font-bold text-foreground">Settings</h1>
                <p class="text-sm text-muted-foreground mt-2">
                    Configure runtime settings for AeroDB
                </p>
            </div>

            <!-- Loading State -->
            <div v-if="loading" class="flex items-center justify-center p-12">
                <div class="animate-spin h-8 w-8 border-4 border-primary border-t-transparent rounded-full"></div>
            </div>

            <!-- Error State -->
            <div v-if="error" class="bg-red-500/10 border border-red-500 rounded-lg p-4">
                <p class="text-red-500">{{ error }}</p>
            </div>

            <!-- Settings Sections -->
            <div v-if="!loading && settings" class="space-y-8">
                <!-- Observability Settings -->
                <section class="bg-card border border-border rounded-lg p-6 space-y-6">
                    <div>
                        <h2 class="text-2xl font-semibold text-foreground">Observability Settings</h2>
                        <p class="text-sm text-muted-foreground mt-1">
                            Configure slow query tracking and operation logging
                        </p>
                    </div>

                    <!-- Slow Query Configuration -->
                    <div class="space-y-4">
                        <h3 class="text-lg font-medium text-foreground">Slow Query Tracking</h3>

                        <!-- Enabled Toggle -->
                        <div class="flex items-center space-x-3">
                            <input
                                v-model="settings.observability.slow_query.enabled"
                                type="checkbox"
                                id="slow-query-enabled"
                                class="h-4 w-4 rounded border-border bg-input"
                            />
                            <label for="slow-query-enabled" class="text-sm text-foreground cursor-pointer">
                                Enable slow query tracking
                            </label>
                        </div>

                        <!-- Threshold -->
                        <div class="grid grid-cols-2 gap-4">
                            <div>
                                <label for="threshold" class="block text-sm font-medium text-foreground mb-2">
                                    Threshold (ms)
                                </label>
                                <input
                                    v-model.number="settings.observability.slow_query.threshold_ms"
                                    type="number"
                                    id="threshold"
                                    min="1"
                                    max="60000"
                                    class="w-full px-3 py-2 bg-input border border-border rounded-md text-foreground"
                                />
                                <p class="text-xs text-muted-foreground mt-1">
                                    Queries exceeding this duration are considered slow
                                </p>
                            </div>

                            <div>
                                <label for="webhook-timeout" class="block text-sm font-medium text-foreground mb-2">
                                    Webhook Timeout (ms)
                                </label>
                                <input
                                    v-model.number="settings.observability.slow_query.webhook_timeout_ms"
                                    type="number"
                                    id="webhook-timeout"
                                    min="100"
                                    max="30000"
                                    class="w-full px-3 py-2 bg-input border border-border rounded-md text-foreground"
                                />
                            </div>
                        </div>

                        <!-- Emit Log Toggle -->
                        <div class="flex items-center space-x-3">
                            <input
                                v-model="settings.observability.slow_query.emit_log"
                                type="checkbox"
                                id="emit-log"
                                class="h-4 w-4 rounded border-border bg-input"
                            />
                            <label for="emit-log" class="text-sm text-foreground cursor-pointer">
                                Emit structured logs for slow queries
                            </label>
                        </div>

                        <!-- Webhook URL -->
                        <div>
                            <label for="webhook-url" class="block text-sm font-medium text-foreground mb-2">
                                Webhook URL (optional)
                            </label>
                            <input
                                v-model="settings.observability.slow_query.webhook_url"
                                type="url"
                                id="webhook-url"
                                placeholder="https://example.com/webhook"
                                class="w-full px-3 py-2 bg-input border border-border rounded-md text-foreground"
                            />
                            <p class="text-xs text-muted-foreground mt-1">
                                POST slow query events to this URL
                            </p>
                        </div>
                    </div>

                    <!-- Operation Log -->
                    <div class="space-y-4 pt-4 border-t border-border">
                        <h3 class="text-lg font-medium text-foreground">Operation Log</h3>
                        <div class="flex items-center space-x-3">
                            <input
                                v-model="settings.observability.operation_log_enabled"
                                type="checkbox"
                                id="operation-log"
                                class="h-4 w-4 rounded border-border bg-input"
                            />
                            <label for="operation-log" class="text-sm text-foreground cursor-pointer">
                                Enable operation logging (audit trail)
                            </label>
                        </div>
                    </div>

                    <!-- Save Button -->
                    <div class="flex justify-end pt-4">
                        <button
                            @click="saveObservabilitySettings"
                            :disabled="savingObservability"
                            class="px-6 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed"
                        >
                            {{ savingObservability ? 'Saving...' : 'Save Observability Settings' }}
                        </button>
                    </div>
                </section>

                <!-- Realtime Settings -->
                <section class="bg-card border border-border rounded-lg p-6 space-y-6">
                    <div>
                        <h2 class="text-2xl font-semibold text-foreground">Realtime Settings</h2>
                        <p class="text-sm text-muted-foreground mt-1">
                            Configure backpressure and message delivery policies
                        </p>
                    </div>

                    <!-- Backpressure Configuration -->
                    <div class="space-y-4">
                        <h3 class="text-lg font-medium text-foreground">Backpressure</h3>

                        <!-- Max Pending Messages -->
                        <div>
                            <label for="max-pending" class="block text-sm font-medium text-foreground mb-2">
                                Max Pending Messages
                            </label>
                            <input
                                v-model.number="settings.realtime.backpressure.max_pending_messages"
                                type="number"
                                id="max-pending"
                                min="1"
                                max="1000000"
                                class="w-full px-3 py-2 bg-input border border-border rounded-md text-foreground"
                            />
                            <p class="text-xs text-muted-foreground mt-1">
                                Maximum number of pending messages before backpressure kicks in
                            </p>
                        </div>

                        <!-- Drop Policy -->
                        <div>
                            <label for="drop-policy" class="block text-sm font-medium text-foreground mb-2">
                                Drop Policy
                            </label>
                            <select
                                v-model="settings.realtime.backpressure.drop_policy"
                                id="drop-policy"
                                class="w-full px-3 py-2 bg-input border border-border rounded-md text-foreground"
                            >
                                <option value="oldest_first">Oldest First - Drop oldest message when full</option>
                                <option value="newest_first">Newest First - Reject new messages when full</option>
                                <option value="reject">Reject - Return error to sender when full</option>
                            </select>
                            <p class="text-xs text-muted-foreground mt-1">
                                How to handle messages when buffer is full
                            </p>
                        </div>
                    </div>

                    <!-- Save Button -->
                    <div class="flex justify-end pt-4">
                        <button
                            @click="saveRealtimeSettings"
                            :disabled="savingRealtime"
                            class="px-6 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed"
                        >
                            {{ savingRealtime ? 'Saving...' : 'Save Realtime Settings' }}
                        </button>
                    </div>
                </section>

                <!-- Success Message -->
                <div
                    v-if="successMessage"
                    class="bg-green-500/10 border border-green-500 rounded-lg p-4 flex items-center justify-between"
                >
                    <p class="text-green-500">{{ successMessage }}</p>
                    <button @click="successMessage = ''" class="text-green-500 hover:text-green-400">
                        ✕
                    </button>
                </div>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { settingsService, type Settings } from '@/services/settings'

const loading = ref(true)
const error = ref('')
const successMessage = ref('')
const settings = ref<Settings | null>(null)
const savingObservability = ref(false)
const savingRealtime = ref(false)

onMounted(async () => {
    try {
        loading.value = true
        settings.value = await settingsService.getSettings()
    } catch (err: any) {
        error.value = err.response?.data?.error || 'Failed to load settings'
    } finally {
        loading.value = false
    }
})

const validateObservabilitySettings = (): string | null => {
    if (!settings.value) return 'Settings not loaded'
    
    const { slow_query } = settings.value.observability
    
    if (slow_query.threshold_ms < 1 || slow_query.threshold_ms > 60000) {
        return 'Slow query threshold must be between 1ms and 60000ms'
    }
    
    if (slow_query.webhook_url && !slow_query.webhook_url.match(/^https?:\/\//)) {
        return 'Webhook URL must start with http:// or https://'
    }
    
    if (slow_query.webhook_timeout_ms < 100 || slow_query.webhook_timeout_ms > 30000) {
        return 'Webhook timeout must be between 100ms and 30000ms'
    }
    
    return null
}

const validateRealtimeSettings = (): string | null => {
    if (!settings.value) return 'Settings not loaded'
    
    const { backpressure } = settings.value.realtime
    
    if (backpressure.max_pending_messages < 1 || backpressure.max_pending_messages > 1000000) {
        return 'Max pending messages must be between 1 and 1,000,000'
    }
    
    return null
}

const saveObservabilitySettings = async () => {
    error.value = ''
    successMessage.value = ''
    
    const validationError = validateObservabilitySettings()
    if (validationError) {
        error.value = validationError
        return
    }
    
    try {
        savingObservability.value = true
        if (!settings.value) return
        
        await settingsService.updateObservabilitySettings(settings.value.observability)
        successMessage.value = 'Observability settings saved successfully'
        
        // Clear success message after 3 seconds
        setTimeout(() => {
            successMessage.value = ''
        }, 3000)
    } catch (err: any) {
        error.value = err.response?.data?.error || 'Failed to save observability settings'
    } finally {
        savingObservability.value = false
    }
}

const saveRealtimeSettings = async () => {
    error.value = ''
    successMessage.value = ''
    
    const validationError = validateRealtimeSettings()
    if (validationError) {
        error.value = validationError
        return
    }
    
    try {
        savingRealtime.value = true
        if (!settings.value) return
        
        await settingsService.updateRealtimeSettings(settings.value.realtime)
        successMessage.value = 'Realtime settings saved successfully'
        
        // Clear success message after 3 seconds
        setTimeout(() => {
            successMessage.value = ''
        }, 3000)
    } catch (err: any) {
        error.value = err.response?.data?.error || 'Failed to save realtime settings'
    } finally {
        savingRealtime.value = false
    }
}
</script>
