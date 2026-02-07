<template>
    <div class="code-editor">
        <div v-if="label" class="editor-label">{{ label }}</div>
        <div class="editor-wrapper">
            <textarea
                v-model="localValue"
                :readonly="readonly"
                :placeholder="placeholder"
                class="code-textarea"
                :class="{ readonly }"
                spellcheck="false"
                @input="onInput"
            />
        </div>
        <div v-if="error" class="editor-error">{{ error }}</div>
    </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'

interface Props {
    modelValue: string
    language?: 'sql' | 'javascript' | 'typescript' | 'json' | 'text'
    readonly?: boolean
    placeholder?: string
    label?: string
    error?: string
}

const props = withDefaults(defineProps<Props>(), {
    language: 'text',
    readonly: false,
    placeholder: '',
    label: '',
    error: '',
})

const emit = defineEmits<{
    (e: 'update:modelValue', value: string): void
}>()

const localValue = ref(props.modelValue)

watch(
    () => props.modelValue,
    (newValue) => {
        if (newValue !== localValue.value) {
            localValue.value = newValue
        }
    }
)

const onInput = () => {
    emit('update:modelValue', localValue.value)
}
</script>

<style scoped>
.code-editor {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

.editor-label {
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--color-text);
}

.editor-wrapper {
    border: 1px solid var(--color-border);
    border-radius: 0.375rem;
    overflow: hidden;
    background: var(--color-bg-secondary);
}

.code-textarea {
    width: 100%;
    min-height: 200px;
    padding: 1rem;
    font-family: 'Fira Code', 'Courier New', monospace;
    font-size: 0.875rem;
    line-height: 1.5;
    color: var(--color-text);
    background: transparent;
    border: none;
    outline: none;
    resize: vertical;
    tab-size: 4;
}

.code-textarea.readonly {
    background: var(--color-bg-disabled);
    cursor: not-allowed;
}

.editor-error {
    font-size: 0.75rem;
    color: var(--color-error);
}

/* Scrollbar styling */
.code-textarea::-webkit-scrollbar {
    width: 8px;
    height: 8px;
}

.code-textarea::-webkit-scrollbar-track {
    background: var(--color-bg-secondary);
}

.code-textarea::-webkit-scrollbar-thumb {
    background: var(--color-border);
    border-radius: 4px;
}

.code-textarea::-webkit-scrollbar-thumb:hover {
    background: var(--color-border-hover);
}
</style>
