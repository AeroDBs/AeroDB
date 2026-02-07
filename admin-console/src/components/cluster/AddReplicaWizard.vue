<script setup lang="ts">
import { ref } from 'vue'
import { clusterService } from '@/services/cluster'

const emit = defineEmits<{
    (e: 'close'): void
    (e: 'complete'): void
}>()

const step = ref(1)
const isAdding = ref(false)
const error = ref<string | null>(null)
const syncProgress = ref(0)

// Form data
const host = ref('')
const port = ref(5432)
const replicationMode = ref<'sync' | 'async'>('async')

// Step 2: Connection test
const connectionStatus = ref<'pending' | 'testing' | 'success' | 'failed'>('pending')

// Step 3: Sync status
const syncStatus = ref<'pending' | 'syncing' | 'complete' | 'failed'>('pending')

const testConnection = async () => {
    if (!host.value || !port.value) return
    
    step.value = 2
    connectionStatus.value = 'testing'
    
    // Simulate connection test (in real impl, would call backend)
    await new Promise(resolve => setTimeout(resolve, 1500))
    
    // For demo, assume success if host is not empty
    connectionStatus.value = host.value ? 'success' : 'failed'
}

const addReplica = async () => {
    step.value = 3
    syncStatus.value = 'syncing'
    isAdding.value = true
    
    try {
        await clusterService.addReplica({
            host: host.value,
            port: port.value,
            replication_mode: replicationMode.value,
        })
        
        // Simulate sync progress
        for (let i = 0; i <= 100; i += 10) {
            syncProgress.value = i
            await new Promise(resolve => setTimeout(resolve, 300))
        }
        
        syncStatus.value = 'complete'
    } catch (e) {
        syncStatus.value = 'failed'
        error.value = String(e)
    } finally {
        isAdding.value = false
    }
}
</script>

<template>
    <div
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
        @click="emit('close')"
    >
        <div
            class="bg-card border border-border rounded-lg shadow-lg max-w-lg w-full mx-4 p-6"
            @click.stop
        >
            <div class="flex items-center justify-between mb-6">
                <h2 class="text-lg font-semibold">Add Replica Node</h2>
                <button @click="emit('close')" class="p-2 hover:bg-muted rounded">✕</button>
            </div>
            
            <!-- Steps indicator -->
            <div class="flex items-center gap-2 mb-6">
                <div v-for="s in 3" :key="s" class="flex items-center gap-2">
                    <div
                        class="w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium"
                        :class="step >= s ? 'bg-primary text-primary-foreground' : 'bg-muted text-muted-foreground'"
                    >
                        {{ s }}
                    </div>
                    <div v-if="s < 3" class="w-8 h-0.5" :class="step > s ? 'bg-primary' : 'bg-muted'"></div>
                </div>
            </div>
            
            <!-- Step 1: Connection Details -->
            <div v-if="step === 1">
                <h3 class="font-medium mb-4">Connection Details</h3>
                
                <div class="space-y-4">
                    <div>
                        <label class="block text-sm font-medium mb-1">Host</label>
                        <input
                            v-model="host"
                            type="text"
                            placeholder="192.168.1.100 or replica.example.com"
                            class="w-full px-3 py-2 bg-background border border-input rounded-md"
                        />
                    </div>
                    
                    <div>
                        <label class="block text-sm font-medium mb-1">Port</label>
                        <input
                            v-model.number="port"
                            type="number"
                            class="w-full px-3 py-2 bg-background border border-input rounded-md"
                        />
                    </div>
                    
                    <div>
                        <label class="block text-sm font-medium mb-1">Replication Mode</label>
                        <select
                            v-model="replicationMode"
                            class="w-full px-3 py-2 bg-background border border-input rounded-md"
                        >
                            <option value="async">Asynchronous (faster, less consistent)</option>
                            <option value="sync">Synchronous (slower, more consistent)</option>
                        </select>
                    </div>
                </div>
                
                <button
                    @click="testConnection"
                    :disabled="!host || !port"
                    class="w-full mt-4 px-4 py-2 bg-primary text-primary-foreground rounded hover:opacity-90 disabled:opacity-50"
                >
                    Test Connection
                </button>
            </div>
            
            <!-- Step 2: Connection Test -->
            <div v-if="step === 2">
                <h3 class="font-medium mb-4">Connection Test</h3>
                
                <div class="py-8 text-center">
                    <div v-if="connectionStatus === 'testing'" class="space-y-4">
                        <div class="text-4xl animate-spin">⟳</div>
                        <p>Testing connection to {{ host }}:{{ port }}...</p>
                    </div>
                    <div v-else-if="connectionStatus === 'success'" class="space-y-4">
                        <div class="text-4xl text-green-500">✓</div>
                        <p class="text-green-500 font-medium">Connection Successful</p>
                        <p class="text-sm text-muted-foreground">
                            Ready to add as replica
                        </p>
                    </div>
                    <div v-else-if="connectionStatus === 'failed'" class="space-y-4">
                        <div class="text-4xl text-red-500">✗</div>
                        <p class="text-red-500 font-medium">Connection Failed</p>
                        <p class="text-sm text-muted-foreground">
                            Unable to reach {{ host }}:{{ port }}
                        </p>
                    </div>
                </div>
                
                <div class="flex gap-3">
                    <button @click="step = 1" class="flex-1 px-4 py-2 border border-border rounded">
                        Back
                    </button>
                    <button
                        @click="addReplica"
                        :disabled="connectionStatus !== 'success'"
                        class="flex-1 px-4 py-2 bg-primary text-primary-foreground rounded hover:opacity-90 disabled:opacity-50"
                    >
                        Add Replica
                    </button>
                </div>
            </div>
            
            <!-- Step 3: Initial Sync -->
            <div v-if="step === 3">
                <h3 class="font-medium mb-4">Initial Sync</h3>
                
                <div class="py-8 text-center">
                    <div v-if="syncStatus === 'syncing'" class="space-y-4">
                        <p>Syncing data to replica...</p>
                        <div class="w-full bg-muted rounded-full h-3">
                            <div
                                class="bg-primary h-3 rounded-full transition-all duration-300"
                                :style="{ width: `${syncProgress}%` }"
                            ></div>
                        </div>
                        <p class="text-sm text-muted-foreground">{{ syncProgress }}% complete</p>
                    </div>
                    <div v-else-if="syncStatus === 'complete'" class="space-y-4">
                        <div class="text-4xl text-green-500">✓</div>
                        <p class="text-green-500 font-medium">Replica Added Successfully</p>
                        <p class="text-sm text-muted-foreground">
                            {{ host }}:{{ port }} is now replicating data
                        </p>
                    </div>
                    <div v-else-if="syncStatus === 'failed'" class="space-y-4">
                        <div class="text-4xl text-red-500">✗</div>
                        <p class="text-red-500 font-medium">Sync Failed</p>
                        <p class="text-sm text-muted-foreground">{{ error }}</p>
                    </div>
                </div>
                
                <button
                    @click="syncStatus === 'complete' ? emit('complete') : emit('close')"
                    :disabled="syncStatus === 'syncing'"
                    class="w-full mt-4 px-4 py-2 bg-primary text-primary-foreground rounded hover:opacity-90 disabled:opacity-50"
                >
                    {{ syncStatus === 'complete' ? 'Done' : 'Close' }}
                </button>
            </div>
        </div>
    </div>
</template>
