<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { databaseService } from '@/services/database'

type Migration = {
    id: string
    name: string
    status: 'pending' | 'applied' | 'failed'
    applied_at?: string
    sql_up: string
    sql_down: string
}

const migrations = ref<Migration[]>([])
const loading = ref(false)
const selectedMigration = ref<Migration | null>(null)
const isApplying = ref(false)
const error = ref<string | null>(null)

onMounted(async () => {
    await loadMigrations()
})

const loadMigrations = async () => {
    loading.value = true
    try {
        migrations.value = await databaseService.getMigrations()
    } finally {
        loading.value = false
    }
}

const applyMigration = async (migration: Migration) => {
    isApplying.value = true
    error.value = null
    try {
        await databaseService.applyMigration(migration.id)
        await loadMigrations()
    } catch (e) {
        error.value = String(e)
    } finally {
        isApplying.value = false
    }
}

const rollbackMigration = async (migration: Migration) => {
    if (!confirm('Are you sure you want to rollback this migration?')) return
    
    isApplying.value = true
    error.value = null
    try {
        await databaseService.rollbackMigration(migration.id)
        await loadMigrations()
    } catch (e) {
        error.value = String(e)
    } finally {
        isApplying.value = false
    }
}

const pendingCount = computed(() => migrations.value.filter(m => m.status === 'pending').length)
const appliedCount = computed(() => migrations.value.filter(m => m.status === 'applied').length)

const formatDate = (ts: string) => new Date(ts).toLocaleString()

const getStatusColor = (status: string) => {
    switch (status) {
        case 'pending': return 'bg-yellow-500/10 text-yellow-500'
        case 'applied': return 'bg-green-500/10 text-green-500'
        case 'failed': return 'bg-red-500/10 text-red-500'
        default: return 'bg-muted text-muted-foreground'
    }
}
</script>

<template>
    <div class="h-full flex flex-col">
        <div class="p-4 border-b border-border">
            <div class="flex items-center justify-between">
                <h2 class="font-semibold">Schema Migrations</h2>
                <button @click="loadMigrations" class="px-3 py-1 text-sm bg-muted rounded hover:bg-muted/80">
                    Refresh
                </button>
            </div>
            <div class="flex gap-4 mt-2 text-sm">
                <span class="text-yellow-500">{{ pendingCount }} pending</span>
                <span class="text-green-500">{{ appliedCount }} applied</span>
            </div>
        </div>
        
        <div class="flex-1 overflow-hidden flex">
            <!-- Migration list -->
            <div class="w-1/2 border-r border-border overflow-y-auto">
                <div v-if="loading" class="p-4 text-center text-muted-foreground">
                    Loading...
                </div>
                
                <div v-else-if="migrations.length === 0" class="p-4 text-center text-muted-foreground">
                    No migrations found
                </div>
                
                <div v-else>
                    <div
                        v-for="migration in migrations"
                        :key="migration.id"
                        @click="selectedMigration = migration"
                        class="p-4 border-b border-border cursor-pointer hover:bg-muted/50"
                        :class="{ 'bg-muted/50': selectedMigration?.id === migration.id }"
                    >
                        <div class="flex items-center justify-between mb-2">
                            <span class="font-medium">{{ migration.name }}</span>
                            <span :class="['text-xs px-2 py-0.5 rounded', getStatusColor(migration.status)]">
                                {{ migration.status }}
                            </span>
                        </div>
                        <p v-if="migration.applied_at" class="text-xs text-muted-foreground">
                            Applied: {{ formatDate(migration.applied_at) }}
                        </p>
                    </div>
                </div>
            </div>
            
            <!-- Migration details -->
            <div class="w-1/2 p-4 overflow-y-auto">
                <div v-if="!selectedMigration" class="text-center text-muted-foreground py-8">
                    Select a migration to view details
                </div>
                
                <div v-else class="space-y-4">
                    <div class="flex items-center justify-between">
                        <h3 class="font-medium">{{ selectedMigration.name }}</h3>
                        <div class="flex gap-2">
                            <button
                                v-if="selectedMigration.status === 'pending'"
                                @click="applyMigration(selectedMigration)"
                                :disabled="isApplying"
                                class="px-3 py-1 text-sm bg-green-500 text-white rounded hover:opacity-90 disabled:opacity-50"
                            >
                                {{ isApplying ? 'Applying...' : 'Apply' }}
                            </button>
                            <button
                                v-if="selectedMigration.status === 'applied'"
                                @click="rollbackMigration(selectedMigration)"
                                :disabled="isApplying"
                                class="px-3 py-1 text-sm bg-yellow-500 text-white rounded hover:opacity-90 disabled:opacity-50"
                            >
                                {{ isApplying ? 'Rolling back...' : 'Rollback' }}
                            </button>
                        </div>
                    </div>
                    
                    <div v-if="error" class="p-3 bg-destructive/10 text-destructive rounded text-sm">
                        {{ error }}
                    </div>
                    
                    <div>
                        <h4 class="text-sm font-medium mb-2">Migration Up (Apply)</h4>
                        <pre class="p-3 bg-muted rounded text-xs font-mono overflow-x-auto whitespace-pre-wrap">{{ selectedMigration.sql_up }}</pre>
                    </div>
                    
                    <div>
                        <h4 class="text-sm font-medium mb-2">Migration Down (Rollback)</h4>
                        <pre class="p-3 bg-muted rounded text-xs font-mono overflow-x-auto whitespace-pre-wrap">{{ selectedMigration.sql_down }}</pre>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>
