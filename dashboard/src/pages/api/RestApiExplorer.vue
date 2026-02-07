<script setup lang="ts">
import { ref } from 'vue'
import { useQuery, useMutation, useQueryClient } from '@tanstack/vue-query'
import AppLayout from '@/components/layout/AppLayout.vue'
import { restApiService, type ApiRequestResult } from '@/services/restApi'

const queryClient = useQueryClient()

// Tabs
const activeTab = ref<'explorer' | 'keys' | 'docs'>('explorer')

// Explorer state
const selectedCollection = ref<string>('')
const requestMethod = ref<'GET' | 'POST' | 'PATCH' | 'DELETE'>('GET')
const requestPath = ref('/api/')
const requestBody = ref('')
const requestResponse = ref<ApiRequestResult | null>(null)
const isExecuting = ref(false)

// Collections query
const { data: collections } = useQuery({
    queryKey: ['rest-collections'],
    queryFn: () => restApiService.getCollections(),
})

// API Keys query
const { data: apiKeys, isLoading: keysLoading } = useQuery({
    queryKey: ['api-keys'],
    queryFn: () => restApiService.getApiKeys(),
})

// Create key state
const showCreateKeyDialog = ref(false)
const newKeyName = ref('')
const newKeyPermissions = ref<string[]>(['read'])
const newKeySecret = ref('')

// Create key mutation
const createKeyMutation = useMutation({
    mutationFn: (data: { name: string; permissions: string[] }) => 
        restApiService.createApiKey(data),
    onSuccess: (result) => {
        newKeySecret.value = result.secret
        queryClient.invalidateQueries({ queryKey: ['api-keys'] })
    },
})

// Revoke key mutation
const revokeKeyMutation = useMutation({
    mutationFn: (keyId: string) => restApiService.revokeApiKey(keyId),
    onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: ['api-keys'] })
    },
})

// Sync path with selected collection
const updatePathFromCollection = () => {
    if (selectedCollection.value) {
        requestPath.value = `/api/${selectedCollection.value}`
    }
}

// Execute API request
const executeRequest = async () => {
    isExecuting.value = true
    requestResponse.value = null
    
    try {
        let body: unknown = undefined
        if (requestBody.value && ['POST', 'PATCH'].includes(requestMethod.value)) {
            try {
                body = JSON.parse(requestBody.value)
            } catch {
                requestResponse.value = {
                    status: 0,
                    headers: {},
                    body: { error: 'Invalid JSON in request body' },
                    duration_ms: 0,
                }
                return
            }
        }
        
        requestResponse.value = await restApiService.executeRequest(
            requestMethod.value,
            requestPath.value,
            body
        )
    } catch (error) {
        requestResponse.value = {
            status: 0,
            headers: {},
            body: { error: String(error) },
            duration_ms: 0,
        }
    } finally {
        isExecuting.value = false
    }
}

const handleCreateKey = () => {
    createKeyMutation.mutate({
        name: newKeyName.value,
        permissions: newKeyPermissions.value,
    })
}

const closeCreateDialog = () => {
    showCreateKeyDialog.value = false
    newKeyName.value = ''
    newKeyPermissions.value = ['read']
    newKeySecret.value = ''
}

const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text)
}

const formatJson = (data: unknown): string => {
    try {
        return JSON.stringify(data, null, 2)
    } catch {
        return String(data)
    }
}

const getStatusColor = (status: number): string => {
    if (status >= 200 && status < 300) return 'text-green-500'
    if (status >= 400 && status < 500) return 'text-yellow-500'
    if (status >= 500) return 'text-red-500'
    return 'text-muted-foreground'
}
</script>

<template>
    <AppLayout>
        <div class="space-y-6">
            <div class="flex items-center justify-between">
                <h1 class="text-3xl font-bold">REST API</h1>
            </div>

            <!-- Tabs -->
            <div class="border-b border-border">
                <nav class="flex gap-4">
                    <button 
                        @click="activeTab = 'explorer'"
                        class="px-4 py-2 font-medium transition-colors border-b-2"
                        :class="activeTab === 'explorer' ? 'border-primary text-primary' : 'border-transparent text-muted-foreground hover:text-foreground'"
                    >
                        API Explorer
                    </button>
                    <button 
                        @click="activeTab = 'keys'"
                        class="px-4 py-2 font-medium transition-colors border-b-2"
                        :class="activeTab === 'keys' ? 'border-primary text-primary' : 'border-transparent text-muted-foreground hover:text-foreground'"
                    >
                        API Keys
                    </button>
                    <button 
                        @click="activeTab = 'docs'"
                        class="px-4 py-2 font-medium transition-colors border-b-2"
                        :class="activeTab === 'docs' ? 'border-primary text-primary' : 'border-transparent text-muted-foreground hover:text-foreground'"
                    >
                        Documentation
                    </button>
                </nav>
            </div>

            <!-- Explorer Tab -->
            <div v-if="activeTab === 'explorer'" class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                <!-- Request Panel -->
                <div class="space-y-4">
                    <div class="bg-card border border-border rounded-lg p-4">
                        <h3 class="font-medium mb-4">Request</h3>
                        
                        <!-- Collection Selector -->
                        <div class="mb-4">
                            <label class="block text-sm text-muted-foreground mb-2">Collection</label>
                            <select
                                v-model="selectedCollection"
                                @change="updatePathFromCollection"
                                class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
                            >
                                <option value="">Select collection...</option>
                                <option v-for="col in collections" :key="col.name" :value="col.name">
                                    {{ col.name }} ({{ col.row_count }} rows)
                                </option>
                            </select>
                        </div>
                        
                        <!-- Method + Path -->
                        <div class="flex gap-2 mb-4">
                            <select
                                v-model="requestMethod"
                                class="px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
                            >
                                <option value="GET">GET</option>
                                <option value="POST">POST</option>
                                <option value="PATCH">PATCH</option>
                                <option value="DELETE">DELETE</option>
                            </select>
                            <input
                                v-model="requestPath"
                                type="text"
                                placeholder="/api/collection"
                                class="flex-1 px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring font-mono"
                            />
                        </div>
                        
                        <!-- Body -->
                        <div v-if="requestMethod === 'POST' || requestMethod === 'PATCH'" class="mb-4">
                            <label class="block text-sm text-muted-foreground mb-2">Body (JSON)</label>
                            <textarea
                                v-model="requestBody"
                                rows="6"
                                placeholder='{"field": "value"}'
                                class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring font-mono text-sm"
                            ></textarea>
                        </div>
                        
                        <button
                            @click="executeRequest"
                            :disabled="isExecuting || !requestPath"
                            class="w-full px-4 py-2 rounded bg-primary text-primary-foreground hover:opacity-90 disabled:opacity-50"
                        >
                            {{ isExecuting ? 'Executing...' : 'Send Request' }}
                        </button>
                    </div>
                </div>

                <!-- Response Panel -->
                <div class="space-y-4">
                    <div class="bg-card border border-border rounded-lg p-4">
                        <h3 class="font-medium mb-4">Response</h3>
                        
                        <div v-if="requestResponse" class="space-y-4">
                            <div class="flex items-center gap-4">
                                <span 
                                    class="font-mono font-bold"
                                    :class="getStatusColor(requestResponse.status)"
                                >
                                    {{ requestResponse.status }}
                                </span>
                                <span class="text-sm text-muted-foreground">
                                    {{ requestResponse.duration_ms }}ms
                                </span>
                            </div>
                            
                            <div class="bg-background border border-input rounded p-3 max-h-96 overflow-auto">
                                <pre class="text-sm font-mono whitespace-pre-wrap">{{ formatJson(requestResponse.body) }}</pre>
                            </div>
                        </div>
                        
                        <div v-else class="text-center text-muted-foreground py-8">
                            No response yet. Send a request to see results.
                        </div>
                    </div>
                </div>
            </div>

            <!-- API Keys Tab -->
            <div v-if="activeTab === 'keys'" class="space-y-4">
                <div class="flex justify-end">
                    <button
                        @click="showCreateKeyDialog = true"
                        class="px-4 py-2 rounded bg-primary text-primary-foreground hover:opacity-90"
                    >
                        + Create API Key
                    </button>
                </div>

                <div class="bg-card border border-border rounded-lg overflow-hidden">
                    <table class="w-full">
                        <thead class="bg-muted/50">
                            <tr>
                                <th class="px-4 py-3 text-left text-sm font-medium">Name</th>
                                <th class="px-4 py-3 text-left text-sm font-medium">Key Prefix</th>
                                <th class="px-4 py-3 text-left text-sm font-medium">Permissions</th>
                                <th class="px-4 py-3 text-left text-sm font-medium">Created</th>
                                <th class="px-4 py-3 text-left text-sm font-medium">Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr v-if="keysLoading">
                                <td colspan="5" class="px-4 py-8 text-center text-muted-foreground">Loading...</td>
                            </tr>
                            <tr v-else-if="!apiKeys?.length">
                                <td colspan="5" class="px-4 py-8 text-center text-muted-foreground">No API keys created yet</td>
                            </tr>
                            <tr v-for="key in apiKeys" :key="key.id" class="border-t border-border">
                                <td class="px-4 py-3">{{ key.name }}</td>
                                <td class="px-4 py-3 font-mono text-sm">{{ key.key_prefix }}...</td>
                                <td class="px-4 py-3">
                                    <span v-for="perm in key.permissions" :key="perm" class="inline-block px-2 py-0.5 bg-muted rounded text-xs mr-1">
                                        {{ perm }}
                                    </span>
                                </td>
                                <td class="px-4 py-3 text-sm text-muted-foreground">{{ new Date(key.created_at).toLocaleDateString() }}</td>
                                <td class="px-4 py-3">
                                    <button 
                                        @click="revokeKeyMutation.mutate(key.id)"
                                        class="text-destructive hover:underline text-sm"
                                    >
                                        Revoke
                                    </button>
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </div>

            <!-- Docs Tab -->
            <div v-if="activeTab === 'docs'" class="bg-card border border-border rounded-lg p-6">
                <h3 class="text-lg font-semibold mb-4">REST API Documentation</h3>
                
                <div class="prose prose-invert max-w-none">
                    <h4 class="text-base font-medium mt-4 mb-2">Endpoints</h4>
                    <p class="text-muted-foreground mb-4">All collections expose auto-generated REST endpoints:</p>
                    
                    <div class="bg-background border border-input rounded-lg p-4 font-mono text-sm space-y-2">
                        <div><span class="text-green-500">GET</span> /api/:collection - List records</div>
                        <div><span class="text-green-500">GET</span> /api/:collection/:id - Get single record</div>
                        <div><span class="text-blue-500">POST</span> /api/:collection - Create record</div>
                        <div><span class="text-yellow-500">PATCH</span> /api/:collection/:id - Update record</div>
                        <div><span class="text-red-500">DELETE</span> /api/:collection/:id - Delete record</div>
                    </div>
                    
                    <h4 class="text-base font-medium mt-6 mb-2">Query Parameters</h4>
                    <ul class="list-disc list-inside text-muted-foreground space-y-1">
                        <li><code class="bg-muted px-1 rounded">filter</code> - Filter records (e.g., filter[status]=active)</li>
                        <li><code class="bg-muted px-1 rounded">select</code> - Select specific fields</li>
                        <li><code class="bg-muted px-1 rounded">order</code> - Sort results (e.g., order=created_at.desc)</li>
                        <li><code class="bg-muted px-1 rounded">limit</code> - Limit number of results</li>
                        <li><code class="bg-muted px-1 rounded">offset</code> - Pagination offset</li>
                    </ul>
                    
                    <h4 class="text-base font-medium mt-6 mb-2">Authentication</h4>
                    <p class="text-muted-foreground">
                        Include your API key in the <code class="bg-muted px-1 rounded">Authorization</code> header:
                    </p>
                    <div class="bg-background border border-input rounded p-3 font-mono text-sm mt-2">
                        Authorization: Bearer your-api-key
                    </div>
                </div>
            </div>
        </div>

        <!-- Create Key Dialog -->
        <div
            v-if="showCreateKeyDialog"
            class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
            @click="closeCreateDialog"
        >
            <div class="bg-card border border-border rounded-lg shadow-lg max-w-md w-full mx-4 p-6" @click.stop>
                <h2 class="text-lg font-semibold mb-4">Create API Key</h2>
                
                <div v-if="newKeySecret" class="space-y-4">
                    <div class="p-4 bg-yellow-500/10 border border-yellow-500/30 rounded-lg">
                        <p class="text-sm text-yellow-500 mb-2">
                            ⚠️ Copy this key now. You won't be able to see it again!
                        </p>
                        <div class="flex items-center gap-2">
                            <code class="flex-1 bg-background px-3 py-2 rounded font-mono text-sm break-all">
                                {{ newKeySecret }}
                            </code>
                            <button
                                @click="copyToClipboard(newKeySecret)"
                                class="px-3 py-2 bg-primary text-primary-foreground rounded hover:opacity-90"
                            >
                                Copy
                            </button>
                        </div>
                    </div>
                    <button @click="closeCreateDialog" class="w-full px-4 py-2 rounded bg-muted hover:bg-muted/80">
                        Done
                    </button>
                </div>
                
                <form v-else @submit.prevent="handleCreateKey" class="space-y-4">
                    <div>
                        <label class="block text-sm font-medium mb-2">Key Name</label>
                        <input
                            v-model="newKeyName"
                            type="text"
                            required
                            placeholder="My API Key"
                            class="w-full px-3 py-2 bg-background border border-input rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
                        />
                    </div>
                    
                    <div>
                        <label class="block text-sm font-medium mb-2">Permissions</label>
                        <div class="space-y-2">
                            <label class="flex items-center gap-2">
                                <input type="checkbox" value="read" v-model="newKeyPermissions" />
                                <span>Read</span>
                            </label>
                            <label class="flex items-center gap-2">
                                <input type="checkbox" value="write" v-model="newKeyPermissions" />
                                <span>Write</span>
                            </label>
                            <label class="flex items-center gap-2">
                                <input type="checkbox" value="delete" v-model="newKeyPermissions" />
                                <span>Delete</span>
                            </label>
                        </div>
                    </div>
                    
                    <div class="flex justify-end gap-3">
                        <button type="button" @click="closeCreateDialog" class="px-4 py-2 rounded border border-border hover:bg-secondary">
                            Cancel
                        </button>
                        <button
                            type="submit"
                            :disabled="!newKeyName || createKeyMutation.isPending.value"
                            class="px-4 py-2 rounded bg-primary text-primary-foreground hover:opacity-90 disabled:opacity-50"
                        >
                            {{ createKeyMutation.isPending.value ? 'Creating...' : 'Create Key' }}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    </AppLayout>
</template>
