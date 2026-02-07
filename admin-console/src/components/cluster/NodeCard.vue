<template>
    <div class="node-card">
        <div class="card-header">
            <div class="node-info">
                <h3>{{ node.id }}</h3>
                <span class="node-role" :class="roleClass">{{ node.role }}</span>
            </div>
            <span class="node-status" :class="statusClass">
                {{ node.status }}
            </span>
        </div>

        <div class="card-body">
            <div v-if="node.replication_lag" class="info-row">
                <span class="label">Replication Lag:</span>
                <span class="value">{{ node.replication_lag }}ms</span>
            </div>
        </div>

        <div v-if="node.role === 'replica'" class="card-actions">
            <button class="btn-action" @click="emit('promote', node)">Promote to Authority</button>
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { ClusterNode } from '@/types'

interface Props {
    node: ClusterNode
}

const props = defineProps<Props>()

const emit = defineEmits<{
    (e: 'promote', node: ClusterNode): void
}>()

const roleClass = computed(() => ({
    'role-authority': props.node.role === 'authority',
    'role-replica': props.node.role === 'replica',
}))

const statusClass = computed(() => ({
    'status-online': props.node.status === 'online',
    'status-offline': props.node.status === 'offline',
}))
</script>

<style scoped>
.node-card {
    border: 1px solid var(--color-border);
    border-radius: 0.5rem;
    padding: 1.5rem;
    background: var(--color-bg-secondary);
}

.card-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 1rem;
}

.node-info {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

.node-info h3 {
    font-size: 1rem;
    font-weight: 600;
    font-family: monospace;
}

.node-role {
    padding: 0.25rem 0.75rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 500;
    width: fit-content;
}

.node-role.role-primary {
    background: #dbeafe;
    color: #1e40af;
}

.node-role.role-replica {
    background: #e5e7eb;
    color: #374151;
}

.node-status {
    padding: 0.25rem 0.75rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 500;
}

.node-status.status-healthy {
    background: #d1fae5;
    color: #065f46;
}

.node-status.status-unhealthy {
    background: #fee2e2;
    color: #991b1b;
}

.card-body {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-bottom: 1rem;
}

.info-row {
    display: flex;
    justify-content: space-between;
}

.label {
    font-weight: 500;
    color: var(--color-text-muted);
}

.value {
    color: var(--color-text);
    font-family: monospace;
}

.card-actions {
    display: flex;
    gap: 0.5rem;
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
</style>
