<template>
    <div class="log-viewer">
        <div v-if="logs.length === 0" class="empty-state">
            <p>No logs to display</p>
        </div>
        <div v-else ref="logContainer" class="log-container" :class="{ 'auto-scroll': autoScroll }">
            <div
                v-for="(log, index) in displayedLogs"
                :key="index"
                class="log-entry"
                :class="`level-${log.level}`"
            >
                <span class="log-timestamp">{{ formatTimestamp(log.timestamp) }}</span>
                <span class="log-level">{{ log.level.toUpperCase() }}</span>
                <span v-if="log.module" class="log-module">{{ log.module }}</span>
                <span class="log-message">{{ log.message }}</span>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import type { LogEntry } from '@/types'

interface Props {
    logs: LogEntry[]
    autoScroll?: boolean
    filter?: {
        level?: 'debug' | 'info' | 'warn' | 'error'
        module?: string
        search?: string
    }
    maxLogs?: number
}

const props = withDefaults(defineProps<Props>(), {
    autoScroll: true,
    filter: () => ({}),
    maxLogs: 1000,
})

const logContainer = ref<HTMLElement>()

const displayedLogs = computed(() => {
    let filtered = props.logs

    // Apply filters
    if (props.filter?.level) {
        filtered = filtered.filter((log) => log.level === props.filter?.level)
    }
    if (props.filter?.module) {
        filtered = filtered.filter((log) => log.module === props.filter?.module)
    }
    if (props.filter?.search) {
        const search = props.filter.search.toLowerCase()
        filtered = filtered.filter((log) => log.message.toLowerCase().includes(search))
    }

    // Limit to max logs
    return filtered.slice(-props.maxLogs)
})

const formatTimestamp = (timestamp: string): string => {
    const date = new Date(timestamp)
    return date.toLocaleTimeString('en-US', {
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
    })
}

// Auto-scroll to bottom when new logs arrive
watch(
    () => props.logs.length,
    async () => {
        if (props.autoScroll && logContainer.value) {
            await nextTick()
            logContainer.value.scrollTop = logContainer.value.scrollHeight
        }
    }
)
</script>

<style scoped>
.log-viewer {
    display: flex;
    flex-direction: column;
    height: 100%;
    border: 1px solid var(--color-border);
    border-radius: 0.375rem;
    background: var(--color-bg-secondary);
    overflow: hidden;
}

.empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 200px;
    color: var(--color-text-muted);
}

.log-container {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
    font-family: 'Fira Code', 'Courier New', monospace;
    font-size: 0.75rem;
    line-height: 1.5;
}

.log-entry {
    display: flex;
    gap: 0.75rem;
    padding: 0.25rem 0.5rem;
    border-radius: 0.25rem;
}

.log-entry:hover {
    background: var(--color-bg-hover);
}

.log-timestamp {
    color: var(--color-text-muted);
    white-space: nowrap;
}

.log-level {
    font-weight: 600;
    min-width: 50px;
    white-space: nowrap;
}

.log-module {
    color: var(--color-primary);
    white-space: nowrap;
}

.log-message {
    flex: 1;
    word-break: break-word;
}

/* Level-specific colors */
.level-debug .log-level {
    color: #6b7280;
}

.level-info .log-level {
    color: #3b82f6;
}

.level-warn .log-level {
    color: #f59e0b;
}

.level-error .log-level {
    color: #ef4444;
}

/* Scrollbar styling */
.log-container::-webkit-scrollbar {
    width: 8px;
}

.log-container::-webkit-scrollbar-track {
    background: var(--color-bg-secondary);
}

.log-container::-webkit-scrollbar-thumb {
    background: var(--color-border);
    border-radius: 4px;
}

.log-container::-webkit-scrollbar-thumb:hover {
    background: var(--color-border-hover);
}
</style>
