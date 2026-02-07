<template>
    <div class="replication-status">
        <h3>Replication Status</h3>
        
        <div class="status-grid">
            <div class="status-item">
                <span class="label">Node:</span>
                <span class="value">{{ status.node_id }}</span>
            </div>
            <div class="status-item">
                <span class="label">Role:</span>
                <span class="value">{{ status.role }}</span>
            </div>
            <div class="status-item">
                <span class="label">Status:</span>
                <span class="value" :class="`state-${status.status}`">{{ status.status }}</span>
            </div>
            <div class="status-item">
                <span class="label">WAL Position:</span>
                <span class="value">{{ status.wal_position }}</span>
            </div>
            <div class="status-item">
                <span class="label">Replication Lag:</span>
                <span class="value">{{ status.replication_lag_ms }}ms</span>
            </div>
            <div v-if="status.last_sync_at" class="status-item">
                <span class="label">Last Sync:</span>
                <span class="value">{{ new Date(status.last_sync_at).toLocaleString() }}</span>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import type { ReplicationStatus } from '@/types'

interface Props {
    status: ReplicationStatus
}

const props = defineProps<Props>()
</script>

<style scoped>
.replication-status {
    border: 1px solid var(--color-border);
    border-radius: 0.5rem;
    padding: 1.5rem;
    background: var(--color-bg-secondary);
}

.replication-status h3 {
    font-size: 1.125rem;
    font-weight: 600;
    margin-bottom: 1rem;
}

.status-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
    margin-bottom: 1.5rem;
}

.status-item {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
}

.label {
    font-weight: 500;
    font-size: 0.875rem;
    color: var(--color-text-muted);
}

.value {
    font-family: monospace;
    font-size: 0.875rem;
    color: var(--color-text);
}

.value.state-streaming {
    color: #10b981;
}

.value.state-catchup {
    color: #f59e0b;
}

.value.state-disconnected {
    color: #ef4444;
}

.sync-replicas {
    border-top: 1px solid var(--color-border);
    padding-top: 1rem;
}

.sync-replicas h4 {
    font-size: 0.875rem;
    font-weight: 600;
    margin-bottom: 0.5rem;
}

.replica-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

.replica-item {
    padding: 0.5rem;
    background: var(--color-bg);
    border-radius: 0.375rem;
    font-family: monospace;
    font-size: 0.875rem;
}
</style>
