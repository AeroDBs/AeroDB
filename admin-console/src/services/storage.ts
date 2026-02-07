import { useApi } from '@/composables/useApi'
import type { Bucket, StorageFile } from '@/types'

const { api } = useApi()

export const storageService = {
    /**
     * Get all buckets
     */
    async getBuckets(): Promise<Bucket[]> {
        const response = await api.get('/storage/buckets')
        return response.data
    },

    /**
     * Get a specific bucket
     */
    async getBucket(bucketName: string): Promise<Bucket> {
        const response = await api.get(`/storage/buckets/${bucketName}`)
        return response.data
    },

    /**
     * Create a new bucket
     */
    async createBucket(name: string, isPublic: boolean = false): Promise<Bucket> {
        const response = await api.post('/storage/buckets', {
            name,
            public: isPublic,
        })
        return response.data
    },

    /**
     * Delete a bucket
     */
    async deleteBucket(bucketName: string): Promise<void> {
        await api.delete(`/storage/buckets/${bucketName}`)
    },

    /**
     * Update bucket settings
     */
    async updateBucket(bucketName: string, isPublic: boolean): Promise<Bucket> {
        const response = await api.patch(`/storage/buckets/${bucketName}`, {
            public: isPublic,
        })
        return response.data
    },

    /**
     * List files in a bucket
     */
    async listFiles(bucketName: string, path: string = ''): Promise<StorageFile[]> {
        const params = new URLSearchParams()
        if (path) params.append('path', path)

        const response = await api.get(`/storage/buckets/${bucketName}/files?${params}`)
        return response.data
    },

    /**
     * Get file metadata
     */
    async getFile(bucketName: string, filePath: string): Promise<StorageFile> {
        const response = await api.get(`/storage/buckets/${bucketName}/files/${encodeURIComponent(filePath)}`)
        return response.data
    },

    /**
     * Upload a file
     */
    async uploadFile(
        bucketName: string,
        path: string,
        file: File,
        onProgress?: (progress: number) => void
    ): Promise<StorageFile> {
        const formData = new FormData()
        formData.append('file', file)
        formData.append('path', path)

        const response = await api.post(`/storage/buckets/${bucketName}/files`, formData, {
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
     * Delete a file
     */
    async deleteFile(bucketName: string, filePath: string): Promise<void> {
        await api.delete(`/storage/buckets/${bucketName}/files/${encodeURIComponent(filePath)}`)
    },

    /**
     * Move/rename a file
     */
    async moveFile(bucketName: string, fromPath: string, toPath: string): Promise<StorageFile> {
        const response = await api.post(`/storage/buckets/${bucketName}/files/move`, {
            from: fromPath,
            to: toPath,
        })
        return response.data
    },

    /**
     * Create a signed URL for temporary access
     */
    async createSignedUrl(
        bucketName: string,
        filePath: string,
        expiresIn: number = 3600
    ): Promise<{ url: string; expires_at: string }> {
        const response = await api.post(`/storage/buckets/${bucketName}/files/${encodeURIComponent(filePath)}/sign`, {
            expires_in: expiresIn,
        })
        return response.data
    },

    /**
     * Get public URL for a file (only works for public buckets)
     */
    getPublicUrl(bucketName: string, filePath: string): string {
        const { baseURL } = useApi()
        return `${baseURL}/storage/buckets/${bucketName}/public/${filePath}`
    },

    /**
     * Get bucket statistics
     */
    async getBucketStats(bucketName: string): Promise<{
        total_files: number
        total_size: number
        quota_used: number
        quota_limit: number
    }> {
        const response = await api.get(`/storage/buckets/${bucketName}/stats`)
        return response.data
    },

    // ========== Folder Management ==========

    /**
     * Create a folder (virtual path)
     */
    async createFolder(bucketName: string, folderPath: string): Promise<void> {
        await api.post(`/storage/buckets/${bucketName}/folders`, { path: folderPath })
    },

    /**
     * List folders in a bucket
     */
    async listFolders(bucketName: string, prefix?: string): Promise<string[]> {
        const params = prefix ? `?prefix=${encodeURIComponent(prefix)}` : ''
        const response = await api.get(`/storage/buckets/${bucketName}/folders${params}`)
        return response.data
    },

    /**
     * Delete a folder and all contents
     */
    async deleteFolder(bucketName: string, folderPath: string): Promise<void> {
        await api.delete(`/storage/buckets/${bucketName}/folders/${encodeURIComponent(folderPath)}`)
    },

    // ========== File Permissions ==========

    /**
     * Get file permissions
     */
    async getFilePermissions(bucketName: string, filePath: string): Promise<{
        public: boolean
        allowed_roles: string[]
        owner_id: string
    }> {
        const response = await api.get(`/storage/buckets/${bucketName}/files/${encodeURIComponent(filePath)}/permissions`)
        return response.data
    },

    /**
     * Update file permissions
     */
    async updateFilePermissions(
        bucketName: string,
        filePath: string,
        permissions: {
            public?: boolean
            allowed_roles?: string[]
        }
    ): Promise<void> {
        await api.patch(`/storage/buckets/${bucketName}/files/${encodeURIComponent(filePath)}/permissions`, permissions)
    },

    // ========== Signed URLs Management ==========

    /**
     * List active signed URLs for a file
     */
    async listSignedUrls(bucketName: string, filePath: string): Promise<Array<{
        id: string
        url: string
        expires_at: string
        created_at: string
    }>> {
        const response = await api.get(`/storage/buckets/${bucketName}/files/${encodeURIComponent(filePath)}/signed-urls`)
        return response.data
    },

    /**
     * Revoke a signed URL
     */
    async revokeSignedUrl(bucketName: string, filePath: string, urlId: string): Promise<void> {
        await api.delete(`/storage/buckets/${bucketName}/files/${encodeURIComponent(filePath)}/signed-urls/${urlId}`)
    },

    // ========== Bulk Operations ==========

    /**
     * Upload multiple files
     */
    async bulkUpload(
        bucketName: string,
        files: File[],
        folderPath?: string,
        onProgress?: (current: number, total: number) => void
    ): Promise<StorageFile[]> {
        const results: StorageFile[] = []

        for (let i = 0; i < files.length; i++) {
            const file = files[i]
            const formData = new FormData()
            formData.append('file', file)
            if (folderPath) {
                formData.append('path', folderPath)
            }

            const response = await api.post(
                `/storage/buckets/${bucketName}/files`,
                formData,
                { headers: { 'Content-Type': 'multipart/form-data' } }
            )
            results.push(response.data)

            if (onProgress) {
                onProgress(i + 1, files.length)
            }
        }

        return results
    },

    /**
     * Delete multiple files
     */
    async bulkDelete(bucketName: string, filePaths: string[]): Promise<void> {
        await api.post(`/storage/buckets/${bucketName}/files/bulk-delete`, { paths: filePaths })
    },

    // ========== Storage Quota ==========

    /**
     * Get overall storage quota
     */
    async getStorageQuota(): Promise<{
        used_bytes: number
        limit_bytes: number
        buckets_count: number
        files_count: number
    }> {
        const response = await api.get('/storage/quota')
        return response.data
    },
}

