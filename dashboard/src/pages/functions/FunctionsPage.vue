<template>
    <div class="functions-page">
        <div class="page-header">
            <h1>Edge Functions</h1>
            <button class="btn-primary" @click="showCreateDialog = true">
                + Create Function
            </button>
        </div>

        <div v-if="loading" class="loading">Loading functions...</div>

        <div v-else-if="error" class="error-state">
            <p>{{ error }}</p>
            <button class="btn-secondary" @click="loadFunctions">Retry</button>
        </div>

        <div v-else-if="functions.length === 0" class="empty-state">
            <p>No functions yet</p>
            <button class="btn-primary" @click="showCreateDialog = true">
                Create Your First Function
            </button>
        </div>

        <div v-else class="functions-grid">
            <FunctionCard
                v-for="fn in functions"
                :key="fn.id"
                :func="fn"
                @edit="editFunction"
                @delete="deleteFunction"
                @invoke="invokeFunction"
                @view-logs="viewLogs"
            />
        </div>

        <!-- Create Function Dialog - Simple version since ConfirmDialog API is different -->
        <div v-if="showCreateDialog" class="modal-overlay" @click.self="showCreateDialog = false">
            <div class="modal-content">
                <h2>Create New Function</h2>
                <div class="form-group">
                    <label for="function-name">Function Name</label>
                    <input
                        id="function-name"
                        v-model="newFunction.name"
                        type="text"
                        placeholder="my-function"
                    />
                </div>
                <div class="form-group">
                    <label for="runtime">Runtime</label>
                    <select id="runtime" v-model="newFunction.runtime">
                        <option value="deno">Deno</option>
                        <option value="wasm">WebAssembly</option>
                    </select>
                </div>
                <div class="modal-actions">
                    <button class="btn-secondary" @click="showCreateDialog = false">Cancel</button>
                    <button class="btn-primary" @click="createFunction">Create</button>
                </div>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { functionsService } from '@/services'
import { getErrorMessage } from '@/composables/useApi'
import type { Function } from '@/types'
import FunctionCard from '@/components/functions/FunctionCard.vue'

const router = useRouter()

const functions = ref<Function[]>([])
const loading = ref(false)
const error = ref('')
const showCreateDialog = ref(false)

const newFunction = ref({
    name: '',
    runtime: 'deno' as 'deno' | 'wasm',
})

const loadFunctions = async () => {
    loading.value = true
    error.value = ''
    try {
        functions.value = await functionsService.getFunctions()
    } catch (err) {
        error.value = getErrorMessage(err)
    } finally {
        loading.value = false
    }
}

const createFunction = async () => {
    try {
        const func = await functionsService.createFunction({
            name: newFunction.value.name,
            runtime: newFunction.value.runtime,
            code: '// Write your function code here\nexport default async function handler(req) {\n  return { message: "Hello World" }\n}',
        })
        await loadFunctions()
        showCreateDialog.value = false
        newFunction.value = { name: '', runtime: 'deno' }
        router.push(`/functions/${func.id}/edit`)
    } catch (err) {
        error.value = getErrorMessage(err)
    }
}

const editFunction = (func: Function) => {
    router.push(`/functions/${func.id}/edit`)
}

const deleteFunction = async (func: Function) => {
    if (!confirm(`Delete function "${func.name}"?`)) return
    
    try {
        await functionsService.deleteFunction(func.id)
        await loadFunctions()
    } catch (err) {
        error.value = getErrorMessage(err)
    }
}

const invokeFunction = async (func: Function) => {
    try {
        const result = await functionsService.invokeFunction(func.id, {})
        alert(`Function executed successfully!\nResult: ${JSON.stringify(result.result, null, 2)}`)
    } catch (err) {
        alert(`Function execution failed:\n${getErrorMessage(err)}`)
    }
}

const viewLogs = (func: Function) => {
    router.push(`/functions/${func.id}/logs`)
}

onMounted(() => {
    loadFunctions()
})
</script>

<style scoped>
.functions-page {
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
    color: var(--color-text);
}

.functions-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 1.5rem;
}

.loading,
.empty-state,
.error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 400px;
    gap: 1rem;
}

.empty-state p,
.error-state p {
    color: var(--color-text-muted);
}

.form-group {
    margin-bottom: 1rem;
}

.form-group label {
    display: block;
    margin-bottom: 0.5rem;
    font-weight: 500;
    color: var(--color-text);
}

.form-group input,
.form-group select {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid var(--color-border);
    border-radius: 0.375rem;
    background: var(--color-bg);
    color: var(--color-text);
}

.btn-primary,
.btn-secondary {
    padding: 0.5rem 1rem;
    border-radius: 0.375rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
}

.btn-primary {
    background: var(--color-primary);
    color: white;
    border: none;
}

.btn-primary:hover {
    background: var(--color-primary-dark);
}

.btn-secondary {
    background: transparent;
    border: 1px solid var(--color-border);
    color: var(--color-text);
}

.btn-secondary:hover {
    background: var(--color-bg-hover);
}
</style>
