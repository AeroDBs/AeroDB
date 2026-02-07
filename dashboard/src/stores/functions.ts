import { defineStore } from 'pinia'
import { ref } from 'vue'
import { functionsService } from '@/services'
import type { Function } from '@/types'

export const useFunctionsStore = defineStore('functions', () => {
    const functions = ref<Function[]>([])
    const loading = ref(false)
    const error = ref<string | null>(null)

    const fetchFunctions = async () => {
        loading.value = true
        error.value = null
        try {
            functions.value = await functionsService.getFunctions()
        } catch (err) {
            error.value = err instanceof Error ? err.message : 'Failed to fetch functions'
        } finally {
            loading.value = false
        }
    }

    const createFunction = async (data: {
        name: string
        runtime: 'deno' | 'wasm'
        code: string
        env_vars?: Record<string, string>
    }) => {
        const fn = await functionsService.createFunction(data)
        functions.value.push(fn)
        return fn
    }

    const updateFunction = async (id: string, data: Partial<Function>) => {
        const updated = await functionsService.updateFunction(id, data)
        const index = functions.value.findIndex((f) => f.id === id)
        if (index !== -1) {
            functions.value[index] = updated
        }
        return updated
    }

    const deleteFunction = async (id: string) => {
        await functionsService.deleteFunction(id)
        functions.value = functions.value.filter((f) => f.id !== id)
    }

    return {
        functions,
        loading,
        error,
        fetchFunctions,
        createFunction,
        updateFunction,
        deleteFunction,
    }
})
