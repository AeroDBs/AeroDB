# Phase 16: Developer Tools - Readiness Criteria

## Freeze Checklist

Phase 16 is ready to freeze when **all items below** are complete:

---

## 1. CLI (`aerodb`)

- [ ] **Project Management**
  - [ ] `aerodb init <name>` (create project)
  - [ ] `aerodb start` (start local server)
  - [ ] `aerodb stop` (stop local server)
  - [ ] `aerodb status` (check if running)

- [ ] **Migrations**
  - [ ] `aerodb migrations new <name>` (create migration)
  - [ ] `aerodb migrations up` (apply all pending)
  - [ ] `aerodb migrations down` (rollback last)
  - [ ] `aerodb migrations status` (show applied/pending)
  - [ ] `aerodb migrations reset` (rollback all + reapply)

- [ ] **Code Generation**
  - [ ] `aerodb types --lang typescript` (generate TS types)
  - [ ] `aerodb types --lang python` (generate Pydantic models)
  - [ ] `aerodb types --output <file>` (write to file)

- [ ] **Deployment**
  - [ ] `aerodb deploy --env <name>` (apply migrations to remote)
  - [ ] `aerodb deploy --dry-run` (show pending migrations)

- [ ] **Utilities**
  - [ ] `aerodb seed <file>` (run seed SQL)
  - [ ] `aerodb dump` (export data)
  - [ ] `aerodb diff --source local --target prod` (schema diff)

---

## 2. Migration System

- [ ] **File Format**
  - [ ] `.up.sql` and `.down.sql` files
  - [ ] Timestamp prefix (YYYYMMDDHHMMSS_name.up.sql)
  - [ ] Stored in `migrations/` directory

- [ ] **Tracking**
  - [ ] `_migrations` table (version, name, applied_at)
  - [ ] Idempotent up/down (can rerun safely)
  - [ ] Transactional (all or nothing)

- [ ] **Features**
  - [ ] Dependency checking (migrations applied in order)
  - [ ] Rollback support (down.sql must undo up.sql)
  - [ ] Dry-run mode (show SQL without executing)

---

## 3. Code Generation

- [ ] **TypeScript**
  - [ ] Interface per table
  - [ ] Nullable fields use `Type | null`
  - [ ] UUID/TIMESTAMPTZ mapped correctly
  - [ ] `Database` type (union of all tables)

- [ ] **Python**
  - [ ] Pydantic BaseModel per table
  - [ ] Optional fields use `Optional[Type]`
  - [ ] datetime, UUID types imported

- [ ] **Schema Introspection**
  - [ ] Read from `_schema` REST endpoint
  - [ ] Parse columns, types, nullability
  - [ ] Handle foreign keys (references)

---

## 4. Configuration

- [ ] **aerodb.toml**
  - [ ] `[project]` section (name, schema)
  - [ ] `[database]` section (url)
  - [ ] `[database.<env>]` sections (production, staging)
  - [ ] `[migrations]` section (directory)
  - [ ] `[codegen]` section (output, lang)

- [ ] **Environment Variables**
  - [ ] `AERODB_URL` override
  - [ ] `AERODB_KEY` for auth
  - [ ] `.env` file support

---

## 5. VS Code Extension

- [ ] **Features**
  - [ ] SQL syntax highlighting
  - [ ] Schema explorer (sidebar)
  - [ ] Run SQL inline (Cmd+Enter)
  - [ ] Autocomplete for table/column names
  - [ ] Migration snippets

- [ ] **Installation**
  - [ ] Published to VS Code Marketplace
  - [ ] Search "AeroDB" in extensions

---

## 6. Testing

- [ ] **CLI Tests**
  - [ ] `aerodb init` creates correct files
  - [ ] `aerodb migrations up` applies migrations
  - [ ] `aerodb types` generates valid TypeScript
  - [ ] `aerodb deploy` fails gracefully on error

- [ ] **Code Gen Tests**
  - [ ] Generated TS types compile without errors
  - [ ] Generated Python models validate with mypy

- [ ] **Integration Tests**
  - [ ] Full workflow: init → migrate → codegen → deploy

---

## 7. Documentation

- [ ] **CLI Reference**
  - [ ] All commands documented
  - [ ] Examples for each command
  - [ ] Flag descriptions

- [ ] **Migration Guide**
  - [ ] How to create first migration
  - [ ] Best practices (naming, rollbacks)
  - [ ] Common patterns (add column, drop table)

- [ ] **Code Gen Guide**
  - [ ] Using generated types in app
  - [ ] Customizing output (exclude tables)

- [ ] **aerodb.toml Reference**
  - [ ] All sections explained
  - [ ] Example configurations

---

## 8. Platform Support

- [ ] **Operating Systems**
  - [ ] macOS (Intel + Apple Silicon)
  - [ ] Linux (x86_64, aarch64)
  - [ ] Windows (x86_64)

- [ ] **Installation**
  - [ ] Homebrew (macOS/Linux): `brew install aerodb`
  - [ ] Cargo (Rust): `cargo install aerodb-cli`
  - [ ] npm (global): `npm install -g @aerodb/cli` (wrapper)
  - [ ] Binary release (GitHub Releases)

---

## 9. CI/CD Integration

- [ ] **GitHub Actions**
  - [ ] `aerodb/setup-cli` action
  - [ ] Example workflow (migrate on push)

- [ ] **GitLab CI**
  - [ ] Docker image with `aerodb` CLI
  - [ ] Example `.gitlab-ci.yml`

- [ ] **Vercel/Netlify**
  - [ ] Run migrations via webhook
  - [ ] Example integrations

---

## 10. Performance

- [ ] **Startup Time**
  - [ ] `aerodb start` < 2 seconds
  - [ ] `aerodb types` < 1 second

- [ ] **Binary Size**
  - [ ] < 20MB (release build)

---

## Sign-Off

Phase 16 is **frozen** when:

1. All checklist items complete
2. CLI published to Homebrew, Cargo, npm
3. VS Code extension published
4. Full workflow tested (init → migrate → deploy)
5. Documentation complete

**Frozen on**: [DATE]

**Approved by**: [NAME]
