<template>
    <div class="snapshot-page">
        <div class="page-header">
            <h1>Snapshots</h1>
            <button class="btn-primary" @click="createSnapshot" :disabled="creating">
                {{ creating ? 'Creating...' : '+ Create Snapshot' }}
            </button>
        </div>

        <div v-if="loading" class="loading">Loading snapshots...</div>

        <div v-else-if="error" class="error-state">
            <p>{{ error }}</p>
            <button class="btn-secondary" @click="loadSnapshots">Retry</button>
        </div>

        <div v-else class="snapshots-grid">
            <div
                v-for="snapshot in snapshots"
                :key="snapshot.id"
                class="snapshot-card"
            >
                <div class="card-header">
                    <h3>{{ snapshot.id.substring(0, 12) }}</h3>
                    <span class="snapshot-type">SNAPSHOT</span>
                </div>
                <div class="card-body">
                    <div class="info-row">
                        <span class="label">Created:</span>
                        <span class="value">{{ formatDate(snapshot.created_at) }}</span>
                    </div>
                    <div class="info-row">
                        <span class="label">WAL Position:</span>
                        <span class="value">{{ snapshot.manifest.wal_position }}</span>
                    </div>
                    <div class="info-row">
                        <span class="label">Size:</span>
                        <span class="value">{{ formatBytes(snapshot.size_bytes) }}</span>
                    </div>
                    <div class="info-row">
                        <span class="label">Tables:</span>
                        <span class="value">{{ snapshot.manifest.tables.length }}</span>
                    </div>
                </div>
                <div class="card-actions">
                    <button class="btn-action" @click="viewDetails(snapshot)">Details</button>
                    <button class="btn-action" @click="restoreFromSnapshot(snapshot)">Restore</button>
                    <button class="btn-action danger" @click="deleteSnapshot(snapshot)">Delete</button>
                </div>
            </div>
        </div>

        <!-- Snapshot Details Dialog -->
        <div v-if="selectedSnapshot" class="modal-overlay" @click.self="selectedSnapshot = null">
            <div class="modal-content">
                <h2>Snapshot Details</h2>
                <JsonViewer :data="selectedSnapshot.manifest" />
                <div class="modal-actions">
                    <button class="btn-secondary" @click="selectedSnapshot = null">Close</button>
                </div>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { getErrorMessage } from '@/composables/useApi'
import type { Snapshot } from '@/types'
import JsonViewer from '@/components/common/JsonViewer.vue'

const snapshots = ref<Snapshot[]>([])
const selectedSnapshot = ref<Snapshot | null>(null)
const loading = ref(false)
const creating = ref(false)
const error = ref('')

const loadSnapshots = async () => {
    loading.value = true
    error.value = ''
    try {
        // Mock data for now - replace with actual API call
        snapshots.value = []
    } catch (err) {
        error.value = getErrorMessage(err)
    } finally {
        loading.value = false
    }
}

const createSnapshot = async () => {
    creating.value = true
    try {
        // Mock implementation - replace with actual API call
        alert('Snapshot creation initiated')
        await loadSnapshots()
    } catch (err) {
        error.value = getErrorMessage(err)
    } finally {
        creating.value = false
    }
}

const viewDetails = (snapshot: Snapshot) => {
    selectedSnapshot.value = snapshot
}

const restoreFromSnapshot = (snapshot: Snapshot) => {
    if (confirm(`Restore from snapshot ${snapshot.id.substring(0, 12)}?\nThis will overwrite current data!`)) {
        // Navigate to restore page or trigger restore
        alert('Restore from snapshot - not implemented yet')
    }
}

const deleteSnapshot = async (snapshot: Snapshot) => {
    if (confirm(`Delete snapshot ${snapshot.id.substring(0, 12)}?`)) {
        try {
            // Mock implementation - replace with actual API call
            alert('Snapshot deleted')
            await loadSnapshots()
        } catch (err) {
            error.value = getErrorMessage(err)
        }
    }
}

const formatDate = (date: string): string => {
    return new Date(date).toLocaleString()
}

const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 Bytes'
    const k = 1024
    const sizes = ['Bytes', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i]
}

onMounted(() => {
    loadSnapshots()
})
</script>

<style scoped>
.snapshot-page {
    padding: 2rem;
}

.page-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 2rem;
}

.page-header h1 {
    font-size: 1.875rem;
    font-weight: 700;
}

.snapshots-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 1.5rem;
}

.snapshot-card {
    border: 1px solid var(--color-border);
    border-radius: 0.5rem;
    padding: 1.5rem;
    background: var(--color-bg-secondary);
    transition: box-shadow 0.2s;
}

.snapshot-card:hover {
    box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
}

.card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
}

.card-header h3 {
    font-size: 1rem;
    font-weight: 600;
    font-family: monospace;
}

.snapshot-type {
    padding: 0.25rem 0.75rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 500;
    background: #dbeafe;
    color: #1e40af;
    text-transform: uppercase;
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

.btn-primary {
    padding: 0.5rem 1rem;
    border-radius: 0.375rem;
    font-weight: 500;
    cursor: pointer;
    background: var(--color-primary);
    color: white;
    border: none;
}

.btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.btn-secondary {
    padding: 0.5rem 1rem;
    border: 1px solid var(--color-border);
    border-radius: 0.375rem;
    background: transparent;
    color: var(--color-text);
    cursor: pointer;
}

.loading,
.error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 400px;
    gap: 1rem;
}

.modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
}

.modal-content {
    background: var(--color-bg);
    border-radius: 0.5rem;
    padding: 2rem;
    max-width: 600px;
    max-height: 80vh;
    overflow-y: auto;
}

.modal-content h2 {
    margin-bottom: 1rem;
}

.modal-actions {
    margin-top: 1.5rem;
    display: flex;
    justify-content: flex-end;
}
</style>
