// Core TypeScript interfaces for AeroDB Admin Dashboard

export interface User {
    id: string
    email: string
    name?: string
    role: string
    metadata?: Record<string, unknown>
    email_verified?: boolean
    created_at: string
    last_login?: string
}

export interface AuthTokens {
    access_token: string
    refresh_token?: string
    expires_in: number
}

export interface PasswordPolicy {
    min_length: number
    require_uppercase: boolean
    require_lowercase: boolean
    require_numbers: boolean
    require_symbols: boolean
}


export interface ApiError {
    message: string
    code?: string
    details?: unknown
}

export interface TableRow {
    [key: string]: unknown
}

export interface TableData {
    rows: TableRow[]
    columns: string[]
    total: number
    limit: number
    offset: number
}

export interface Filter {
    field: string
    operator: 'eq' | 'gt' | 'lt' | 'gte' | 'lte' | 'like' | 'in'
    value: string | number | boolean
}

export interface Session {
    id: string
    user_id: string
    device?: string
    last_active: string
    expires_at: string
}

export interface Bucket {
    name: string
    public: boolean
    created_at: string
    updated_at: string
}

export interface StorageFile {
    name: string
    size: number
    content_type: string
    created_at: string
    updated_at: string
}

export interface Subscription {
    id: string
    user_id: string
    channel: string
    filter?: Record<string, unknown>
    connected_at: string
}

export interface ClusterNode {
    id: string
    role: 'authority' | 'replica'
    status: 'online' | 'offline'
    replication_lag?: number
}

export interface LogEntry {
    timestamp: string
    level: 'debug' | 'info' | 'warn' | 'error'
    module: string
    message: string
}

export interface MetricDataPoint {
    timestamp: string
    value: number
}

// ========== Functions/Serverless Types ==========

export interface Function {
    id: string
    name: string
    runtime: 'deno' | 'wasm'
    code: string
    env_vars?: Record<string, string>
    triggers: Array<{
        type: 'http' | 'cron' | 'event'
        config: Record<string, unknown>
    }>
    enabled: boolean
    created_at: string
    updated_at: string
    last_invoked_at?: string
}

export interface FunctionLog {
    id: string
    function_id: string
    invocation_id: string
    level: 'debug' | 'info' | 'warn' | 'error'
    message: string
    timestamp: string
}

export interface FunctionInvocation {
    id: string
    function_id: string
    payload: Record<string, unknown>
    result?: unknown
    error?: string
    duration_ms: number
    status: 'success' | 'error' | 'timeout'
    invoked_at: string
}

// ========== Backup & Restore Types ==========

export interface BackupManifest {
    version: string
    timestamp: string
    database_name: string
    tables: string[]
    wal_position: number
    snapshot_id?: string
    compression: 'none' | 'gzip' | 'zstd'
    incremental: boolean
    base_backup_id?: string
}

export interface BackupJob {
    id: string
    status: 'pending' | 'running' | 'completed' | 'failed'
    progress: number
    manifest?: BackupManifest
    error?: string
    started_at: string
    completed_at?: string
}

export interface RestoreJob {
    id: string
    backup_id: string
    status: 'pending' | 'validating' | 'restoring' | 'completed' | 'failed'
    progress: number
    error?: string
    validation_errors?: string[]
    started_at: string
    completed_at?: string
}

export interface BackupInfo {
    id: string
    name: string
    created_at: string
    size_bytes: number
    backup_type: string
    status: string
}

export interface BackupSchedule {
    enabled: boolean
    cron_expression: string
    retention_days: number
    backup_type: string
    last_run?: string
    next_run?: string
}

export interface BackupStats {
    total_backups: number
    total_size_bytes: number
    last_backup_at?: string
    scheduled_backups_count: number
    failed_backups_24h: number
}

// ========== Snapshot & Checkpoint Types ==========

export interface Snapshot {
    id: string
    timestamp: string
    wal_position: number
    size_bytes: number
    manifest: SnapshotManifest
    created_at: string
}

export interface SnapshotManifest {
    snapshot_id: string
    timestamp: string
    wal_position: number
    tables: Array<{
        name: string
        row_count: number
        size_bytes: number
    }>
    checksum: string
}

export interface Checkpoint {
    id: string
    wal_position: number
    timestamp: string
    status: 'pending' | 'in_progress' | 'completed' | 'failed'
}

// ========== Promotion & Replication Types ==========

export interface PromotionRequest {
    id: string
    node_id: string
    status: 'pending' | 'validating' | 'promoting' | 'completed' | 'failed'
    requested_at: string
    completed_at?: string
    error?: string
}

export interface PromotionState {
    current_state: 'idle' | 'validating' | 'draining' | 'marking' | 'transitioning' | 'completed'
    target_node_id: string
    started_at: string
    progress: number
    error?: string
}

export interface ReplicationStatus {
    node_id: string
    role: 'authority' | 'replica'
    status: 'online' | 'offline' | 'syncing' | 'paused'
    wal_position: number
    replication_lag_ms: number
    last_sync_at?: string
    sync_mode: 'sync' | 'async'
}

// ========== Enhanced Observability Types ==========

export interface LogFilter {
    level?: 'debug' | 'info' | 'warn' | 'error'
    module?: string
    search?: string
    since?: string
    until?: string
}

export interface MetricType {
    name: string
    description: string
    unit: string
    category: 'system' | 'database' | 'replication' | 'performance'
}

// ========== Control Plane (Multi-Tenant) Types ==========

export interface Tenant {
    id: string
    name: string
    admin_email: string
    plan: 'free' | 'pro' | 'enterprise'
    status: 'active' | 'suspended' | 'deleted'
    created_at: string
    updated_at: string
}

export interface TenantDetails extends Tenant {
    isolation_model: 'schema' | 'database' | 'cluster'
    database_name?: string
    schema_name?: string
    metadata?: Record<string, unknown>
}

export interface TenantUsage {
    tenant_id: string
    api_requests: number
    storage_bytes: number
    database_rows: number
    active_connections: number
    bandwidth_bytes: number
    period_start: string
    period_end: string
}

export interface TenantQuota {
    api_requests_per_month: number
    storage_bytes: number
    max_connections: number
    max_databases: number
    max_tables_per_database: number
    max_rows_per_table: number
}

export interface Invoice {
    id: string
    tenant_id: string
    period_start: string
    period_end: string
    amount_cents: number
    currency: string
    status: 'draft' | 'pending' | 'paid' | 'overdue'
    line_items: Array<{
        description: string
        quantity: number
        unit_price_cents: number
        total_cents: number
    }>
    generated_at: string
}

