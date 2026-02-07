# Phase 13: Admin Dashboard - Architecture

## System Overview

The Admin Dashboard is a **React-based SPA** (Single Page Application) that communicates exclusively with AeroDB's public APIs. It has no direct database access, no special privileges, and can be deployed independently of the database server.

```
┌─────────────────────────────────────────────────────────────┐
│                     Browser (User)                          │
│  ┌───────────────────────────────────────────────────────┐  │
│  │         Admin Dashboard (React SPA)                   │  │
│  │  ┌─────────────┐  ┌──────────────┐  ┌─────────────┐  │  │
│  │  │ Table       │  │ Schema       │  │ User        │  │  │
│  │  │ Browser     │  │ Editor       │  │ Manager     │  │  │
│  │  └─────────────┘  └──────────────┘  └─────────────┘  │  │
│  └───────────────────────────────────────────────────────┘  │
│         ↓ HTTP/WS                                           │
└─────────┼───────────────────────────────────────────────────┘
          │
┌─────────┼───────────────────────────────────────────────────┐
│ AeroDB  ↓ Public APIs Only                                  │
│  ┌──────────────┐  ┌───────────────┐  ┌─────────────────┐  │
│  │  REST API    │  │  Auth API     │  │  Control Plane  │  │
│  │  /rest/v1    │  │  /auth        │  │  /control       │  │
│  └──────────────┘  └───────────────┘  └─────────────────┘  │
│         ↓                  ↓                      ↓          │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              Query Executor (Core)                   │  │
│  └──────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────┘
```

---

## Technology Stack

### Frontend

- **Framework**: React 18+ (with TypeScript)
- **State Management**: React Query (for server state) + Zustand (for UI state)
- **UI Library**: shadcn/ui (Radix UI primitives + Tailwind CSS)
- **Routing**: React Router v6
- **Charts**: Recharts (for metrics dashboards)
- **Code Editor**: Monaco Editor (for SQL console)
- **HTTP Client**: Axios (with interceptors for auth)
- **WebSocket**: Native WebSocket API (for real-time updates)

### Build Tooling

- **Bundler**: Vite (fast dev server, HMR)
- **TypeScript**: Strict mode enabled
- **Linter**: ESLint + Prettier
- **Testing**: Vitest (unit) + Playwright (E2E)

---

## Application Structure

```
dashboard/
├── src/
│   ├── components/          # Shared UI components
│   │   ├── ui/              # shadcn/ui components
│   │   ├── layout/          # Header, Sidebar, Layout
│   │   └── common/          # Buttons, Modals, Tables
│   │
│   ├── features/            # Feature-specific modules
│   │   ├── database/        # Table browser, SQL console
│   │   ├── auth/            # User management, sessions
│   │   ├── storage/         # File browser, uploads
│   │   ├── realtime/        # Subscriptions, events
│   │   ├── cluster/         # Topology, replication
│   │   └── observability/   # Logs, metrics, audit
│   │
│   ├── lib/                 # Shared utilities
│   │   ├── api/             # API client wrappers
│   │   ├── auth/            # Auth context, hooks
│   │   ├── hooks/           # Custom React hooks
│   │   └── utils/           # Helper functions
│   │
│   ├── types/               # TypeScript type definitions
│   ├── config/              # App configuration
│   ├── App.tsx              # Root component
│   └── main.tsx             # Entry point
│
├── public/                  # Static assets
├── tests/                   # E2E tests
└── package.json
```

---

## Core Components

### 1. API Client Layer

All HTTP requests go through a centralized client:

```typescript
// lib/api/client.ts
import axios from 'axios';

export const apiClient = axios.create({
  baseURL: import.meta.env.VITE_AERODB_URL,
  timeout: 30000,
});

// Request interceptor: Attach JWT
apiClient.interceptors.request.use((config) => {
  const token = localStorage.getItem('access_token');
  if (token) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

// Response interceptor: Handle 401, refresh tokens
apiClient.interceptors.response.use(
  (response) => response,
  async (error) => {
    if (error.response?.status === 401) {
      // Attempt token refresh
      const refreshed = await refreshAccessToken();
      if (refreshed) {
        // Retry original request
        return apiClient.request(error.config);
      } else {
        // Redirect to login
        window.location.href = '/login';
      }
    }
    return Promise.reject(error);
  }
);
```

### 2. Data Fetching with React Query

Use React Query for server state management:

```typescript
// features/database/hooks/useCollections.ts
import { useQuery } from '@tanstack/react-query';
import { apiClient } from '@/lib/api/client';

export function useCollections() {
  return useQuery({
    queryKey: ['collections'],
    queryFn: async () => {
      const { data } = await apiClient.get('/rest/v1/_schema/collections');
      return data;
    },
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}
```

### 3. Table Browser

Paginated table view with filters:

```typescript
// features/database/components/TableBrowser.tsx
import { useTableData } from '../hooks/useTableData';
import { DataTable } from '@/components/common/DataTable';

export function TableBrowser({ collection }: { collection: string }) {
  const [page, setPage] = useState(0);
  const [filters, setFilters] = useState({});
  
  const { data, isLoading } = useTableData(collection, {
    limit: 20,
    offset: page * 20,
    filters,
  });
  
  return (
    <div>
      <FilterBar filters={filters} onChange={setFilters} />
      <DataTable
        data={data?.rows || []}
        columns={data?.columns || []}
        loading={isLoading}
      />
      <Pagination
        total={data?.total || 0}
        page={page}
        onChange={setPage}
      />
    </div>
  );
}
```

### 4. SQL Console

Execute queries with Monaco Editor:

```typescript
// features/database/components/SQLConsole.tsx
import MonacoEditor from '@monaco-editor/react';
import { useMutation } from '@tanstack/react-query';

export function SQLConsole() {
  const [sql, setSql] = useState('');
  const [results, setResults] = useState(null);
  
  const executeMutation = useMutation({
    mutationFn: async (query: string) => {
      const { data } = await apiClient.post('/rest/v1/_query', { query });
      return data;
    },
    onSuccess: (data) => setResults(data),
  });
  
  return (
    <div className="grid grid-rows-2 h-full">
      <MonacoEditor
        language="sql"
        value={sql}
        onChange={(value) => setSql(value || '')}
      />
      <Button onClick={() => executeMutation.mutate(sql)}>
        Execute
      </Button>
      {results && <ResultsTable data={results} />}
    </div>
  );
}
```

### 5. Real-Time Updates

Connect to WebSocket for live data:

```typescript
// features/realtime/hooks/useRealtimeSubscription.ts
import { useEffect, useState } from 'react';

export function useRealtimeSubscription(channel: string) {
  const [events, setEvents] = useState([]);
  
  useEffect(() => {
    const ws = new WebSocket('wss://aerodb.example.com/realtime/v1');
    
    ws.onopen = () => {
      ws.send(JSON.stringify({ type: 'subscribe', channel }));
    };
    
    ws.onmessage = (event) => {
      const message = JSON.parse(event.data);
      if (message.type === 'event') {
        setEvents((prev) => [...prev, message.payload]);
      }
    };
    
    return () => ws.close();
  }, [channel]);
  
  return { events };
}
```

---

## Routing Structure

```
/                       # Dashboard home
/login                  # Login page
/database               # Database section
  /database/tables      # Table list
  /database/table/:name # Table browser
  /database/sql         # SQL console
  /database/schema      # Schema editor
/auth                   # Authentication section
  /auth/users           # User list
  /auth/sessions        # Active sessions
  /auth/policies        # RLS policies
/storage                # File storage
  /storage/buckets      # Bucket list
  /storage/bucket/:name # File browser
/realtime               # Real-time monitoring
  /realtime/subscriptions # Active subscriptions
  /realtime/events      # Event log
/cluster                # Cluster management
  /cluster/topology     # Topology view
  /cluster/replication  # Replication status
/logs                   # Observability
  /logs/system          # System logs
  /logs/audit           # Audit log
/metrics                # Metrics dashboard
```

---

## State Management Strategy

### Server State (React Query)

All API data is managed by React Query:
- Automatic caching
- Background refetching
- Optimistic updates
- Request deduplication

### UI State (Zustand)

Local UI state uses Zustand:
- Sidebar collapsed/expanded
- Current theme (light/dark)
- User preferences
- Active filters

```typescript
// lib/store/uiStore.ts
import create from 'zustand';

interface UIState {
  sidebarOpen: boolean;
  theme: 'light' | 'dark';
  toggleSidebar: () => void;
  setTheme: (theme: 'light' | 'dark') => void;
}

export const useUIStore = create<UIState>((set) => ({
  sidebarOpen: true,
  theme: 'dark',
  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
  setTheme: (theme) => set({ theme }),
}));
```

---

## Security Considerations

### 1. Token Storage

- **Access Token**: Store in memory (React state, not localStorage)
- **Refresh Token**: Store in httpOnly cookie (if server supports)
- **Fallback**: If no httpOnly cookies, use localStorage with XSS precautions

### 2. XSS Protection

- Sanitize all user input displayed in UI (use DOMPurify)
- Use React's JSX auto-escaping
- Set Content-Security-Policy headers

### 3. CSRF Protection

- All mutations use POST/PUT/DELETE (not GET)
- Require explicit confirmation for destructive actions

---

## Deployment

The dashboard is **statically deployable**:

1. Build: `npm run build` → outputs to `dist/`
2. Deploy `dist/` to:
   - **Vercel/Netlify**: Auto-deploy from Git
   - **S3 + CloudFront**: Static hosting
   - **Local**: Serve with `npx serve dist`

Environment variables:
```bash
VITE_AERODB_URL=https://api.aerodb.example.com
VITE_WS_URL=wss://api.aerodb.example.com/realtime/v1
```

---

## Non-Goals

The dashboard **does not**:
- Store any application state server-side
- Have its own database or cache layer
- Require a backend-for-frontend (BFF)
- Provide offline-first capabilities (read-only cache is acceptable)

All logic is client-side, all data is fetched from AeroDB APIs.
