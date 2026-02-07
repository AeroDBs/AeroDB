/**
 * Setup API Module
 *
 * HTTP client for setup wizard endpoints.
 * These endpoints are only accessible before setup is complete.
 */

import axios from 'axios'

const baseURL = import.meta.env.VITE_AERODB_URL || 'http://localhost:54321'

// Types
export type SetupStatus = 'Uninitialized' | 'InProgress' | 'Ready'

export interface StatusResponse {
    status: SetupStatus
    storage_configured: boolean
    auth_configured: boolean
    admin_created: boolean
}

export interface StorageConfig {
    data_dir: string
    wal_dir: string
    snapshot_dir: string
}

export interface StorageResponse {
    success: boolean
    data_dir: string
    wal_dir: string
    snapshot_dir: string
    data_dir_exists?: boolean
    wal_dir_exists?: boolean
    snapshot_dir_exists?: boolean
}

export interface AuthConfig {
    jwt_expiry_hours: number
    refresh_expiry_days: number
    password_min_length: number
    require_uppercase: boolean
    require_number: boolean
    require_special: boolean
}

export interface AuthResponse {
    success: boolean
    jwt_expiry_hours: number
    refresh_expiry_days: number
}

export interface AdminConfig {
    email: string
    password: string
    confirm_password: string
}

export interface AdminResponse {
    success: boolean
    email: string
    message: string
}

export interface CompleteResponse {
    success: boolean
    status: SetupStatus
    message: string
}

export interface ErrorResponse {
    error: string
    code: number
}

// API Client
export const setupApi = {
    /**
     * Get current setup status
     */
    async getStatus(): Promise<StatusResponse> {
        const { data } = await axios.get<StatusResponse>(`${baseURL}/setup/status`)
        return data
    },

    /**
     * Configure storage directories
     */
    async configureStorage(config: StorageConfig): Promise<StorageResponse> {
        const { data } = await axios.post<StorageResponse>(`${baseURL}/setup/storage`, config)
        return data
    },

    /**
     * Configure authentication settings
     */
    async configureAuth(config: AuthConfig): Promise<AuthResponse> {
        const { data } = await axios.post<AuthResponse>(`${baseURL}/setup/auth`, config)
        return data
    },

    /**
     * Create first admin user
     */
    async createAdmin(config: AdminConfig): Promise<AdminResponse> {
        const { data } = await axios.post<AdminResponse>(`${baseURL}/setup/admin`, config)
        return data
    },

    /**
     * Complete setup and lock configuration
     */
    async complete(): Promise<CompleteResponse> {
        const { data } = await axios.post<CompleteResponse>(`${baseURL}/setup/complete`)
        return data
    },
}
