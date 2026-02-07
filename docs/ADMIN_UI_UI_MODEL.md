# Phase 13: Admin Dashboard - UI Components Model

## Design System

### shadcn/ui Foundation

Use **shadcn/ui** as the component library foundation:
- Radix UI primitives (accessible, unstyled)
- Tailwind CSS for styling
- Components copied into project (not npm dependency)
- Full customization control

### Theme

**Dark mode by default** with light mode toggle:

```typescript
// Tailwind config
module.exports = {
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        // Dark mode colors
        background: 'hsl(222.2 84% 4.9%)',
        foreground: 'hsl(210 40% 98%)',
        card: 'hsl(222.2 84% 4.9%)',
        'card-foreground': 'hsl(210 40% 98%)',
        primary: 'hsl(210 40% 98%)',
        secondary: 'hsl(217.2 32.6% 17.5%)',
        // ... etc
      },
    },
  },
};
```

---

## Core Components

### 1. DataTable

Reusable table with sorting, filtering, pagination:

```typescript
// components/common/DataTable.tsx
interface Column<T> {
  key: keyof T;
  header: string;
  cell?: (value: T[keyof T], row: T) => React.ReactNode;
  sortable?: boolean;
}

interface DataTableProps<T> {
  data: T[];
  columns: Column<T>[];
  loading?: boolean;
  onRowClick?: (row: T) => void;
  pagination?: {
    total: number;
    page: number;
    pageSize: number;
    onPageChange: (page: number) => void;
  };
}

export function DataTable<T>({ data, columns, loading, pagination }: DataTableProps<T>) {
  const [sortKey, setSortKey] = useState<keyof T | null>(null);
  const [sortDir, setSortDir] = useState<'asc' | 'desc'>('asc');
  
  const sortedData = useMemo(() => {
    if (!sortKey) return data;
    return [...data].sort((a, b) => {
      const aVal = a[sortKey];
      const bVal = b[sortKey];
      return sortDir === 'asc' 
        ? String(aVal).localeCompare(String(bVal))
        : String(bVal).localeCompare(String(aVal));
    });
  }, [data, sortKey, sortDir]);
  
  return (
    <Table>
      <TableHeader>
        <TableRow>
          {columns.map((col) => (
            <TableHead key={String(col.key)}>
              {col.sortable ? (
                <button onClick={() => {
                  setSortKey(col.key);
                  setSortDir(sortDir === 'asc' ? 'desc' : 'asc');
                }}>
                  {col.header} {sortKey === col.key && (sortDir === 'asc' ? '↑' : '↓')}
                </button>
              ) : (
                col.header
              )}
            </TableHead>
          ))}
        </TableRow>
      </TableHeader>
      <TableBody>
        {loading ? (
          <TableRow>
            <TableCell colSpan={columns.length}>Loading...</TableCell>
          </TableRow>
        ) : (
          sortedData.map((row, i) => (
            <TableRow key={i}>
              {columns.map((col) => (
                <TableCell key={String(col.key)}>
                  {col.cell ? col.cell(row[col.key], row) : String(row[col.key])}
                </TableCell>
              ))}
            </TableRow>
          ))
        )}
      </TableBody>
    </Table>
  );
}
```

### 2. FilterBuilder

Visual filter creator:

```typescript
// components/common/FilterBuilder.tsx
interface Filter {
  field: string;
  operator: 'eq' | 'gt' | 'lt' | 'like' | 'in';
  value: string | number;
}

export function FilterBuilder({ onChange }: { onChange: (filters: Filter[]) => void }) {
  const [filters, setFilters] = useState<Filter[]>([]);
  
  const addFilter = () => {
    setFilters([...filters, { field: '', operator: 'eq', value: '' }]);
  };
  
  const updateFilter = (index: number, updates: Partial<Filter>) => {
    const updated = [...filters];
    updated[index] = { ...updated[index], ...updates };
    setFilters(updated);
    onChange(updated);
  };
  
  return (
    <div className="space-y-2">
      {filters.map((filter, i) => (
        <div key={i} className="flex gap-2">
          <Input
            placeholder="Field"
            value={filter.field}
            onChange={(e) => updateFilter(i, { field: e.target.value })}
          />
          <Select
            value={filter.operator}
            onValueChange={(op) => updateFilter(i, { operator: op as Filter['operator'] })}
          >
            <SelectTrigger><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem value="eq">=</SelectItem>
              <SelectItem value="gt">&gt;</SelectItem>
              <SelectItem value="lt">&lt;</SelectItem>
              <SelectItem value="like">LIKE</SelectItem>
            </SelectContent>
          </Select>
          <Input
            placeholder="Value"
            value={filter.value}
            onChange={(e) => updateFilter(i, { value: e.target.value })}
          />
          <Button variant="ghost" size="icon" onClick={() => {
            const updated = filters.filter((_, idx) => idx !== i);
            setFilters(updated);
            onChange(updated);
          }}>
            <X className="h-4 w-4" />
          </Button>
        </div>
      ))}
      <Button onClick={addFilter} variant="outline">
        <Plus className="h-4 w-4 mr-2" /> Add Filter
      </Button>
    </div>
  );
}
```

### 3. SchemaVisualizer

Interactive schema diagram:

```typescript
// features/database/components/SchemaVisualizer.tsx
import { useState } from 'react';
import { Handle, Position, ReactFlow } from 'reactflow';

interface Table {
  name: string;
  fields: { name: string; type: string; nullable: boolean }[];
  relations: { field: string; references: string }[];
}

export function SchemaVisualizer({ schema }: { schema: Table[] }) {
  const nodes = schema.map((table, i) => ({
    id: table.name,
    type: 'tableNode',
    data: { table },
    position: { x: i * 300, y: 100 },
  }));
  
  const edges = schema.flatMap((table) =>
    table.relations.map((rel) => ({
      id: `${table.name}-${rel.field}`,
      source: table.name,
      target: rel.references,
      label: rel.field,
    }))
  );
  
  return (
    <div className="h-full w-full">
      <ReactFlow nodes={nodes} edges={edges} nodeTypes={{ tableNode: TableNode }} />
    </div>
  );
}

function TableNode({ data }: { data: { table: Table } }) {
  const { table } = data;
  return (
    <div className="bg-card border rounded-lg p-4 min-w-[200px]">
      <div className="font-bold mb-2">{table.name}</div>
      {table.fields.map((field) => (
        <div key={field.name} className="text-sm">
          {field.name}: {field.type}
          {field.nullable && ' (nullable)'}
        </div>
      ))}
      <Handle type="source" position={Position.Right} />
      <Handle type="target" position={Position.Left} />
    </div>
  );
}
```

### 4. MetricsChart

Time-series visualization:

```typescript
// features/observability/components/MetricsChart.tsx
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip } from 'recharts';

interface DataPoint {
  timestamp: string;
  value: number;
}

export function MetricsChart({ data, title }: { data: DataPoint[]; title: string }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
      </CardHeader>
      <CardContent>
        <LineChart width={600} height={300} data={data}>
          <CartesianGrid strokeDasharray="3 3" />
          <XAxis dataKey="timestamp" />
          <YAxis />
          <Tooltip />
          <Line type="monotone" dataKey="value" stroke="hsl(210 40% 98%)" />
        </LineChart>
      </CardContent>
    </Card>
  );
}
```

### 5. ConfirmDialog

Confirmation for destructive actions:

```typescript
// components/common/ConfirmDialog.tsx
export function ConfirmDialog({
  open,
  onOpenChange,
  title,
  description,
  action,
  onConfirm,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description: string;
  action: string;
  onConfirm: () => void;
}) {
  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{title}</AlertDialogTitle>
          <AlertDialogDescription>{description}</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction onClick={onConfirm}>{action}</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

// Usage
function DeleteUserButton({ userId }: { userId: string }) {
  const [open, setOpen] = useState(false);
  const deleteMutation = useMutation({ /* ... */ });
  
  return (
    <>
      <Button variant="destructive" onClick={() => setOpen(true)}>Delete</Button>
      <ConfirmDialog
        open={open}
        onOpenChange={setOpen}
        title="Delete User"
        description="This action cannot be undone. The user will lose all data."
        action="Delete"
        onConfirm={() => {
          deleteMutation.mutate(userId);
          setOpen(false);
        }}
      />
    </>
  );
}
```

---

## Layout Components

### AppLayout

```typescript
// components/layout/AppLayout.tsx
export function AppLayout({ children }: { children: React.ReactNode }) {
  const { sidebarOpen } = useUIStore();
  
  return (
    <div className="flex h-screen">
      <Sidebar open={sidebarOpen} />
      <div className="flex-1 flex flex-col">
        <Header />
        <main className="flex-1 overflow-auto p-6">
          {children}
        </main>
      </div>
    </div>
  );
}
```

### Sidebar

```typescript
// components/layout/Sidebar.tsx
export function Sidebar({ open }: { open: boolean }) {
  return (
    <aside className={cn('bg-card border-r transition-all', open ? 'w-64' : 'w-16')}>
      <nav className="p-4 space-y-2">
        <NavLink to="/database" icon={Database}>Database</NavLink>
        <NavLink to="/auth" icon={Users}>Auth</NavLink>
        <NavLink to="/storage" icon={HardDrive}>Storage</NavLink>
        <NavLink to="/realtime" icon={Activity}>Real-Time</NavLink>
        <NavLink to="/cluster" icon={Server}>Cluster</NavLink>
        <NavLink to="/logs" icon={FileText}>Logs</NavLink>
        <NavLink to="/metrics" icon={BarChart}>Metrics</NavLink>
      </nav>
    </aside>
  );
}

function NavLink({ to, icon: Icon, children }: { to: string; icon: any; children: React.ReactNode }) {
  return (
    <Link to={to} className="flex items-center gap-2 p-2 rounded hover:bg-secondary">
      <Icon className="h-5 w-5" />
      <span>{children}</span>
    </Link>
  );
}
```

---

## Component Composition Patterns

### List-Detail Pattern

```typescript
// features/database/pages/TablePage.tsx
export function TablePage() {
  const { tables } = useTables();
  const [selected, setSelected] = useState<string | null>(null);
  
  return (
    <div className="grid grid-cols-[300px_1fr] gap-4 h-full">
      {/* List */}
      <div className="border rounded-lg p-4 overflow-auto">
        {tables.map((table) => (
          <button
            key={table.name}
            onClick={() => setSelected(table.name)}
            className={cn('w-full text-left p-2 rounded', selected === table.name && 'bg-secondary')}
          >
            {table.name}
          </button>
        ))}
      </div>
      
      {/* Detail */}
      <div className="border rounded-lg p-4">
        {selected ? <TableBrowser collection={selected} /> : <EmptyState />}
      </div>
    </div>
  );
}
```

---

## Accessibility

All components meet **WCAG 2.1 AA** standards:
- Keyboard navigation (Tab, Enter, Escape)
- Screen reader support (ARIA labels)
- Focus indicators
- Color contrast ratio ≥ 4.5:1

---

## Responsive Design

Mobile-first approach:
- Sidebar collapses on mobile (< 768px)
- Tables scroll horizontally
- Charts resize to container width
- Touch-friendly tap targets (min 44x44px)

---

## Performance

- **Code splitting**: Lazy-load routes
- **Virtualization**: Large tables use `react-window`
- **Memoization**: Use `React.memo`, `useMemo`, `useCallback`
- **Image optimization**: WebP format, lazy loading
