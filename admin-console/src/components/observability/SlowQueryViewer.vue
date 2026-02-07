<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { observabilityService } from '@/services/observability'

const queries = ref<Array<{
    id: string
    query: string
    duration_ms: number
    timestamp: string
    user_id?: string
    rows_examined: number
    rows_returned: number
    index_used: boolean
}>>([])

const loading = ref(false)
const threshold = ref(100) // ms
const limit = ref(50)
const selectedQuery = ref<string | null>(null)
const explainResult = ref<{
    plan: string
    estimated_cost: number
    index_suggestions: string[]
} | null>(null)

onMounted(() => {
    loadQueries()
})

const loadQueries = async () => {
    loading.value = true
    try {
        queries.value = await observabilityService.getSlowQueries({
            limit: limit.value,
            threshold_ms: threshold.value,
        })
    } finally {
        loading.value = false
    }
}

const explainQuery = async (query: string) => {
    selectedQuery.value = query
    explainResult.value = null
    try {
        explainResult.value = await observabilityService.explainQuery(query)
    } catch (e) {
        console.error('Failed to explain query', e)
    }
}

const formatDuration = (ms: number): string => {
    if (ms < 1000) return `${ms}ms`
    return `${(ms / 1000).toFixed(2)}s`
}

const formatTimestamp = (ts: string) => new Date(ts).toLocaleString()
</script>

<template>
    <div class="h-full flex flex-col">
        <!-- Header -->
        <div class="p-4 border-b border-border">
            <div class="flex items-center justify-between mb-4">
                <h2 class="font-semibold">Slow Query Log</h2>
                <div class="flex items-center gap-3">
                    <div class="flex items-center gap-2">
                        <label class="text-sm text-muted-foreground">Threshold:</label>
                        <select
                            v-model="threshold"
                            @change="loadQueries"
                            class="px-2 py-1 bg-background border border-input rounded text-sm"
                        >
                            <option :value="50">50ms</option>
                            <option :value="100">100ms</option>
                            <option :value="500">500ms</option>
                            <option :value="1000">1s</option>
                        </select>
                    </div>
                    <button
                        @click="loadQueries"
                        class="px-3 py-1 text-sm bg-muted hover:bg-muted/80 rounded"
                    >
                        Refresh
                    </button>
                </div>
            </div>
        </div>
        
        <!-- Content -->
        <div class="flex-1 overflow-hidden flex">
            <!-- Query list -->
            <div class="w-1/2 border-r border-border overflow-y-auto">
                <div v-if="loading" class="p-4 text-center text-muted-foreground">
                    Loading...
                </div>
                
                <div v-else-if="queries.length === 0" class="p-4 text-center text-muted-foreground">
                    No slow queries found
                </div>
                
                <div v-else>
                    <div
                        v-for="q in queries"
                        :key="q.id"
                        @click="explainQuery(q.query)"
                        class="p-4 border-b border-border hover:bg-muted/50 cursor-pointer"
                        :class="{ 'bg-muted/50': selectedQuery === q.query }"
                    >
                        <div class="flex items-center justify-between mb-2">
                            <span 
                                class="text-sm font-medium px-2 py-0.5 rounded"
                                :class="q.duration_ms > 1000 ? 'bg-red-500/10 text-red-500' : 'bg-yellow-500/10 text-yellow-500'"
                            >
                                {{ formatDuration(q.duration_ms) }}
                            </span>
                            <span class="text-xs text-muted-foreground">
                                {{ formatTimestamp(q.timestamp) }}
                            </span>
                        </div>
                        <pre class="text-xs font-mono text-muted-foreground truncate">{{ q.query }}</pre>
                        <div class="flex items-center gap-4 mt-2 text-xs text-muted-foreground">
                            <span>Rows examined: {{ q.rows_examined.toLocaleString() }}</span>
                            <span>Returned: {{ q.rows_returned.toLocaleString() }}</span>
                            <span :class="q.index_used ? 'text-green-500' : 'text-yellow-500'">
                                {{ q.index_used ? '✓ Index used' : '⚠ No index' }}
                            </span>
                        </div>
                    </div>
                </div>
            </div>
            
            <!-- Explain panel -->
            <div class="w-1/2 p-4 overflow-y-auto">
                <div v-if="!selectedQuery" class="text-center text-muted-foreground py-8">
                    Select a query to view execution plan
                </div>
                
                <div v-else-if="!explainResult" class="text-center text-muted-foreground py-8">
                    Loading execution plan...
                </div>
                
                <div v-else class="space-y-4">
                    <div>
                        <h3 class="font-medium mb-2">Query</h3>
                        <pre class="p-3 bg-muted rounded text-xs font-mono overflow-x-auto whitespace-pre-wrap">{{ selectedQuery }}</pre>
                    </div>
                    
                    <div>
                        <h3 class="font-medium mb-2">Execution Plan</h3>
                        <pre class="p-3 bg-muted rounded text-xs font-mono overflow-x-auto whitespace-pre-wrap">{{ explainResult.plan }}</pre>
                    </div>
                    
                    <div>
                        <h3 class="font-medium mb-2">Estimated Cost</h3>
                        <p class="text-sm">{{ explainResult.estimated_cost.toFixed(2) }}</p>
                    </div>
                    
                    <div v-if="explainResult.index_suggestions.length > 0">
                        <h3 class="font-medium mb-2">Index Suggestions</h3>
                        <ul class="space-y-1">
                            <li
                                v-for="(suggestion, idx) in explainResult.index_suggestions"
                                :key="idx"
                                class="text-sm p-2 bg-yellow-500/10 text-yellow-600 rounded font-mono"
                            >
                                {{ suggestion }}
                            </li>
                        </ul>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>
