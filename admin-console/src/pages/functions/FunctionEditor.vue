<template>
    <div class="function-editor-page">
        <div class="page-header">
            <button class="btn-back" @click="router.back()">← Back</button>
            <h1>{{ functionData?.name || 'Loading...' }}</h1>
            <div class="header-actions">
                <button class="btn-secondary" @click="loadFunction">Refresh</button>
                <button class="btn-primary" @click="saveFunction" :disabled="saving">
                    {{ saving ? 'Saving...' : 'Save' }}
                </button>
                <button class="btn-primary" @click="testFunction">Test</button>
            </div>
        </div>

        <div v-if="loading" class="loading">Loading function...</div>

        <div v-else-if="error" class="error-state">
            <p>{{ error }}</p>
        </div>

        <div v-else-if="functionData" class="editor-content">
            <div class="editor-section">
                <h2>Function Code</h2>
                <CodeEditor
                    v-model="functionData.code"
                    language="javascript"
                    :readonly="false"
                />
            </div>

            <div class="editor-section">
                <h2>Environment Variables</h2>
                <div class="env-vars">
                    <div
                        v-for="key in Object.keys(functionData.env_vars || {})"
                        :key="key"
                        class="env-var-row"
                    >
                        <input
                            v-model="functionData.env_vars![key]"
                            type="text"
                            :placeholder="`${key}=value`"
                        />
                        <button
                            class="btn-remove"
                            @click="functionData.env_vars && delete functionData.env_vars[key]"
                        >
                            ✕
                        </button>
                    </div>
                    <button class="btn-secondary" @click="addEnvVar">+ Add Variable</button>
                </div>
            </div>

            <div class="editor-section">
                <h2>Settings</h2>
                <div class="form-group">
                    <label>
                        <input v-model="functionData.enabled" type="checkbox" />
                        Enabled
                    </label>
                </div>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { functionsService } from '@/services'
import { getErrorMessage } from '@/composables/useApi'
import type { Function } from '@/types'
import CodeEditor from '@/components/common/CodeEditor.vue'

const router = useRouter()
const route = useRoute()

const functionData = ref<Function | null>(null)
const loading = ref(false)
const saving = ref(false)
const error = ref('')

const loadFunction = async () => {
    loading.value = true
    error.value = ''
    try {
        const id = route.params.id as string
        functionData.value = await functionsService.getFunction(id)
    } catch (err) {
        error.value = getErrorMessage(err)
    } finally {
        loading.value = false
    }
}

const saveFunction = async () => {
    if (!functionData.value) return
    
    saving.value = true
    try {
        await functionsService.updateFunction(functionData.value.id, {
            code: functionData.value.code,
            env_vars: functionData.value.env_vars,
            enabled: functionData.value.enabled,
        })
        alert('Function saved successfully!')
    } catch (err) {
        error.value = getErrorMessage(err)
    } finally {
        saving.value = false
    }
}

const testFunction = async () => {
    if (!functionData.value) return
    
    try {
        const result = await functionsService.invokeFunction(functionData.value.id, {})
        alert(`Function executed!\nDuration: ${result.duration_ms}ms\nResult: ${JSON.stringify(result.result, null, 2)}`)
    } catch (err) {
        alert(`Error: ${getErrorMessage(err)}`)
    }
}

const addEnvVar = () => {
    if (!functionData.value) return
    const key = prompt('Environment variable name:')
    if (key) {
        if (!functionData.value.env_vars) {
            functionData.value.env_vars = {}
        }
        functionData.value.env_vars[key] = ''
    }
}

onMounted(() => {
    loadFunction()
})
</script>

<style scoped>
.function-editor-page {
    padding: 2rem;
}

.page-header {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 2rem;
}

.btn-back {
    padding: 0.5rem 1rem;
    border: 1px solid var(--color-border);
    border-radius: 0.375rem;
    background: transparent;
    color: var(--color-text);
    cursor: pointer;
}

.page-header h1 {
    flex: 1;
    font-size: 1.875rem;
    font-weight: 700;
    margin: 0;
}

.header-actions {
    display: flex;
    gap: 0.5rem;
}

.editor-content {
    display: flex;
    flex-direction: column;
    gap: 2rem;
}

.editor-section h2 {
    font-size: 1.25rem;
    font-weight: 600;
    margin-bottom: 1rem;
}

.env-vars {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

.env-var-row {
    display: flex;
    gap: 0.5rem;
}

.env-var-row input {
    flex: 1;
    padding: 0.5rem;
    border: 1px solid var(--color-border);
    border-radius: 0.375rem;
}

.btn-remove {
    padding: 0.5rem;
    border: 1px solid #ef4444;
    border-radius: 0.375rem;
    background: transparent;
    color: #ef4444;
    cursor: pointer;
}

.form-group label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
}

.btn-primary,
.btn-secondary {
    padding: 0.5rem 1rem;
    border-radius: 0.375rem;
    font-weight: 500;
    cursor: pointer;
}

.btn-primary {
    background: var(--color-primary);
    color: white;
    border: none;
}

.btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.btn-secondary {
    background: transparent;
    border: 1px solid var(--color-border);
    color: var(--color-text);
}

.loading,
.error-state {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 400px;
}
</style>
