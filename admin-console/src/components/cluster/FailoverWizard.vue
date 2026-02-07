<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { clusterService } from '@/services/cluster'
import type { ClusterNode } from '@/types'

const emit = defineEmits<{
    (e: 'close'): void
    (e: 'complete'): void
}>()

const step = ref(1)
const loading = ref(false)
const error = ref<string | null>(null)

// Step 1: Select replica
const replicas = ref<ClusterNode[]>([])
const selectedReplica = ref<string | null>(null)

// Step 2: Validation status
const validationStatus = ref<{
    lag_check: 'pending' | 'passed' | 'failed'
    health_check: 'pending' | 'passed' | 'failed'
    authority_check: 'pending' | 'passed' | 'failed'
}>({
    lag_check: 'pending',
    health_check: 'pending',
    authority_check: 'pending',
})

// Step 3: Promotion progress
const promotionStatus = ref<'idle' | 'in_progress' | 'complete' | 'failed'>('idle')

const selectedReplicaData = computed(() => 
    replicas.value.find(r => r.id === selectedReplica.value)
)

const validationPassed = computed(() =>
    validationStatus.value.lag_check === 'passed' &&
    validationStatus.value.health_check === 'passed' &&
    validationStatus.value.authority_check === 'passed'
)

onMounted(async () => {
    loading.value = true
    try {
        const topology = await clusterService.getTopology()
        replicas.value = topology.replicas.filter(r => r.status === 'online')
    } catch (e) {
        error.value = 'Failed to load cluster topology'
    } finally {
        loading.value = false
    }
})

const runValidation = async () => {
    if (!selectedReplica.value) return
    
    step.value = 2
    validationStatus.value = {
        lag_check: 'pending',
        health_check: 'pending',
        authority_check: 'pending',
    }
    
    try {
        // Check replication lag
        const repStatus = await clusterService.getNodeReplicationStatus(selectedReplica.value)
        validationStatus.value.lag_check = repStatus.replication_lag_ms < 1000 ? 'passed' : 'failed'
        
        // Check node health
        const node = await clusterService.getNode(selectedReplica.value)
        validationStatus.value.health_check = node.status === 'online' ? 'passed' : 'failed'
        
        // Check current authority
        const marker = await clusterService.getAuthorityMarker()
        validationStatus.value.authority_check = marker.has_marker ? 'passed' : 'failed'
    } catch (e) {
        error.value = 'Validation failed'
    }
}

const executePromotion = async () => {
    if (!selectedReplica.value) return
    
    step.value = 3
    promotionStatus.value = 'in_progress'
    
    try {
        await clusterService.promoteReplica(selectedReplica.value)
        promotionStatus.value = 'complete'
    } catch (e) {
        promotionStatus.value = 'failed'
        error.value = 'Promotion failed'
    }
}

const getStatusIcon = (status: 'pending' | 'passed' | 'failed') => {
    switch (status) {
        case 'pending': return '⏳'
        case 'passed': return '✓'
        case 'failed': return '✗'
    }
}

const getStatusClass = (status: 'pending' | 'passed' | 'failed') => {
    switch (status) {
        case 'pending': return 'text-muted-foreground'
        case 'passed': return 'text-green-500'
        case 'failed': return 'text-red-500'
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
                <h2 class="text-lg font-semibold">Failover Wizard</h2>
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
            
            <!-- Step 1: Select Replica -->
            <div v-if="step === 1">
                <h3 class="font-medium mb-4">Select Replica to Promote</h3>
                
                <div v-if="loading" class="text-center py-4 text-muted-foreground">
                    Loading replicas...
                </div>
                
                <div v-else-if="replicas.length === 0" class="text-center py-4 text-muted-foreground">
                    No eligible replicas found
                </div>
                
                <div v-else class="space-y-2">
                    <button
                        v-for="replica in replicas"
                        :key="replica.id"
                        @click="selectedReplica = replica.id"
                        class="w-full p-4 text-left rounded-lg border transition-colors"
                        :class="selectedReplica === replica.id ? 'border-primary bg-primary/5' : 'border-border hover:bg-muted/50'"
                    >
                        <p class="font-medium">{{ replica.id }}</p>
                        <p class="text-sm text-muted-foreground">
                            Lag: {{ replica.replication_lag || 0 }}ms
                        </p>
                    </button>
                </div>
                
                <button
                    @click="runValidation"
                    :disabled="!selectedReplica"
                    class="w-full mt-4 px-4 py-2 bg-primary text-primary-foreground rounded hover:opacity-90 disabled:opacity-50"
                >
                    Continue
                </button>
            </div>
            
            <!-- Step 2: Validation -->
            <div v-if="step === 2">
                <h3 class="font-medium mb-4">Validation Checks</h3>
                
                <div class="space-y-3 mb-6">
                    <div class="flex items-center justify-between p-3 bg-muted/50 rounded-lg">
                        <span>Replication Lag Check</span>
                        <span :class="getStatusClass(validationStatus.lag_check)">
                            {{ getStatusIcon(validationStatus.lag_check) }}
                        </span>
                    </div>
                    <div class="flex items-center justify-between p-3 bg-muted/50 rounded-lg">
                        <span>Node Health Check</span>
                        <span :class="getStatusClass(validationStatus.health_check)">
                            {{ getStatusIcon(validationStatus.health_check) }}
                        </span>
                    </div>
                    <div class="flex items-center justify-between p-3 bg-muted/50 rounded-lg">
                        <span>Authority Marker Check</span>
                        <span :class="getStatusClass(validationStatus.authority_check)">
                            {{ getStatusIcon(validationStatus.authority_check) }}
                        </span>
                    </div>
                </div>
                
                <div v-if="!validationPassed && validationStatus.lag_check !== 'pending'" class="p-3 bg-yellow-500/10 text-yellow-600 rounded mb-4 text-sm">
                    Some checks failed. Proceed with caution.
                </div>
                
                <div class="flex gap-3">
                    <button @click="step = 1" class="flex-1 px-4 py-2 border border-border rounded">
                        Back
                    </button>
                    <button
                        @click="executePromotion"
                        :disabled="validationStatus.lag_check === 'pending'"
                        class="flex-1 px-4 py-2 bg-primary text-primary-foreground rounded hover:opacity-90 disabled:opacity-50"
                    >
                        Execute Failover
                    </button>
                </div>
            </div>
            
            <!-- Step 3: Execution -->
            <div v-if="step === 3">
                <div class="text-center py-8">
                    <div v-if="promotionStatus === 'in_progress'" class="space-y-4">
                        <div class="text-4xl animate-spin">⟳</div>
                        <p>Executing failover...</p>
                    </div>
                    <div v-else-if="promotionStatus === 'complete'" class="space-y-4">
                        <div class="text-4xl text-green-500">✓</div>
                        <p class="text-green-500 font-medium">Failover Complete</p>
                        <p class="text-sm text-muted-foreground">
                            {{ selectedReplicaData?.id }} is now the authority.
                        </p>
                    </div>
                    <div v-else-if="promotionStatus === 'failed'" class="space-y-4">
                        <div class="text-4xl text-red-500">✗</div>
                        <p class="text-red-500 font-medium">Failover Failed</p>
                        <p class="text-sm text-muted-foreground">{{ error }}</p>
                    </div>
                </div>
                
                <button
                    @click="promotionStatus === 'complete' ? emit('complete') : emit('close')"
                    class="w-full mt-4 px-4 py-2 bg-primary text-primary-foreground rounded hover:opacity-90"
                >
                    {{ promotionStatus === 'complete' ? 'Done' : 'Close' }}
                </button>
            </div>
        </div>
    </div>
</template>
