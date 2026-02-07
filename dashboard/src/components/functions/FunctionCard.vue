<template>
    <div class="function-card">
        <div class="card-header">
            <h3 class="function-name">{{ func.name }}</h3>
            <span class="function-status" :class="{ enabled: func.enabled }">
                {{ func.enabled ? 'Enabled' : 'Disabled' }}
            </span>
        </div>

        <div class="card-body">
            <div class="function-info">
                <span class="info-label">Runtime:</span>
                <span class="info-value">{{ func.runtime }}</span>
            </div>
            <div v-if="func.last_invoked_at" class="function-info">
                <span class="info-label">Last invoked:</span>
                <span class="info-value">{{ formatDate(func.last_invoked_at) }}</span>
            </div>
            <div class="function-info">
                <span class="info-label">Created:</span>
                <span class="info-value">{{ formatDate(func.created_at) }}</span>
            </div>
        </div>

        <div class="card-actions">
            <button class="btn-action" @click="emit('edit', func)">Edit</button>
            <button class="btn-action" @click="emit('invoke', func)">Invoke</button>
            <button class="btn-action" @click="emit('viewLogs', func)">Logs</button>
            <button class="btn-action danger" @click="emit('delete', func)">Delete</button>
        </div>
    </div>
</template>

<script setup lang="ts">
import type { Function } from '@/types'

interface Props {
    func: Function
}

const props = defineProps<Props>()

const emit = defineEmits<{
    (e: 'edit', func: Function): void
    (e: 'delete', func: Function): void
    (e: 'invoke', func: Function): void
    (e: 'viewLogs', func: Function): void
}>()

const formatDate = (date: string): string => {
    return new Date(date).toLocaleString()
}
</script>

<style scoped>
.function-card {
    border: 1px solid var(--color-border);
    border-radius: 0.5rem;
    padding: 1.5rem;
    background: var(--color-bg-secondary);
    transition: box-shadow 0.2s;
}

.function-card:hover {
    box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
}

.card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
}

.function-name {
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--color-text);
    margin: 0;
}

.function-status {
    padding: 0.25rem 0.75rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 500;
    background: #e5e7eb;
    color: #6b7280;
}

.function-status.enabled {
    background: #d1fae5;
    color: #065f46;
}

.card-body {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-bottom: 1rem;
}

.function-info {
    display: flex;
    gap: 0.5rem;
}

.info-label {
    font-weight: 500;
    color: var(--color-text-muted);
}

.info-value {
    color: var(--color-text);
}

.card-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
}

.btn-action {
    padding: 0.375rem 0.75rem;
    border: 1px solid var(--color-border);
    border-radius: 0.375rem;
    background: transparent;
    color: var(--color-text);
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.2s;
}

.btn-action:hover {
    background: var(--color-bg-hover);
}

.btn-action.danger {
    color: #ef4444;
    border-color: #ef4444;
}

.btn-action.danger:hover {
    background: #fee2e2;
}
</style>
