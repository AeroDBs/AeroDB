<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { functionsService } from '@/services'

const router = useRouter()

const emit = defineEmits<{
    (e: 'close'): void
}>()

const templates = ref<Array<{
    id: string
    name: string
    description: string
    runtime: 'deno' | 'wasm'
    code: string
    category: string
}>>([])

const loading = ref(false)
const creating = ref(false)
const error = ref<string | null>(null)
const selectedTemplate = ref<string | null>(null)
const newFunctionName = ref('')

const categories = new Set<string>()

onMounted(async () => {
    loading.value = true
    try {
        templates.value = await functionsService.getTemplates()
        templates.value.forEach(t => categories.add(t.category))
    } catch (e) {
        error.value = 'Failed to load templates'
    } finally {
        loading.value = false
    }
})

const getTemplatesByCategory = (category: string) => {
    return templates.value.filter(t => t.category === category)
}

const selectTemplate = (templateId: string) => {
    selectedTemplate.value = templateId
    const template = templates.value.find(t => t.id === templateId)
    if (template) {
        newFunctionName.value = template.name.toLowerCase().replace(/\s+/g, '-')
    }
}

const createFromTemplate = async () => {
    if (!selectedTemplate.value || !newFunctionName.value) return
    
    creating.value = true
    try {
        const func = await functionsService.createFromTemplate(selectedTemplate.value, newFunctionName.value)
        emit('close')
        router.push(`/functions/${func.id}/edit`)
    } catch (e) {
        error.value = 'Failed to create function'
    } finally {
        creating.value = false
    }
}

const selectedTemplateData = () => {
    return templates.value.find(t => t.id === selectedTemplate.value)
}
</script>

<template>
    <div
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
        @click="emit('close')"
    >
        <div
            class="bg-card border border-border rounded-lg shadow-lg max-w-4xl w-full mx-4 max-h-[90vh] flex flex-col"
            @click.stop
        >
            <div class="p-4 border-b border-border flex items-center justify-between">
                <h2 class="text-lg font-semibold">Function Templates</h2>
                <button @click="emit('close')" class="p-2 hover:bg-muted rounded">✕</button>
            </div>
            
            <div v-if="loading" class="flex-1 flex items-center justify-center text-muted-foreground">
                Loading templates...
            </div>
            
            <div v-else class="flex-1 overflow-hidden flex">
                <!-- Template list -->
                <div class="w-1/2 border-r border-border overflow-y-auto p-4">
                    <div v-for="category in Array.from(categories)" :key="category" class="mb-6">
                        <h3 class="font-medium text-sm text-muted-foreground uppercase tracking-wide mb-3">
                            {{ category }}
                        </h3>
                        <div class="space-y-2">
                            <button
                                v-for="template in getTemplatesByCategory(category)"
                                :key="template.id"
                                @click="selectTemplate(template.id)"
                                class="w-full p-3 text-left rounded-lg border transition-colors"
                                :class="selectedTemplate === template.id 
                                    ? 'border-primary bg-primary/5' 
                                    : 'border-border hover:bg-muted/50'"
                            >
                                <p class="font-medium">{{ template.name }}</p>
                                <p class="text-sm text-muted-foreground truncate">{{ template.description }}</p>
                                <span class="inline-block mt-2 text-xs px-2 py-0.5 bg-muted rounded">
                                    {{ template.runtime }}
                                </span>
                            </button>
                        </div>
                    </div>
                </div>
                
                <!-- Preview -->
                <div class="w-1/2 p-4 overflow-y-auto">
                    <div v-if="!selectedTemplate" class="text-center text-muted-foreground py-12">
                        Select a template to preview
                    </div>
                    
                    <div v-else-if="selectedTemplateData()">
                        <h3 class="font-medium mb-2">{{ selectedTemplateData()?.name }}</h3>
                        <p class="text-sm text-muted-foreground mb-4">
                            {{ selectedTemplateData()?.description }}
                        </p>
                        
                        <div class="bg-muted rounded-lg p-3 mb-4">
                            <pre class="text-xs font-mono overflow-x-auto whitespace-pre-wrap">{{ selectedTemplateData()?.code }}</pre>
                        </div>
                        
                        <div class="space-y-3">
                            <div>
                                <label class="block text-sm font-medium mb-1">Function Name</label>
                                <input
                                    v-model="newFunctionName"
                                    type="text"
                                    placeholder="my-function"
                                    class="w-full px-3 py-2 bg-background border border-input rounded-md"
                                />
                            </div>
                            
                            <button
                                @click="createFromTemplate"
                                :disabled="creating || !newFunctionName"
                                class="w-full px-4 py-2 bg-primary text-primary-foreground rounded hover:opacity-90 disabled:opacity-50"
                            >
                                {{ creating ? 'Creating...' : 'Create Function' }}
                            </button>
                        </div>
                    </div>
                </div>
            </div>
            
            <div v-if="error" class="p-4 border-t border-border">
                <div class="p-3 bg-destructive/10 text-destructive rounded text-sm">
                    {{ error }}
                </div>
            </div>
        </div>
    </div>
</template>
