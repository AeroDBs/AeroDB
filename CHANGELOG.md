# Changelog

All notable changes to AeroDB will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-01-20

### Added

#### Backend
- **MVCC Transaction System** with snapshot isolation
- **Write-Ahead Logging (WAL)** for crash-safe durability
  - Deterministic recovery
  - Checksum validation
  - Fsync-based durability guarantees
- **Point-in-Time Snapshots** with tar-based archiving
- **Replication & Failover**
  - Leader-follower replication
  - Atomic promotion with durable markers
  - WAL streaming
- **Authentication & Authorization**
  - JWT-based session management
  - Argon2 password hashing
  - Password reset workflow
  - Configurable password policies
- **RESTful HTTP API** with comprehensive endpoints
  - Database operations (`/api/*`)
  - Auth management (`/auth/*`)
  - Storage operations (`/storage/*`)
  - Function invocation (`/functions/*`)
  - Real-time subscriptions (`/realtime/*`)
  - Backup/restore (`/backup/*`)
  - Cluster management (`/cluster/*`)
- **Real-time Subscriptions** via WebSocket
  - Collection-level subscriptions
  - Live data streaming
  - Presence tracking
- **File Storage Service**
  - S3-compatible API
  - Signed URL generation
  - Bucket management
  - Metadata storage
- **Serverless Functions**
  - WASM-based runtime (Wasmtime)
  - HTTP trigger support
  - Cron scheduling
  - Function versioning
- **Observability**
  - Query execution metrics
  - System health endpoints
  - Structured logging
  - Query explanation
- **Setup Wizard** (`/setup/*`)
  - First-run initialization
  - Storage configuration
  - Auth settings
  - Admin user creation

#### Frontend (Admin Dashboard)
- **Vue.js 3 + TypeScript** modern single-page application
- **Setup Wizard** (WordPress-style)
  - 6-step guided initialization
  - Configuration validation
  - One-time setup enforcement
- **Database Management**
  - Table browser with pagination
  - SQL console with syntax highlighting
  - Schema viewer
- **User & Auth Management**
  - User CRUD operations
  - Role and permission management
  - Password reset interface
- **Storage Browser**
  - File upload/download
  - Bucket management
  - Signed URL generation
- **Real-time Monitoring**
  - Live metrics dashboard
  - System health indicators
  - Active connections view
- **Functions Management**
  - Function editor
  - Deployment interface
  - Logs viewer
  - Scheduling configuration
- **Backup & Restore**
  - Snapshot creation
  - Point-in-time recovery
  - Backup history
- **Cluster Dashboard**
  - Replication status
  - Failover controls
  - Topology visualization

#### Documentation
- Comprehensive specification documents
  - 164 detailed spec files in `docs/`
  - Phase-based architecture documentation (Phases 0-16)
  - MVCC, WAL, Replication, Performance specs
  - Implementation guides
- Test suite with 26 integration tests
  - Crash safety tests
  - MVCC invariant tests
  - Replication authority tests
  - Storage integrity tests

### Security
- Argon2 password hashing with secure defaults
- JWT with configurable expiration
- CORS protection
- SQL injection prevention (prepared statements)
- XSS protection in dashboard
- Signed URLs for storage access

### Performance
- Async I/O with Tokio
- Connection pooling
- Optimized WAL batching
- Memory-mapped file support (foundation)

### Testing
- 26 integration tests
- Crash simulation framework
- MVCC invariant verification
- Frontend unit tests with Vitest
- E2E tests with Playwright

### Dependencies
- **Backend**: Rust 1.70+, Axum, Tokio, Serde, Argon2, JWT, Wasmtime
- **Frontend**: Vue 3, Pinia, Vue Router, Axios, Tailwind CSS 4, Vite

---

## Release Notes Format

### Added
New features

### Changed
Changes in existing functionality

### Deprecated
Soon-to-be removed features

### Removed
Removed features

### Fixed
Bug fixes

### Security
Vulnerability fixes

---

[Unreleased]: https://github.com/eshanized/AeroDB/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/eshanized/AeroDB/releases/tag/v0.1.0
