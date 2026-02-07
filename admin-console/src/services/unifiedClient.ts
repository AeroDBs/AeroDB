/**
 * Unified Pipeline Client
 * 
 * Single client for all AeroDB operations using the unified endpoint.
 * Replaces multiple service calls with typed operation requests.
 */

import { useApi } from '@/composables/useApi'

const { api } = useApi()

// ============================================================================
// Operation Types
// ============================================================================

export type Operation =
    | ReadOperation
    | WriteOperation
    | UpdateOperation
    | DeleteOperation
    | QueryOperation
    | SubscribeOperation
    | UnsubscribeOperation
    | BroadcastOperation
    | InvokeOperation
    | UploadOperation
    | DownloadOperation

export interface ReadOperation {
    op: 'read'
    collection: string
    id: string
    select?: string[]
}

export interface WriteOperation {
    op: 'write'
    collection: string
    document: Record<string, unknown>
    schema_id: string
    schema_version: string
}

export interface UpdateOperation {
    op: 'update'
    collection: string
    id: string
    updates: Record<string, unknown>
}

export interface DeleteOperation {
    op: 'delete'
    collection: string
    id: string
}

export interface QueryOperation {
    op: 'query'
    collection: string
    filter?: Record<string, unknown>
    select?: string[]
    order?: Array<{ field: string; ascending: boolean }>
    limit: number
    offset: number
}

export interface SubscribeOperation {
    op: 'subscribe'
    channel: string
    filter?: Record<string, unknown>
}

export interface UnsubscribeOperation {
    op: 'unsubscribe'
    subscription_id: string
}

export interface BroadcastOperation {
    op: 'broadcast'
    channel: string
    event: string
    payload: unknown
}

export interface InvokeOperation {
    op: 'invoke'
    function_name: string
    payload: unknown
    async_mode: boolean
}

export interface UploadOperation {
    op: 'upload'
    bucket: string
    path: string
}

export interface DownloadOperation {
    op: 'download'
    bucket: string
    path: string
}

// ============================================================================
// Response Types
// ============================================================================

export interface OperationResponse<T = unknown> {
    success: boolean
    data?: T
    error?: {
        code: string
        message: string
        status: number
    }
}

// ============================================================================
// Unified Client
// ============================================================================

/**
 * Execute any operation through the unified pipeline endpoint.
 * 
 * @example
 * // Read a document
 * const user = await execute<User>({
 *     op: 'read',
 *     collection: 'users',
 *     id: '123'
 * })
 * 
 * @example
 * // Query with filter
 * const posts = await execute<Post[]>({
 *     op: 'query',
 *     collection: 'posts',
 *     filter: { author_id: { $eq: userId } },
 *     limit: 10,
 *     offset: 0
 * })
 */
export async function execute<T = unknown>(operation: Operation): Promise<T> {
    const response = await api.post<OperationResponse<T>>('/api/v1/operation', operation)

    if (!response.data.success) {
        throw new PipelineError(
            response.data.error?.code || 'UNKNOWN',
            response.data.error?.message || 'Unknown error',
            response.data.error?.status || 500
        )
    }

    return response.data.data as T
}

/**
 * Execute with full response (includes metadata)
 */
export async function executeRaw<T = unknown>(operation: Operation): Promise<OperationResponse<T>> {
    const response = await api.post<OperationResponse<T>>('/api/v1/operation', operation)
    return response.data
}

// ============================================================================
// Error Handling
// ============================================================================

export class PipelineError extends Error {
    constructor(
        public code: string,
        message: string,
        public status: number
    ) {
        super(message)
        this.name = 'PipelineError'
    }

    get isNotFound(): boolean {
        return this.status === 404
    }

    get isUnauthorized(): boolean {
        return this.status === 401
    }

    get isForbidden(): boolean {
        return this.status === 403
    }

    get isValidation(): boolean {
        return this.status === 400
    }
}

// ============================================================================
// Convenience Helpers
// ============================================================================

/**
 * Read a single document by ID
 */
export async function read<T>(collection: string, id: string): Promise<T> {
    return execute<T>({
        op: 'read',
        collection,
        id
    })
}

/**
 * Query documents with pagination
 */
export async function query<T>(
    collection: string,
    options?: {
        filter?: Record<string, unknown>
        limit?: number
        offset?: number
        order?: Array<{ field: string; ascending: boolean }>
    }
): Promise<T[]> {
    const result = await execute<{ data: T[] }>({
        op: 'query',
        collection,
        filter: options?.filter,
        limit: options?.limit || 50,
        offset: options?.offset || 0,
        order: options?.order
    })
    return result.data || []
}

/**
 * Write a new document
 */
export async function write<T>(
    collection: string,
    document: Record<string, unknown>,
    schemaId = 'default',
    schemaVersion = '1.0'
): Promise<T> {
    return execute<T>({
        op: 'write',
        collection,
        document,
        schema_id: schemaId,
        schema_version: schemaVersion
    })
}

/**
 * Update an existing document
 */
export async function update<T>(
    collection: string,
    id: string,
    updates: Record<string, unknown>
): Promise<T> {
    return execute<T>({
        op: 'update',
        collection,
        id,
        updates
    })
}

/**
 * Delete a document
 */
export async function remove(collection: string, id: string): Promise<void> {
    await execute({
        op: 'delete',
        collection,
        id
    })
}

/**
 * Invoke a serverless function
 */
export async function invoke<T>(
    functionName: string,
    payload: unknown = {},
    asyncMode = false
): Promise<T> {
    return execute<T>({
        op: 'invoke',
        function_name: functionName,
        payload,
        async_mode: asyncMode
    })
}

// Export the unified client as a namespace
export const unifiedClient = {
    execute,
    executeRaw,
    read,
    query,
    write,
    update,
    remove,
    invoke
}

export default unifiedClient
