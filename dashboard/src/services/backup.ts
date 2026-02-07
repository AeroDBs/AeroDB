import { useApi } from '@/composables/useApi'
import type { BackupJob, BackupManifest, RestoreJob } from '@/types'

const { api } = useApi()

export const backupService = {
    /**
     * Create a new backup
     */
    async createBackup(options?: {
        incremental?: boolean
        compression?: 'none' | 'gzip' | 'zstd'
    }): Promise<BackupJob> {
        const response = await api.post('/backup/create', options || {})
        return response.data
    },

    /**
     * List all backups
     */
    async listBackups(): Promise<Array<{
        id: string
        manifest: BackupManifest
        created_at: string
        size_bytes: number
    }>> {
        const response = await api.get('/backup/list')
        return response.data
    },

    /**
     * Get backup details
     */
    async getBackup(backupId: string): Promise<{
        id: string
        manifest: BackupManifest
        created_at: string
        size_bytes: number
        files: string[]
    }> {
        const response = await api.get(`/backup/${backupId}`)
        return response.data
    },

    /**
     * Delete a backup
     */
    async deleteBackup(backupId: string): Promise<void> {
        await api.delete(`/backup/${backupId}`)
    },

    /**
     * Download a backup
     */
    async downloadBackup(backupId: string): Promise<Blob> {
        const response = await api.get(`/backup/${backupId}/download`, {
            responseType: 'blob',
        })
        return response.data
    },

    /**
     * Get backup job status
     */
    async getBackupStatus(jobId: string): Promise<BackupJob> {
        const response = await api.get(`/backup/jobs/${jobId}`)
        return response.data
    },

    /**
     * Restore from a backup
     */
    async restoreBackup(backupId: string, options?: {
        validate_only?: boolean
        target_path?: string
    }): Promise<RestoreJob> {
        const response = await api.post(`/backup/${backupId}/restore`, options || {})
        return response.data
    },

    /**
     * Upload a backup file for restore
     */
    async uploadBackup(
        file: File,
        onProgress?: (progress: number) => void
    ): Promise<{ backup_id: string }> {
        const formData = new FormData()
        formData.append('backup', file)

        const response = await api.post('/backup/upload', formData, {
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

    /**
     * Get restore job status
     */
    async getRestoreStatus(jobId: string): Promise<RestoreJob> {
        const response = await api.get(`/restore/jobs/${jobId}`)
        return response.data
    },

    /**
     * Validate a backup without restoring
     */
    async validateBackup(backupId: string): Promise<{
        valid: boolean
        errors: string[]
        warnings: string[]
    }> {
        const response = await api.post(`/backup/${backupId}/validate`)
        return response.data
    },

    /**
     * Get backup statistics
     */
    async getBackupStats(): Promise<{
        total_backups: number
        total_size_bytes: number
        last_backup_at?: string
        oldest_backup_at?: string
    }> {
        const response = await api.get('/backup/stats')
        return response.data
    },

    // ========== Backup Schedules ==========

    /**
     * Get backup schedule
     */
    async getSchedule(): Promise<{
        enabled: boolean
        cron_expression: string
        retention_days: number
        compression: 'none' | 'gzip' | 'zstd'
        incremental: boolean
        last_run_at?: string
        next_run_at?: string
    }> {
        const response = await api.get('/backup/schedule')
        return response.data
    },

    /**
     * Update backup schedule
     */
    async updateSchedule(schedule: {
        enabled?: boolean
        cron_expression?: string
        retention_days?: number
        compression?: 'none' | 'gzip' | 'zstd'
        incremental?: boolean
    }): Promise<void> {
        await api.patch('/backup/schedule', schedule)
    },

    /**
     * Run scheduled backup now
     */
    async runScheduledBackup(): Promise<BackupJob> {
        const response = await api.post('/backup/schedule/run-now')
        return response.data
    },

    /**
     * Get backup history
     */
    async getBackupHistory(options?: {
        limit?: number
        since?: string
    }): Promise<Array<{
        id: string
        type: 'manual' | 'scheduled'
        status: 'success' | 'failed'
        started_at: string
        completed_at?: string
        size_bytes?: number
        error?: string
    }>> {
        const params = new URLSearchParams()
        if (options?.limit) params.append('limit', options.limit.toString())
        if (options?.since) params.append('since', options.since)

        const response = await api.get(`/backup/history?${params}`)
        return response.data
    },

    /**
     * Apply retention policy (cleanup old backups)
     */
    async applyRetention(): Promise<{
        deleted_count: number
        freed_bytes: number
    }> {
        const response = await api.post('/backup/retention/apply')
        return response.data
    },
}

