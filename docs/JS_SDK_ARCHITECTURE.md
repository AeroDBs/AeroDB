# Phase 14: Client SDKs - JavaScript/TypeScript Architecture

## Package Structure

```
@aerodb/client/
├── src/
│   ├── index.ts                 # Main entry point
│   ├── AeroDBClient.ts          # Client class
│   ├── auth/                    # Authentication
│   │   ├── AuthClient.ts
│   │   └── types.ts
│   ├── database/                # CRUD operations
│   │   ├── QueryBuilder.ts
│   │   ├── PostgrestClient.ts   # PostgREST-style API
│   │   └── types.ts
│   ├── realtime/                # WebSocket
│   │   ├── RealtimeClient.ts
│   │   ├── RealtimeChannel.ts
│   │   └── types.ts
│   ├── storage/                 # File uploads
│   │   ├── StorageClient.ts
│   │   └── types.ts
│   ├── functions/               # Serverless
│   │   ├── FunctionsClient.ts
│   │   └── types.ts
│   ├── lib/                     # Shared utilities
│   │   ├── fetch.ts
│   │   ├── constants.ts
│   │   └── helpers.ts
│   └── types/                   # Global types
│       └── index.ts
├── tests/                       # Unit tests
├── package.json
├── tsconfig.json
└── README.md
```

---

## Core Classes

### AeroDBClient

Main entry point, exposes all sub-clients:

```typescript
// src/AeroDBClient.ts
import { AuthClient } from './auth/AuthClient';
import { PostgrestClient } from './database/PostgrestClient';
import { RealtimeClient } from './realtime/RealtimeClient';
import { StorageClient } from './storage/StorageClient';
import { FunctionsClient } from './functions/FunctionsClient';

export interface AeroDBClientOptions {
  url: string;              // Base URL (e.g., https://api.aerodb.com)
  key?: string;             // API key (optional, can use signIn)
  schema?: string;          // Database schema (default: 'public')
  headers?: Record<string, string>;
  realtime?: { url: string }; // WebSocket URL override
}

export class AeroDBClient {
  auth: AuthClient;
  private db: PostgrestClient;
  realtime: RealtimeClient;
  storage: StorageClient;
  functions: FunctionsClient;
  
  constructor(options: AeroDBClientOptions) {
    this.auth = new AuthClient(options);
    this.db = new PostgrestClient(options);
    this.realtime = new RealtimeClient(options);
    this.storage = new StorageClient(options);
    this.functions = new FunctionsClient(options);
  }
  
  // Shorthand for database queries
  from<T = any>(collection: string) {
    return this.db.from<T>(collection);
  }
  
  // Create a real-time channel
  channel(name: string) {
    return this.realtime.channel(name);
  }
}
```

### AuthClient

```typescript
// src/auth/AuthClient.ts
import type { User, Session, AuthResponse } from './types';

export class AuthClient {
  private url: string;
  
  constructor(options: AeroDBClientOptions) {
    this.url = `${options.url}/auth`;
  }
  
  async signUp(email: string, password: string): Promise<AuthResponse> {
    const res = await fetch(`${this.url}/signup`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email, password }),
    });
    
    const data = await res.json();
    
    if (!res.ok) {
      return { data: null, error: { message: data.error, status: res.status } };
    }
    
    return { data: { user: data.user, session: data.session }, error: null };
  }
  
  async signIn(email: string, password: string): Promise<AuthResponse> {
    const res = await fetch(`${this.url}/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email, password }),
    });
    
    const data = await res.json();
    
    if (!res.ok) {
      return { data: null, error: { message: data.error, status: res.status } };
    }
    
    // Store tokens
    localStorage.setItem('access_token', data.access_token);
    localStorage.setItem('refresh_token', data.refresh_token);
    
    return { data: { user: data.user, session: data.session }, error: null };
  }
  
  async signOut(): Promise<{ error: null | { message: string } }> {
    const token = localStorage.getItem('access_token');
    
    await fetch(`${this.url}/logout`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${token}` },
    });
    
    localStorage.removeItem('access_token');
    localStorage.removeItem('refresh_token');
    
    return { error: null };
  }
  
  async getUser(): Promise<{ data: User | null; error: any }> {
    const token = localStorage.getItem('access_token');
    
    if (!token) {
      return { data: null, error: { message: 'Not authenticated' } };
    }
    
    const res = await fetch(`${this.url}/user`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    
    if (!res.ok) {
      return { data: null, error: { message: 'Failed to fetch user' } };
    }
    
    const user = await res.json();
    return { data: user, error: null };
  }
  
  onAuthStateChange(callback: (event: 'SIGNED_IN' | 'SIGNED_OUT', session: Session | null) => void) {
    // Listen for storage events (multi-tab sync)
    window.addEventListener('storage', (e) => {
      if (e.key === 'access_token') {
        if (e.newValue) {
          callback('SIGNED_IN', { access_token: e.newValue });
        } else {
          callback('SIGNED_OUT', null);
        }
      }
    });
  }
}
```

### QueryBuilder

```typescript
// src/database/QueryBuilder.ts
export type FilterOperator = 'eq' | 'neq' | 'gt' | 'gte' | 'lt' | 'lte' | 'like' | 'ilike' | 'in';

export class QueryBuilder<T = any> {
  private collection: string;
  private selectFields: string = '*';
  private filters: Array<{ field: string; op: FilterOperator; value: any }> = [];
  private orderFields: Array<{ field: string; ascending: boolean }> = [];
  private limitValue?: number;
  private offsetValue?: number;
  private baseUrl: string;
  
  constructor(collection: string, baseUrl: string) {
    this.collection = collection;
    this.baseUrl = baseUrl;
  }
  
  select(fields: string = '*'): this {
    this.selectFields = fields;
    return this;
  }
  
  eq(field: keyof T, value: any): this {
    this.filters.push({ field: field as string, op: 'eq', value });
    return this;
  }
  
  neq(field: keyof T, value: any): this {
    this.filters.push({ field: field as string, op: 'neq', value });
    return this;
  }
  
  gt(field: keyof T, value: any): this {
    this.filters.push({ field: field as string, op: 'gt', value });
    return this;
  }
  
  gte(field: keyof T, value: any): this {
    this.filters.push({ field: field as string, op: 'gte', value });
    return this;
  }
  
  lt(field: keyof T, value: any): this {
    this.filters.push({ field: field as string, op: 'lt', value });
    return this;
  }
  
  lte(field: keyof T, value: any): this {
    this.filters.push({ field: field as string, op: 'lte', value });
    return this;
  }
  
  like(field: keyof T, pattern: string): this {
    this.filters.push({ field: field as string, op: 'like', value: pattern });
    return this;
  }
  
  order(field: keyof T, options: { ascending?: boolean } = {}): this {
    this.orderFields.push({ field: field as string, ascending: options.ascending ?? true });
    return this;
  }
  
  limit(count: number): this {
    this.limitValue = count;
    return this;
  }
  
  offset(count: number): this {
    this.offsetValue = count;
    return this;
  }
  
  private buildQueryString(): string {
    const params = new URLSearchParams();
    
    if (this.selectFields) {
      params.set('select', this.selectFields);
    }
    
    this.filters.forEach(({ field, op, value }) => {
      params.set(field, `${op}.${value}`);
    });
    
    if (this.orderFields.length > 0) {
      const order = this.orderFields.map(({ field, ascending }) => 
        `${field}.${ascending ? 'asc' : 'desc'}`
      ).join(',');
      params.set('order', order);
    }
    
    if (this.limitValue) {
      params.set('limit', String(this.limitValue));
    }
    
    if (this.offsetValue) {
      params.set('offset', String(this.offsetValue));
    }
    
    return params.toString();
  }
  
  async execute(): Promise<{ data: T[] | null; error: any }> {
    const token = localStorage.getItem('access_token');
    const queryString = this.buildQueryString();
    const url = `${this.baseUrl}/rest/v1/${this.collection}?${queryString}`;
    
    const res = await fetch(url, {
      headers: token ? { Authorization: `Bearer ${token}` } : {},
    });
    
    const json = await res.json();
    
    if (!res.ok) {
      return { data: null, error: { message: json.error, status: res.status } };
    }
    
    return { data: json.data, error: null };
  }
}
```

### RealtimeChannel

```typescript
// src/realtime/RealtimeChannel.ts
export type RealtimeEvent = 'INSERT' | 'UPDATE' | 'DELETE';

export class RealtimeChannel {
  private name: string;
  private ws: WebSocket;
  private callbacks: Map<RealtimeEvent, Array<(payload: any) => void>> = new Map();
  
  constructor(name: string, ws: WebSocket) {
    this.name = name;
    this.ws = ws;
    
    this.ws.onmessage = (event) => {
      const message = JSON.parse(event.data);
      if (message.type === 'event' && message.channel === this.name) {
        const eventType = message.payload.type as RealtimeEvent;
        const handlers = this.callbacks.get(eventType) || [];
        handlers.forEach((cb) => cb(message.payload));
      }
    };
  }
  
  on(event: RealtimeEvent, callback: (payload: any) => void): this {
    if (!this.callbacks.has(event)) {
      this.callbacks.set(event, []);
    }
    this.callbacks.get(event)!.push(callback);
    return this;
  }
  
  subscribe(): this {
    this.ws.send(JSON.stringify({ type: 'subscribe', channel: this.name }));
    return this;
  }
  
  unsubscribe(): void {
    this.ws.send(JSON.stringify({ type: 'unsubscribe', channel: this.name }));
    this.ws.close();
  }
}
```

---

## Type Definitions

```typescript
// src/types/index.ts
export interface AeroDBResponse<T> {
  data: T | null;
  error: AeroDBError | null;
}

export interface AeroDBError {
  message: string;
  status?: number;
  code?: string;
}

export interface User {
  id: string;
  email: string;
  created_at: string;
}

export interface Session {
  access_token: string;
  refresh_token?: string;
  expires_at?: number;
}
```

---

## Usage Example

```typescript
import { AeroDBClient } from '@aerodb/client';

const client = new AeroDBClient({
  url: 'https://api.aerodb.com',
});

// Auth
const { data, error } = await client.auth.signIn('user@example.com', 'password');

// Query
const { data: users } = await client
  .from('users')
  .select('id, name, email')
  .eq('role', 'admin')
  .limit(10)
  .execute();

// Real-time
client.channel('users')
  .on('INSERT', (payload) => {
    console.log('New user:', payload.new);
  })
  .subscribe();

// Storage
const { data: file } = await client.storage
  .from('avatars')
  .upload('user-123.png', fileBlob);
```

---

## Build Configuration

```json
// package.json
{
  "name": "@aerodb/client",
  "version": "1.0.0",
  "type": "module",
  "main": "./dist/index.cjs",
  "module": "./dist/index.js",
  "types": "./dist/index.d.ts",
  "exports": {
    ".": {
      "import": "./dist/index.js",
      "require": "./dist/index.cjs",
      "types": "./dist/index.d.ts"
    }
  },
  "scripts": {
    "build": "tsup src/index.ts --format cjs,esm --dts",
    "test": "vitest",
    "lint": "eslint src",
    "typecheck": "tsc --noEmit"
  },
  "dependencies": {},
  "devDependencies": {
    "tsup": "^8.0.0",
    "typescript": "^5.0.0",
    "vitest": "^1.0.0"
  }
}
```

---

## Testing

```typescript
// tests/QueryBuilder.test.ts
import { describe, it, expect } from 'vitest';
import { QueryBuilder } from '../src/database/QueryBuilder';

describe('QueryBuilder', () => {
  it('builds simple query', () => {
    const qb = new QueryBuilder('users', 'https://api.example.com');
    qb.select('id, name').eq('role', 'admin').limit(10);
    
    const query = qb['buildQueryString'](); // Access private for testing
    expect(query).toContain('select=id,name');
    expect(query).toContain('role=eq.admin');
    expect(query).toContain('limit=10');
  });
});
```
