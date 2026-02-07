# Phase 16: Developer Tools - Vision

## Purpose

Provide **CLI, migration tools, and code generators** to streamline AeroDB development workflows.

## Philosophy

### Local-First Development

Developers should be able to:
- Run AeroDB **locally** (single binary, no Docker required)
- Develop **offline** (local DB, no network)
- Deploy **easily** (single command)

### Git-Like Workflow

Database changes are treated like **code**:
- Migrations are commits
- Branches for schema experimentation
- Merge conflicts are schema conflicts

---

## Core Tools

### 1. CLI (`aerodb`)

Command-line interface for all operations:

```bash
# Initialize project
aerodb init my-project

# Start local database
aerodb start

# Create migration
aerodb migrations new add_users_table

# Apply migrations
aerodb migrations up

# Generate TypeScript types
aerodb types > src/database.types.ts

# Deploy to production
aerodb deploy --env production
```

### 2. Migration System

SQL-based migrations with version control:

```bash
# Create migration
aerodb migrations new add_posts_table
# Creates: migrations/20240815120000_add_posts_table.up.sql
#         migrations/20240815120000_add_posts_table.down.sql

# Apply all pending
aerodb migrations up

# Rollback last
aerodb migrations down

# Show status
aerodb migrations status
```

Example migration:
```sql
-- migrations/20240815120000_add_posts_table.up.sql
CREATE TABLE posts (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  title TEXT NOT NULL,
  content TEXT,
  author_id UUID REFERENCES users(id),
  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_posts_author ON posts(author_id);
```

### 3. Code Generation

Generate types from database schema:

```bash
# TypeScript types
aerodb types --lang typescript > types.ts

# Python Pydantic models
aerodb types --lang python > models.py
```

Output (TypeScript):
```typescript
export interface User {
  id: string;
  email: string;
  created_at: string;
}

export interface Post {
  id: string;
  title: string;
  content: string | null;
  author_id: string;
  created_at: string;
}

export interface Database {
  users: User;
  posts: Post;
}
```

### 4. VS Code Extension

Features:
- **SQL syntax highlighting** with AeroDB-specific extensions
- **Schema explorer** (sidebar panel)
- **Query execution** (Run SQL inline)
- **Autocomplete** for table/column names
- **Migration snippets**

### 5. Schema Diff Tool

Compare local and remote schemas:

```bash
aerodb diff --source local --target production
```

Output:
```diff
+ CREATE TABLE posts (...)
- DROP TABLE old_logs
~ ALTER TABLE users ADD COLUMN last_login TIMESTAMPTZ
```

### 6. Seed Data Management

Populate dev/test databases:

```bash
# Run seed file
aerodb seed data/seed.sql

# Export current data
aerodb dump --data-only > data/seed.sql
```

Example seed:
```sql
-- data/seed.sql
INSERT INTO users (email, name) VALUES
  ('alice@example.com', 'Alice'),
  ('bob@example.com', 'Bob');

INSERT INTO posts (title, author_id) VALUES
  ('First Post', (SELECT id FROM users WHERE email = 'alice@example.com'));
```

---

## CLI Architecture

```
aerodb (Rust binary)
├── commands/
│   ├── init.rs          # Project initialization
│   ├── start.rs         # Start local server
│   ├── migrations.rs    # Migration commands
│   ├── types.rs         # Code generation
│   ├── deploy.rs        # Deployment
│   └── seed.rs          # Data seeding
├── lib/
│   ├── config.rs        # Parse aerodb.toml
│   ├── client.rs        # HTTP client for API
│   └── schema.rs        # Schema introspection
└── main.rs
```

### Configuration File

```toml
# aerodb.toml
[project]
name = "my-app"
schema = "public"

[database]
url = "http://localhost:54321"

[database.production]
url = "https://my-app.aerodb.com"
key = "${AERODB_KEY}"

[migrations]
directory = "migrations"

[codegen]
output = "src/types/database.types.ts"
lang = "typescript"
```

---

## Developer Workflow

### Step 1: Initialize Project

```bash
aerodb init my-app
cd my-app
```

Creates:
```
my-app/
├── aerodb.toml
├── migrations/
└── .gitignore
```

### Step 2: Start Local Database

```bash
aerodb start
# Runs local AeroDB on http://localhost:54321
```

### Step 3: Create Schema

```bash
aerodb migrations new create_users
# Edit migrations/...create_users.up.sql
aerodb migrations up
```

### Step 4: Generate Types

```bash
aerodb types > src/database.types.ts
```

### Step 5: Develop Application

Use generated types in application code:

```typescript
import { AeroDBClient } from '@aerodb/client';
import type { Database } from './database.types';

const client = new AeroDBClient<Database>({ url: 'http://localhost:54321' });

const { data } = await client.from('users').select('*');
// `data` is typed as User[]
```

### Step 6: Deploy

```bash
aerodb deploy --env production
# Applies pending migrations to production
```

---

## Integration with Other Tools

### GitHub Actions

```yaml
name: Deploy
on: push

jobs:
  migrate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: aerodb/setup-cli@v1
      - run: aerodb migrations up --env production
        env:
          AERODB_KEY: ${{ secrets.AERODB_KEY }}
```

### Docker

```dockerfile
FROM aerodb/aerodb:latest

COPY migrations /migrations
RUN aerodb migrations up
```

---

## Success Criteria

Developer tools are successful if:

1. **Setup time < 5 minutes**: `aerodb init` → local DB running
2. **Type safety**: Zero runtime errors from schema mismatches
3. **Fast migrations**: < 10s to apply migrations to empty DB

---

## Non-Goals

- **GUI migration tool**: Migrations are code (SQL files)
- **ORM features**: CLI focuses on schema, not query building
- **Database browser**: Use Admin Dashboard (Phase 13)

---

## Prior Art

Inspired by:
- **Prisma**: `prisma migrate`, `prisma generate`
- **Supabase CLI**: `supabase init`, `supabase start`
- **Rails migrations**: `rails db:migrate`, `db:rollback`
- **Flyway**: SQL-based migrations with version control

Differentiator: **Integrated with AeroDB control plane** - deployment, types, migrations all use same config.
