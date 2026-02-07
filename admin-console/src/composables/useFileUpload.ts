import { ref } from 'vue'

export interface UploadProgress {
    loaded: number
    total: number
    percentage: number
}

export function useFileUpload() {
    const uploading = ref(false)
    const progress = ref<UploadProgress>({
        loaded: 0,
        total: 0,
        percentage: 0,
    })
    const error = ref<string | null>(null)

    const upload = async (
        file: File,
        uploadFn: (file: File, onProgress: (progress: number) => void) => Promise<any>
    ) => {
        uploading.value = true
        error.value = null
        progress.value = { loaded: 0, total: 0, percentage: 0 }

        try {
            const result = await uploadFn(file, (percentage: number) => {
                progress.value = {
                    loaded: (file.size * percentage) / 100,
                    total: file.size,
                    percentage,
                }
            })
            return result
        } catch (err) {
            error.value = err instanceof Error ? err.message : 'Upload failed'
            throw err
        } finally {
            uploading.value = false
        }
    }

    const reset = () => {
        uploading.value = false
        progress.value = { loaded: 0, total: 0, percentage: 0 }
        error.value = null
    }

    return {
        uploading,
        progress,
        error,
        upload,
        reset,
    }
}
