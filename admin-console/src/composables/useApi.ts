import axios, { AxiosInstance, AxiosError } from 'axios'
import { useAuthStore } from '@/stores/auth'

const baseURL = import.meta.env.VITE_AERODB_URL || 'http://localhost:54321'

let apiInstance: AxiosInstance | null = null

export const useApi = () => {
    if (!apiInstance) {
        apiInstance = axios.create({
            baseURL,
            timeout: 30000,
            headers: {
                'Content-Type': 'application/json',
            },
        })

        // Request interceptor: Attach JWT token
        apiInstance.interceptors.request.use(
            (config) => {
                const authStore = useAuthStore()
                if (authStore.token) {
                    config.headers.Authorization = `Bearer ${authStore.token}`
                }
                return config
            },
            (error) => Promise.reject(error)
        )

        // Response interceptor: Handle 401 errors and refresh tokens
        apiInstance.interceptors.response.use(
            (response) => response,
            async (error: AxiosError) => {
                const authStore = useAuthStore()

                if (error.response?.status === 401 && authStore.token) {
                    // Token expired, try to refresh
                    try {
                        await authStore.refreshToken()
                        // Retry original request with new token
                        if (error.config) {
                            return apiInstance!(error.config)
                        }
                    } catch (refreshError) {
                        // Refresh failed, redirect to login
                        authStore.signOut()
                        window.location.href = '/login'
                        return Promise.reject(refreshError)
                    }
                }

                // Log errors in development
                if (import.meta.env.DEV) {
                    console.error('API Error:', {
                        url: error.config?.url,
                        method: error.config?.method,
                        status: error.response?.status,
                        message: error.response?.data || error.message,
                    })
                }

                return Promise.reject(error)
            }
        )
    }

    return {
        api: apiInstance,
        baseURL,
    }
}

// Helper for checking if error is an API error
export const isApiError = (error: unknown): error is AxiosError => {
    return axios.isAxiosError(error)
}

// Helper for extracting error message
export const getErrorMessage = (error: unknown): string => {
    if (isApiError(error)) {
        return (error.response?.data as any)?.message || (error as any).message
    }
    if (error instanceof Error) {
        return error.message
    }
    return 'An unknown error occurred'
}
