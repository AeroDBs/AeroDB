import { ref, onUnmounted } from 'vue'

export function usePolling(
    callback: () => void | Promise<void>,
    interval: number = 5000
) {
    const intervalId = ref<number | null>(null)
    const isPolling = ref(false)

    const start = () => {
        if (isPolling.value) return

        isPolling.value = true
        intervalId.value = window.setInterval(async () => {
            try {
                await callback()
            } catch (error) {
                console.error('Polling error:', error)
            }
        }, interval)
    }

    const stop = () => {
        if (intervalId.value !== null) {
            clearInterval(intervalId.value)
            intervalId.value = null
        }
        isPolling.value = false
    }

    const restart = () => {
        stop()
        start()
    }

    // Auto-cleanup on component unmount
    onUnmounted(() => {
        stop()
    })

    return {
        isPolling,
        start,
        stop,
        restart,
    }
}
