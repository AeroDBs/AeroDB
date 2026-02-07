<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { databaseService } from '@/services/database'

type Condition = {
    column: string
    operator: 'eq' | 'neq' | 'gt' | 'lt' | 'gte' | 'lte' | 'like' | 'in' | 'is_null'
    value: string
}

type JoinClause = {
    type: 'inner' | 'left' | 'right'
    table: string
    on: { left: string; right: string }
}

const props = defineProps<{
    tables: string[]
}>()

const emit = defineEmits<{
    (e: 'execute', query: string): void
}>()

// Query state
const selectedTable = ref('')
const selectedColumns = ref<string[]>([])
const conditions = ref<Condition[]>([])
const joins = ref<JoinClause[]>([])
const orderBy = ref('')
const orderDir = ref<'ASC' | 'DESC'>('ASC')
const limit = ref(100)

// Schema cache
const tableSchema = ref<{
    columns: Array<{ name: string; type: string }>
}>({ columns: [] })

watch(selectedTable, async (tableName) => {
    if (tableName) {
        const schema = await databaseService.getTableSchema(tableName)
        tableSchema.value = schema
        selectedColumns.value = schema.columns.map(c => c.name)
    }
})

const addCondition = () => {
    conditions.value.push({
        column: tableSchema.value.columns[0]?.name || '',
        operator: 'eq',
        value: '',
    })
}

const removeCondition = (index: number) => {
    conditions.value.splice(index, 1)
}

const addJoin = () => {
    joins.value.push({
        type: 'inner',
        table: '',
        on: { left: '', right: '' },
    })
}

const removeJoin = (index: number) => {
    joins.value.splice(index, 1)
}

const operatorLabels = {
    eq: '=',
    neq: '≠',
    gt: '>',
    lt: '<',
    gte: '≥',
    lte: '≤',
    like: 'LIKE',
    in: 'IN',
    is_null: 'IS NULL',
}

const generatedQuery = computed(() => {
    if (!selectedTable.value) return ''
    
    const cols = selectedColumns.value.length > 0 
        ? selectedColumns.value.join(', ')
        : '*'
    
    let query = `SELECT ${cols}\nFROM ${selectedTable.value}`
    
    // Joins
    for (const join of joins.value) {
        if (join.table && join.on.left && join.on.right) {
            query += `\n${join.type.toUpperCase()} JOIN ${join.table} ON ${join.on.left} = ${join.on.right}`
        }
    }
    
    // Where conditions
    if (conditions.value.length > 0) {
        const whereClauses = conditions.value
            .filter(c => c.column && c.value)
            .map(c => {
                if (c.operator === 'is_null') {
                    return `${c.column} IS NULL`
                }
                return `${c.column} ${operatorLabels[c.operator]} '${c.value}'`
            })
        
        if (whereClauses.length > 0) {
            query += `\nWHERE ${whereClauses.join(' AND ')}`
        }
    }
    
    // Order by
    if (orderBy.value) {
        query += `\nORDER BY ${orderBy.value} ${orderDir.value}`
    }
    
    // Limit
    if (limit.value > 0) {
        query += `\nLIMIT ${limit.value}`
    }
    
    return query
})

const executeQuery = () => {
    if (generatedQuery.value) {
        emit('execute', generatedQuery.value)
    }
}
</script>

<template>
    <div class="space-y-4">
        <!-- Table Selection -->
        <div>
            <label class="block text-sm font-medium mb-1">Table</label>
            <select
                v-model="selectedTable"
                class="w-full px-3 py-2 bg-background border border-input rounded-md"
            >
                <option value="">Select a table...</option>
                <option v-for="table in tables" :key="table" :value="table">
                    {{ table }}
                </option>
            </select>
        </div>
        
        <!-- Column Selection -->
        <div v-if="tableSchema.columns.length > 0">
            <label class="block text-sm font-medium mb-1">Columns</label>
            <div class="flex flex-wrap gap-2">
                <label
                    v-for="col in tableSchema.columns"
                    :key="col.name"
                    class="flex items-center gap-1 px-2 py-1 bg-muted rounded text-sm cursor-pointer"
                >
                    <input type="checkbox" :value="col.name" v-model="selectedColumns" />
                    {{ col.name }}
                    <span class="text-xs text-muted-foreground">({{ col.type }})</span>
                </label>
            </div>
        </div>
        
        <!-- Joins -->
        <div>
            <div class="flex items-center justify-between mb-2">
                <label class="text-sm font-medium">Joins</label>
                <button @click="addJoin" class="text-xs text-primary hover:underline">+ Add Join</button>
            </div>
            <div class="space-y-2">
                <div v-for="(join, idx) in joins" :key="idx" class="flex items-center gap-2 p-2 bg-muted/50 rounded">
                    <select v-model="join.type" class="px-2 py-1 bg-background border border-input rounded text-sm">
                        <option value="inner">INNER</option>
                        <option value="left">LEFT</option>
                        <option value="right">RIGHT</option>
                    </select>
                    <select v-model="join.table" class="flex-1 px-2 py-1 bg-background border border-input rounded text-sm">
                        <option value="">Table...</option>
                        <option v-for="t in tables" :key="t" :value="t">{{ t }}</option>
                    </select>
                    <span class="text-sm">ON</span>
                    <input v-model="join.on.left" placeholder="left.col" class="w-24 px-2 py-1 bg-background border border-input rounded text-sm" />
                    <span>=</span>
                    <input v-model="join.on.right" placeholder="right.col" class="w-24 px-2 py-1 bg-background border border-input rounded text-sm" />
                    <button @click="removeJoin(idx)" class="text-destructive hover:underline text-xs">×</button>
                </div>
            </div>
        </div>
        
        <!-- Conditions -->
        <div>
            <div class="flex items-center justify-between mb-2">
                <label class="text-sm font-medium">Conditions</label>
                <button @click="addCondition" class="text-xs text-primary hover:underline">+ Add Condition</button>
            </div>
            <div class="space-y-2">
                <div v-for="(cond, idx) in conditions" :key="idx" class="flex items-center gap-2 p-2 bg-muted/50 rounded">
                    <select v-model="cond.column" class="flex-1 px-2 py-1 bg-background border border-input rounded text-sm">
                        <option v-for="col in tableSchema.columns" :key="col.name" :value="col.name">
                            {{ col.name }}
                        </option>
                    </select>
                    <select v-model="cond.operator" class="px-2 py-1 bg-background border border-input rounded text-sm">
                        <option v-for="(label, op) in operatorLabels" :key="op" :value="op">
                            {{ label }}
                        </option>
                    </select>
                    <input
                        v-if="cond.operator !== 'is_null'"
                        v-model="cond.value"
                        placeholder="Value..."
                        class="flex-1 px-2 py-1 bg-background border border-input rounded text-sm"
                    />
                    <button @click="removeCondition(idx)" class="text-destructive hover:underline text-xs">×</button>
                </div>
            </div>
        </div>
        
        <!-- Order By & Limit -->
        <div class="grid grid-cols-3 gap-3">
            <div>
                <label class="block text-sm font-medium mb-1">Order By</label>
                <select v-model="orderBy" class="w-full px-2 py-1 bg-background border border-input rounded text-sm">
                    <option value="">None</option>
                    <option v-for="col in tableSchema.columns" :key="col.name" :value="col.name">
                        {{ col.name }}
                    </option>
                </select>
            </div>
            <div>
                <label class="block text-sm font-medium mb-1">Direction</label>
                <select v-model="orderDir" class="w-full px-2 py-1 bg-background border border-input rounded text-sm">
                    <option value="ASC">Ascending</option>
                    <option value="DESC">Descending</option>
                </select>
            </div>
            <div>
                <label class="block text-sm font-medium mb-1">Limit</label>
                <input
                    v-model.number="limit"
                    type="number"
                    min="0"
                    class="w-full px-2 py-1 bg-background border border-input rounded text-sm"
                />
            </div>
        </div>
        
        <!-- Generated Query -->
        <div>
            <label class="block text-sm font-medium mb-1">Generated Query</label>
            <pre class="p-3 bg-muted rounded text-sm font-mono overflow-x-auto whitespace-pre-wrap">{{ generatedQuery || 'Select a table to build query...' }}</pre>
        </div>
        
        <button
            @click="executeQuery"
            :disabled="!generatedQuery"
            class="w-full px-4 py-2 bg-primary text-primary-foreground rounded hover:opacity-90 disabled:opacity-50"
        >
            Execute Query
        </button>
    </div>
</template>
