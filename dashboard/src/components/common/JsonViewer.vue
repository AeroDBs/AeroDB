<template>
    <div class="json-viewer">
        <div v-if="isObject(data)" class="json-object">
            <div
                v-for="(value, key) in data"
                :key="String(key)"
                class="json-entry"
            >
                <div class="json-key-line">
                    <button
                        v-if="isExpandable(value)"
                        class="expand-button"
                        :class="{ expanded: expandedKeys.has(String(key)) }"
                        @click="toggleExpand(String(key))"
                    >
                        ▶
                    </button>
                    <span class="json-key">{{ key }}:</span>
                    <span v-if="!isExpandable(value)" class="json-value" :class="`type-${getType(value)}`">
                        {{ formatValue(value) }}
                    </span>
                </div>
                <div v-if="isExpandable(value) && expandedKeys.has(String(key))" class="json-nested">
                    <JsonViewer :data="value" :level="level + 1" />
                </div>
            </div>
        </div>
        <div v-else-if="Array.isArray(data)" class="json-array">
            <div
                v-for="(item, index) in data"
                :key="index"
                class="json-entry"
            >
                <div class="json-key-line">
                    <button
                        v-if="isExpandable(item)"
                        class="expand-button"
                        :class="{ expanded: expandedKeys.has(String(index)) }"
                        @click="toggleExpand(String(index))"
                    >
                        ▶
                    </button>
                    <span class="json-index">[{{ index }}]:</span>
                    <span v-if="!isExpandable(item)" class="json-value" :class="`type-${getType(item)}`">
                        {{ formatValue(item) }}
                    </span>
                </div>
                <div v-if="isExpandable(item) && expandedKeys.has(String(index))" class="json-nested">
                    <JsonViewer :data="item" :level="level + 1" />
                </div>
            </div>
        </div>
        <div v-else class="json-value" :class="`type-${getType(data)}`">
            {{ formatValue(data) }}
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

interface Props {
    data: unknown
    level?: number
    expanded?: boolean
}

const props = withDefaults(defineProps<Props>(), {
    level: 0,
    expanded: false,
})

const expandedKeys = ref<Set<string>>(new Set())

const isObject = (value: unknown): value is Record<string, unknown> => {
    return typeof value === 'object' && value !== null && !Array.isArray(value)
}

const isExpandable = (value: unknown): boolean => {
    return isObject(value) || Array.isArray(value)
}

const getType = (value: unknown): string => {
    if (value === null) return 'null'
    if (Array.isArray(value)) return 'array'
    return typeof value
}

const formatValue = (value: unknown): string => {
    if (value === null) return 'null'
    if (value === undefined) return 'undefined'
    if (typeof value === 'string') return `"${value}"`
    if (typeof value === 'boolean') return value.toString()
    if (typeof value === 'number') return value.toString()
    return String(value)
}

const toggleExpand = (key: string) => {
    if (expandedKeys.value.has(key)) {
        expandedKeys.value.delete(key)
    } else {
        expandedKeys.value.add(key)
    }
}
</script>

<style scoped>
.json-viewer {
    font-family: 'Fira Code', 'Courier New', monospace;
    font-size: 0.875rem;
    line-height: 1.5;
}

.json-object,
.json-array {
    display: flex;
    flex-direction: column;
}

.json-entry {
    display: flex;
    flex-direction: column;
}

.json-key-line {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.125rem 0;
}

.expand-button {
    width: 16px;
    height: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: none;
    cursor: pointer;
    color: var(--color-text-muted);
    font-size: 0.625rem;
    transition: transform 0.2s;
}

.expand-button.expanded {
    transform: rotate(90deg);
}

.expand-button:hover {
    color: var(--color-text);
}

.json-key {
    color: #8b5cf6;
    font-weight: 500;
}

.json-index {
    color: #6b7280;
}

.json-value {
    color: var(--color-text);
}

.json-value.type-string {
    color: #10b981;
}

.json-value.type-number {
    color: #f59e0b;
}

.json-value.type-boolean {
    color: #3b82f6;
}

.json-value.type-null {
    color: #ef4444;
    font-style: italic;
}

.json-nested {
    margin-left: 1.5rem;
    border-left: 1px solid var(--color-border);
    padding-left: 0.5rem;
}
</style>
