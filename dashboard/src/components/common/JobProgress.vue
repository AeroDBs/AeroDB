<template>
    <div class="job-progress">
        <div class="progress-header">
            <div class="job-info">
                <span class="job-name">{{ jobName }}</span>
                <span class="job-status" :class="`status-${status}`">{{ status }}</span>
            </div>
            <span class="progress-percentage">{{ Math.round(progress) }}%</span>
        </div>
        <div class="progress-bar-container">
            <div
                class="progress-bar"
                :class="`status-${status}`"
                :style="{ width: `${progress}%` }"
            ></div>
        </div>
        <div v-if="message" class="progress-message">{{ message }}</div>
        <div v-if="error" class="progress-error">⚠️ {{ error }}</div>
    </div>
</template>

<script setup lang="ts">
interface Props {
    jobName: string
    status: 'pending' | 'running' | 'completed' | 'failed' | 'validating' | 'restoring'
    progress: number
    message?: string
    error?: string
}

withDefaults(defineProps<Props>(), {
    message: '',
    error: '',
})
</script>

<style scoped>
.job-progress {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 1rem;
    border: 1px solid var(--color-border);
    border-radius: 0.375rem;
    background: var(--color-bg-secondary);
}

.progress-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.job-info {
    display: flex;
    align-items: center;
    gap: 0.75rem;
}

.job-name {
    font-weight: 500;
    color: var(--color-text);
}

.job-status {
    padding: 0.125rem 0.5rem;
    font-size: 0.75rem;
    border-radius: 9999px;
    font-weight: 500;
}

.status-pending {
    background: #f3f4f6;
    color: #6b7280;
}

.status-running,
.status-validating,
.status-restoring {
    background: #dbeafe;
    color: #1e40af;
}

.status-completed {
    background: #d1fae5;
    color: #065f46;
}

.status-failed {
    background: #fee2e2;
    color: #991b1b;
}

.progress-percentage {
    font-weight: 600;
    color: var(--color-text);
}

.progress-bar-container {
    height: 8px;
    background: var(--color-bg);
    border-radius: 9999px;
    overflow: hidden;
}

.progress-bar {
    height: 100%;
    border-radius: 9999px;
    transition: width 0.3s ease;
}

.progress-bar.status-pending {
    background: #9ca3af;
}

.progress-bar.status-running,
.progress-bar.status-validating,
.progress-bar.status-restoring {
    background: #3b82f6;
}

.progress-bar.status-completed {
    background: #10b981;
}

.progress-bar.status-failed {
    background: #ef4444;
}

.progress-message {
    font-size: 0.875rem;
    color: var(--color-text-muted);
}

.progress-error {
    font-size: 0.875rem;
    color: var(--color-error);
    display: flex;
    align-items: center;
    gap: 0.25rem;
}
</style>
