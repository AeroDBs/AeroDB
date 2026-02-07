import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import axios from 'axios'
import type { User, AuthTokens } from '@/types'

const baseURL = import.meta.env.VITE_AERODB_URL || 'http://localhost:54321'

export const useAuthStore = defineStore('auth', () => {
    // State
    const token = ref<string | null>(localStorage.getItem('access_token'))
    const refreshTokenValue = ref<string | null>(localStorage.getItem('refresh_token'))
    const user = ref<User | null>(null)
    const loading = ref(false)
    const error = ref<string | null>(null)

    // Getters
    const isAuthenticated = computed(() => !!token.value)

    // Actions
    const signIn = async (email: string, password: string): Promise<void> => {
        loading.value = true
        error.value = null

        try {
            const { data } = await axios.post<AuthTokens>(`${baseURL}/auth/login`, {
                email,
                password,
            })

            // Store tokens
            token.value = data.access_token
            localStorage.setItem('access_token', data.access_token)

            if (data.refresh_token) {
                refreshTokenValue.value = data.refresh_token
                localStorage.setItem('refresh_token', data.refresh_token)
            }

            // Fetch user details
            await fetchUser()
        } catch (err: unknown) {
            if (axios.isAxiosError(err)) {
                error.value = err.response?.data?.message || 'Login failed'
            } else {
                error.value = 'An unexpected error occurred'
            }
            throw err
        } finally {
            loading.value = false
        }
    }

    const signOut = (): void => {
        token.value = null
        refreshTokenValue.value = null
        user.value = null
        localStorage.removeItem('access_token')
        localStorage.removeItem('refresh_token')
    }

    const fetchUser = async (): Promise<void> => {
        if (!token.value) {
            throw new Error('No token available')
        }

        try {
            const { data } = await axios.get<User>(`${baseURL}/auth/user`, {
                headers: {
                    Authorization: `Bearer ${token.value}`,
                },
            })
            user.value = data
        } catch (err) {
            console.error('Failed to fetch user:', err)
            throw err
        }
    }

    const refreshToken = async (): Promise<void> => {
        if (!refreshTokenValue.value) {
            throw new Error('No refresh token available')
        }

        try {
            const { data } = await axios.post<AuthTokens>(`${baseURL}/auth/refresh`, {
                refresh_token: refreshTokenValue.value,
            })

            token.value = data.access_token
            localStorage.setItem('access_token', data.access_token)

            if (data.refresh_token) {
                refreshTokenValue.value = data.refresh_token
                localStorage.setItem('refresh_token', data.refresh_token)
            }
        } catch (err) {
            // Refresh failed, sign out
            signOut()
            throw err
        }
    }

    // Initialize: Try to fetch user if token exists
    const initialize = async (): Promise<void> => {
        if (token.value) {
            try {
                await fetchUser()
            } catch (err) {
                // Token might be invalid, clear it
                signOut()
            }
        }
    }

    return {
        // State
        token,
        accessToken: token, // Alias for compatibility
        user,
        loading,
        error,

        // Getters
        isAuthenticated,

        // Actions
        signIn,
        signOut,
        fetchUser,
        refreshToken,
        initialize,
    }
})
