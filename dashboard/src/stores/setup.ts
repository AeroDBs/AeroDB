import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { setupApi, type SetupStatus, type StorageConfig, type AuthConfig, type AdminConfig } from '@/setup/api'

export const useSetupStore = defineStore('setup', () => {
    // State
    const status = ref<SetupStatus>('Uninitialized')
    const statusChecked = ref(false)
    const currentStep = ref(1)
    const loading = ref(false)
    const error = ref<string | null>(null)

    // Configuration state
    const storageConfig = ref<StorageConfig | null>(null)
    const authConfig = ref<AuthConfig | null>(null)
    const adminEmail = ref<string | null>(null)

    // Progress tracking
    const storageConfigured = ref(false)
    const authConfigured = ref(false)
    const adminCreated = ref(false)

    // Getters
    const isReady = computed(() => status.value === 'Ready')
    const isUninitialized = computed(() => status.value === 'Uninitialized')
    const isInProgress = computed(() => status.value === 'InProgress')
    const allStepsComplete = computed(() => storageConfigured.value && authConfigured.value && adminCreated.value)

    // Step info
    const steps = [
        { id: 1, name: 'Welcome', path: '/setup/welcome' },
        { id: 2, name: 'Storage', path: '/setup/storage' },
        { id: 3, name: 'Authentication', path: '/setup/auth' },
        { id: 4, name: 'Admin User', path: '/setup/admin' },
        { id: 5, name: 'Review', path: '/setup/review' },
        { id: 6, name: 'Complete', path: '/setup/complete' },
    ]

    // Actions
    const fetchStatus = async (): Promise<void> => {
        loading.value = true
        error.value = null

        try {
            const response = await setupApi.getStatus()
            status.value = response.status
            storageConfigured.value = response.storage_configured
            authConfigured.value = response.auth_configured
            adminCreated.value = response.admin_created
            statusChecked.value = true
        } catch (err: unknown) {
            error.value = err instanceof Error ? err.message : 'Failed to fetch setup status'
            // Default to uninitialized on error
            status.value = 'Uninitialized'
            statusChecked.value = true
        } finally {
            loading.value = false
        }
    }

    const saveStorage = async (config: StorageConfig): Promise<void> => {
        loading.value = true
        error.value = null

        try {
            await setupApi.configureStorage(config)
            storageConfig.value = config
            storageConfigured.value = true
            status.value = 'InProgress'
        } catch (err: unknown) {
            error.value = err instanceof Error ? err.message : 'Failed to configure storage'
            throw err
        } finally {
            loading.value = false
        }
    }

    const saveAuth = async (config: AuthConfig): Promise<void> => {
        loading.value = true
        error.value = null

        try {
            await setupApi.configureAuth(config)
            authConfig.value = config
            authConfigured.value = true
        } catch (err: unknown) {
            error.value = err instanceof Error ? err.message : 'Failed to configure auth'
            throw err
        } finally {
            loading.value = false
        }
    }

    const createAdmin = async (config: AdminConfig): Promise<void> => {
        loading.value = true
        error.value = null

        try {
            await setupApi.createAdmin(config)
            adminEmail.value = config.email
            adminCreated.value = true
        } catch (err: unknown) {
            error.value = err instanceof Error ? err.message : 'Failed to create admin'
            throw err
        } finally {
            loading.value = false
        }
    }

    const completeSetup = async (): Promise<void> => {
        loading.value = true
        error.value = null

        try {
            await setupApi.complete()
            status.value = 'Ready'
        } catch (err: unknown) {
            error.value = err instanceof Error ? err.message : 'Failed to complete setup'
            throw err
        } finally {
            loading.value = false
        }
    }

    const clearError = (): void => {
        error.value = null
    }

    const goToStep = (step: number): void => {
        if (step >= 1 && step <= steps.length) {
            currentStep.value = step
        }
    }

    const nextStep = (): void => {
        if (currentStep.value < steps.length) {
            currentStep.value++
        }
    }

    const prevStep = (): void => {
        if (currentStep.value > 1) {
            currentStep.value--
        }
    }

    return {
        // State
        status,
        statusChecked,
        currentStep,
        loading,
        error,
        storageConfig,
        authConfig,
        adminEmail,
        storageConfigured,
        authConfigured,
        adminCreated,

        // Getters
        isReady,
        isUninitialized,
        isInProgress,
        allStepsComplete,
        steps,

        // Actions
        fetchStatus,
        saveStorage,
        saveAuth,
        createAdmin,
        completeSetup,
        clearError,
        goToStep,
        nextStep,
        prevStep,
    }
})
