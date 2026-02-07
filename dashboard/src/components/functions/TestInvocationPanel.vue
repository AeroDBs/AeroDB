<script setup lang="ts">
import { ref, computed } from 'vue'
import { functionsService } from '@/services'

const props = defineProps<{
    functionId: string
    functionName: string
}>()

const payload = ref('{}')
const isInvoking = ref(false)
const result = ref<{
    success: boolean
    result?: unknown
    error?: string
    duration_ms: number
} | null>(null)

const history = ref<Array<{
    timestamp: Date
    success: boolean
    duration_ms: number
    payload: string
}>>([])

const payloadError = computed(() => {
    try {
        JSON.parse(payload.value)
        return null
    } catch (e) {
        return 'Invalid JSON'
    }
})

const invokeFunction = async () => {
    if (payloadError.value) return
    
    isInvoking.value = true
    result.value = null
    
    const startTime = Date.now()
    
    try {
        const parsedPayload = JSON.parse(payload.value)
        const response = await functionsService.invokeFunction(props.functionId, parsedPayload)
        
        result.value = {
            success: true,
            result: response.result,
            duration_ms: response.duration_ms || (Date.now() - startTime),
        }
        
        history.value.unshift({
            timestamp: new Date(),
            success: true,
            duration_ms: response.duration_ms || (Date.now() - startTime),
            payload: payload.value,
        })
    } catch (e) {
        result.value = {
            success: false,
            error: String(e),
            duration_ms: Date.now() - startTime,
        }
        
        history.value.unshift({
            timestamp: new Date(),
            success: false,
            duration_ms: Date.now() - startTime,
            payload: payload.value,
        })
    } finally {
        isInvoking.value = false
    }
    
    // Keep only last 10 invocations
    if (history.value.length > 10) {
        history.value = history.value.slice(0, 10)
    }
}

const loadPayload = (payloadString: string) => {
    payload.value = payloadString
}

const formatResult = (data: unknown): string => {
    try {
        return JSON.stringify(data, null, 2)
    } catch {
        return String(data)
    }
}
</script>

<template>
    <div class="space-y-4">
        <h3 class="font-medium">Test Function</h3>
        
        <!-- Payload input -->
        <div>
            <div class="flex items-center justify-between mb-2">
                <label class="text-sm font-medium">Request Payload (JSON)</label>
                <span v-if="payloadError" class="text-xs text-destructive">{{ payloadError }}</span>
            </div>
            <textarea
                v-model="payload"
                rows="4"
                placeholder='{"key": "value"}'
                class="w-full px-3 py-2 bg-background border rounded-md font-mono text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                :class="payloadError ? 'border-destructive' : 'border-input'"
            ></textarea>
        </div>
        
        <!-- Invoke button -->
        <button
            @click="invokeFunction"
            :disabled="isInvoking || !!payloadError"
            class="w-full px-4 py-2 rounded bg-primary text-primary-foreground hover:opacity-90 disabled:opacity-50 flex items-center justify-center gap-2"
        >
            <span v-if="isInvoking" class="animate-spin">⟳</span>
            {{ isInvoking ? 'Invoking...' : 'Invoke Function' }}
        </button>
        
        <!-- Result -->
        <div v-if="result" class="p-4 rounded-lg" :class="result.success ? 'bg-green-500/10' : 'bg-destructive/10'">
            <div class="flex items-center justify-between mb-2">
                <span :class="result.success ? 'text-green-500' : 'text-destructive'" class="font-medium">
                    {{ result.success ? '✓ Success' : '✗ Error' }}
                </span>
                <span class="text-sm text-muted-foreground">{{ result.duration_ms }}ms</span>
            </div>
            <pre class="text-sm font-mono overflow-x-auto whitespace-pre-wrap max-h-64 overflow-y-auto">{{ result.success ? formatResult(result.result) : result.error }}</pre>
        </div>
        
        <!-- History -->
        <div v-if="history.length > 0" class="mt-6">
            <h4 class="text-sm font-medium text-muted-foreground mb-2">Recent Invocations</h4>
            <div class="space-y-2 max-h-48 overflow-y-auto">
                <div
                    v-for="(item, index) in history"
                    :key="index"
                    class="flex items-center justify-between p-2 bg-muted/50 rounded text-sm"
                >
                    <div class="flex items-center gap-2">
                        <span :class="item.success ? 'text-green-500' : 'text-destructive'">
                            {{ item.success ? '✓' : '✗' }}
                        </span>
                        <span class="text-muted-foreground">
                            {{ item.timestamp.toLocaleTimeString() }}
                        </span>
                        <span class="text-muted-foreground">
                            {{ item.duration_ms }}ms
                        </span>
                    </div>
                    <button
                        @click="loadPayload(item.payload)"
                        class="text-xs text-primary hover:underline"
                    >
                        Load Payload
                    </button>
                </div>
            </div>
        </div>
    </div>
</template>
