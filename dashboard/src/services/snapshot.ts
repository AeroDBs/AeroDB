import { useApi } from '@/composables/useApi'

const { api } = useApi()

export interface Snapshot {
    id: string
    name: string
    created_at: string
    size_bytes: number
    status: 'creating' | 'ready' | 'failed'
    collections: string[]
    wal_position: number
}

export const snapshotService = {
    /**
     * Create a new snapshot
     */
    async createSnapshot(name?: string): Promise<Snapshot> {
        const response = await api.post('/snapshots', { name })
        return response.data
    },

    /**
     * List all snapshots
     */
    async listSnapshots(): Promise<Snapshot[]> {
        const response = await api.get('/snapshots')
        return response.data
    },

    /**
     * Get snapshot details
     */
    async getSnapshot(snapshotId: string): Promise<Snapshot> {
        const response = await api.get(`/snapshots/${snapshotId}`)
        return response.data
    },

    /**
     * Delete a snapshot
     */
    async deleteSnapshot(snapshotId: string): Promise<void> {
        await api.delete(`/snapshots/${snapshotId}`)
    },

    /**
     * Restore from snapshot
     */
    async restoreSnapshot(snapshotId: string, options?: {
        target_collections?: string[]
        validate_only?: boolean
    }): Promise<{
        job_id: string
        status: 'pending' | 'running' | 'complete' | 'failed'
    }> {
        const response = await api.post(`/snapshots/${snapshotId}/restore`, options || {})
        return response.data
    },

    /**
     * Get restore job status
     */
    async getRestoreStatus(jobId: string): Promise<{
        status: 'pending' | 'running' | 'complete' | 'failed'
        progress_percent: number
        collections_restored: number
        total_collections: number
        error?: string
    }> {
        const response = await api.get(`/snapshots/restore-jobs/${jobId}`)
        return response.data
    },

    /**
     * Clone a snapshot (create copy)
     */
    async cloneSnapshot(snapshotId: string, newName: string): Promise<Snapshot> {
        const response = await api.post(`/snapshots/${snapshotId}/clone`, { name: newName })
        return response.data
    },

    /**
     * Export snapshot to file
     */
    async exportSnapshot(snapshotId: string): Promise<Blob> {
        const response = await api.get(`/snapshots/${snapshotId}/export`, {
            responseType: 'blob',
        })
        return response.data
    },

    /**
     * Import snapshot from file
     */
    async importSnapshot(file: File, onProgress?: (progress: number) => void): Promise<Snapshot> {
        const formData = new FormData()
        formData.append('snapshot', file)

        const response = await api.post('/snapshots/import', formData, {
            headers: {
                'Content-Type': 'multipart/form-data',
            },
            onUploadProgress: (progressEvent) => {
                if (onProgress && progressEvent.total) {
                    const progress = Math.round((progressEvent.loaded * 100) / progressEvent.total)
                    onProgress(progress)
                }
            },
        })
        return response.data
    },

    // ========== PITR (Point-in-Time Recovery) ==========

    /**
     * Get available PITR range
     */
    async getPITRRange(): Promise<{
        oldest_available: string
        latest_available: string
        wal_retention_hours: number
    }> {
        const response = await api.get('/snapshots/pitr/range')
        return response.data
    },

    /**
     * Restore to point in time
     */
    async restoreToPointInTime(targetTime: string, options?: {
        collections?: string[]
        validate_only?: boolean
    }): Promise<{
        job_id: string
        target_time: string
        status: 'pending' | 'running' | 'complete' | 'failed'
    }> {
        const response = await api.post('/snapshots/pitr/restore', {
            target_time: targetTime,
            ...options,
        })
        return response.data
    },

    /**
     * Get PITR job status
     */
    async getPITRJobStatus(jobId: string): Promise<{
        status: 'pending' | 'running' | 'complete' | 'failed'
        target_time: string
        current_wal_position: number
        target_wal_position: number
        progress_percent: number
        error?: string
    }> {
        const response = await api.get(`/snapshots/pitr/jobs/${jobId}`)
        return response.data
    },
}
