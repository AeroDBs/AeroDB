<template>
    <div class="function-logs-page">
        <div class="page-header">
            <button class="btn-back" @click="router.back()">← Back</button>
            <h1>{{ functionData?.name || 'Function' }} Logs</h1>
            <div class="header-actions">
                <select v-model="logFilter.level" class="filter-select">
                    <option value="">All Levels</option>
                    <option value="debug">Debug</option>
                    <option value="info">Info</option>
                    <option value="warn">Warning</option>
                    <option value="error">Error</option>
                </select>
                <button class="btn-secondary" @click="loadLogs">Refresh</button>
            </div>
        </div>

        <div v-if="loading" class="loading">Loading logs...</div>

        <div v-else-if="error" class="error-state">
            <p>{{ error }}</p>
        </div>

        <div v-else class="logs-container">
            <LogViewer :logs="logs" :filter="logFilter" :auto-scroll="true" />
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { functionsService } from '@/services'
import { getErrorMessage } from '@/composables/useApi'
import type { Function, FunctionLog, LogEntry } from '@/types'
import LogViewer from '@/components/common/LogViewer.vue'

const router = useRouter()
const route = useRoute()

const functionData = ref<Function | null>(null)
const logs = ref<LogEntry[]>([])
const loading = ref(false)
const error = ref('')

const logFilter = ref<{
    level?: 'debug' | 'info' | 'warn' | 'error'
    search?: string
}>({})

let pollInterval: ReturnType<typeof setInterval> | null = null

const loadFunction = async () => {
    try {
        const id = route.params.id as string
        functionData.value = await functionsService.getFunction(id)
    } catch (err) {
        error.value = getErrorMessage(err)
    }
}

const loadLogs = async () => {
    loading.value = true
    error.value = ''
    try {
        const id = route.params.id as string
        const result = await functionsService.getFunctionLogs(id, {
            limit: 500,
            level: logFilter.value.level,
        })
        
        // Convert FunctionLog to LogEntry format
        logs.value = result.logs.map((log: FunctionLog): LogEntry => ({
            timestamp: log.timestamp,
            level: log.level,
            module: 'function',
            message: log.message,
        }))
    } catch (err) {
        error.value = getErrorMessage(err)
    } finally {
        loading.value = false
    }
}

onMounted(() => {
    loadFunction()
    loadLogs()
    
    // Poll for new logs every 5 seconds
    pollInterval = setInterval(() => {
        loadLogs()
    }, 5000)
})

onUnmounted(() => {
    if (pollInterval) {
        clearInterval(pollInterval)
    }
})
</script>

<style scoped>
.function-logs-page {
    padding: 2rem;
    height: 100vh;
    display: flex;
    flex-direction: column;
}

.page-header {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 2rem;
}

.btn-back {
    padding: 0.5rem 1rem;
    border: 1px solid var(--color-border);
    border-radius: 0.375rem;
    background: transparent;
    color: var(--color-text);
    cursor: pointer;
}

.page-header h1 {
    flex: 1;
    font-size: 1.875rem;
    font-weight: 700;
    margin: 0;
}

.header-actions {
    display: flex;
    gap: 0.5rem;
}

.filter-select {
    padding: 0.5rem 1rem;
    border: 1px solid var(--color-border);
    border-radius: 0.375rem;
    background: var(--color-bg);
    color: var(--color-text);
}

.btn-secondary {
    padding: 0.5rem 1rem;
    border: 1px solid var(--color-border);
    border-radius: 0.375rem;
    background: transparent;
    color: var(--color-text);
    cursor: pointer;
}

.logs-container {
    flex: 1;
    overflow: hidden;
}

.loading,
.error-state {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 400px;
}
</style>
