<script setup lang="ts">
import { ref } from 'vue'

const props = defineProps<{
    triggers: Array<{
        type: 'http' | 'cron' | 'event'
        config: Record<string, unknown>
    }>
}>()

const emit = defineEmits<{
    (e: 'update:triggers', triggers: typeof props.triggers): void
}>()

const showAddDialog = ref(false)
const newTriggerType = ref<'http' | 'cron' | 'event'>('http')

const localTriggers = ref([...props.triggers])

// HTTP trigger config
const httpPath = ref('')
const httpMethods = ref(['GET'])

// Cron trigger config
const cronExpression = ref('0 * * * *')

// Event trigger config  
const eventTable = ref('')
const eventOperations = ref(['INSERT'])

const addTrigger = () => {
    let config: Record<string, unknown> = {}
    
    switch (newTriggerType.value) {
        case 'http':
            config = {
                path: httpPath.value || '/my-function',
                methods: httpMethods.value,
            }
            break
        case 'cron':
            config = {
                expression: cronExpression.value,
            }
            break
        case 'event':
            config = {
                table: eventTable.value,
                operations: eventOperations.value,
            }
            break
    }
    
    localTriggers.value.push({
        type: newTriggerType.value,
        config,
    })
    
    emit('update:triggers', [...localTriggers.value])
    resetForm()
    showAddDialog.value = false
}

const removeTrigger = (index: number) => {
    localTriggers.value.splice(index, 1)
    emit('update:triggers', [...localTriggers.value])
}

const resetForm = () => {
    httpPath.value = ''
    httpMethods.value = ['GET']
    cronExpression.value = '0 * * * *'
    eventTable.value = ''
    eventOperations.value = ['INSERT']
}

const getTriggerIcon = (type: string) => {
    switch (type) {
        case 'http': return '🌐'
        case 'cron': return '⏰'
        case 'event': return '📬'
        default: return '⚡'
    }
}

const getTriggerSummary = (trigger: typeof props.triggers[0]) => {
    switch (trigger.type) {
        case 'http':
            return `${(trigger.config.methods as string[])?.join(', ')} ${trigger.config.path}`
        case 'cron':
            return trigger.config.expression as string
        case 'event':
            return `${trigger.config.table}: ${(trigger.config.operations as string[])?.join(', ')}`
        default:
            return JSON.stringify(trigger.config)
    }
}

const cronPresets = [
    { label: 'Every minute', value: '* * * * *' },
    { label: 'Every hour', value: '0 * * * *' },
    { label: 'Every day at midnight', value: '0 0 * * *' },
    { label: 'Every Monday', value: '0 0 * * 1' },
    { label: 'Every month', value: '0 0 1 * *' },
]
</script>

<template>
    <div class="space-y-4">
        <div class="flex items-center justify-between">
            <h3 class="font-medium">Triggers</h3>
            <button
                @click="showAddDialog = true"
                class="text-sm px-3 py-1 rounded bg-primary text-primary-foreground hover:opacity-90"
            >
                + Add Trigger
            </button>
        </div>
        
        <!-- Triggers list -->
        <div v-if="localTriggers.length === 0" class="text-center py-6 text-muted-foreground text-sm">
            No triggers configured
        </div>
        
        <div v-else class="space-y-2">
            <div
                v-for="(trigger, index) in localTriggers"
                :key="index"
                class="flex items-center justify-between p-3 bg-muted/50 rounded-lg"
            >
                <div class="flex items-center gap-3">
                    <span class="text-xl">{{ getTriggerIcon(trigger.type) }}</span>
                    <div>
                        <p class="font-medium capitalize">{{ trigger.type }} Trigger</p>
                        <p class="text-sm text-muted-foreground font-mono">
                            {{ getTriggerSummary(trigger) }}
                        </p>
                    </div>
                </div>
                <button
                    @click="removeTrigger(index)"
                    class="text-destructive hover:underline text-sm"
                >
                    Remove
                </button>
            </div>
        </div>
        
        <!-- Add trigger dialog -->
        <div
            v-if="showAddDialog"
            class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
            @click="showAddDialog = false"
        >
            <div
                class="bg-card border border-border rounded-lg shadow-lg max-w-md w-full mx-4 p-6"
                @click.stop
            >
                <h2 class="text-lg font-semibold mb-4">Add Trigger</h2>
                
                <!-- Trigger type selector -->
                <div class="mb-4">
                    <label class="block text-sm font-medium mb-2">Trigger Type</label>
                    <select
                        v-model="newTriggerType"
                        class="w-full px-3 py-2 bg-background border border-input rounded-md"
                    >
                        <option value="http">HTTP Endpoint</option>
                        <option value="cron">Cron Schedule</option>
                        <option value="event">Database Event</option>
                    </select>
                </div>
                
                <!-- HTTP config -->
                <div v-if="newTriggerType === 'http'" class="space-y-3">
                    <div>
                        <label class="block text-sm font-medium mb-1">Endpoint Path</label>
                        <input
                            v-model="httpPath"
                            type="text"
                            placeholder="/my-function"
                            class="w-full px-3 py-2 bg-background border border-input rounded-md font-mono"
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium mb-1">HTTP Methods</label>
                        <div class="flex flex-wrap gap-2">
                            <label v-for="method in ['GET', 'POST', 'PUT', 'DELETE']" :key="method" class="flex items-center gap-1">
                                <input type="checkbox" :value="method" v-model="httpMethods" />
                                {{ method }}
                            </label>
                        </div>
                    </div>
                </div>
                
                <!-- Cron config -->
                <div v-if="newTriggerType === 'cron'" class="space-y-3">
                    <div>
                        <label class="block text-sm font-medium mb-1">Cron Expression</label>
                        <input
                            v-model="cronExpression"
                            type="text"
                            placeholder="0 * * * *"
                            class="w-full px-3 py-2 bg-background border border-input rounded-md font-mono"
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium mb-1">Presets</label>
                        <div class="flex flex-wrap gap-2">
                            <button
                                v-for="preset in cronPresets"
                                :key="preset.value"
                                @click="cronExpression = preset.value"
                                class="px-2 py-1 text-xs bg-muted rounded hover:bg-muted/80"
                            >
                                {{ preset.label }}
                            </button>
                        </div>
                    </div>
                </div>
                
                <!-- Event config -->
                <div v-if="newTriggerType === 'event'" class="space-y-3">
                    <div>
                        <label class="block text-sm font-medium mb-1">Table Name</label>
                        <input
                            v-model="eventTable"
                            type="text"
                            placeholder="users"
                            class="w-full px-3 py-2 bg-background border border-input rounded-md font-mono"
                        />
                    </div>
                    <div>
                        <label class="block text-sm font-medium mb-1">Operations</label>
                        <div class="flex flex-wrap gap-2">
                            <label v-for="op in ['INSERT', 'UPDATE', 'DELETE']" :key="op" class="flex items-center gap-1">
                                <input type="checkbox" :value="op" v-model="eventOperations" />
                                {{ op }}
                            </label>
                        </div>
                    </div>
                </div>
                
                <div class="flex justify-end gap-3 mt-6">
                    <button
                        @click="showAddDialog = false"
                        class="px-4 py-2 border border-border rounded hover:bg-muted"
                    >
                        Cancel
                    </button>
                    <button
                        @click="addTrigger"
                        class="px-4 py-2 bg-primary text-primary-foreground rounded hover:opacity-90"
                    >
                        Add Trigger
                    </button>
                </div>
            </div>
        </div>
    </div>
</template>
