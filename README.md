# AeroDB

<div align="center">

**A strict, deterministic, self-hostable database built for production**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Vue.js](https://img.shields.io/badge/vuejs-%2335495e.svg?style=for-the-badge&logo=vuedotjs&logoColor=%234FC08D)](https://vuejs.org/)

[Documentation](#documentation) • [Features](#features) • [Quick Start](#quick-start) • [Architecture](#architecture) • [Contributing](#contributing)

</div>

---

## Overview

AeroDB is a production-grade database system designed to outperform MongoDB in correctness, predictability, and operational clarity, while meeting the reliability expectations associated with platforms like PostgreSQL.

**Core Philosophy:** Trust over flexibility • Predictability over cleverness • Correctness over convenience

### Key Principles

- **Deterministic by Design**: Same query + schema + data = same execution plan, always
- **Schema-First**: All data requires explicit, versioned schemas
- **Fail-Fast**: Unsafe operations are rejected before execution
- **WAL-Backed Durability**: No acknowledged write is ever lost
- **Self-Hostable**: First-class support for on-prem deployment

---

## Features

### Backend Capabilities

- ✅ **MVCC Transaction System** - Multi-version concurrency control with snapshot isolation
- ✅ **Write-Ahead Logging** - Crash-safe durability with deterministic recovery
- ✅ **Point-in-Time Snapshots** - Consistent backup and restore
- ✅ **Replication & Failover** - Leader-follower replication with atomic promotion
- ✅ **Authentication & Authorization** - JWT-based auth with password policies
- ✅ **RESTful API** - Comprehensive HTTP API for all operations
- ✅ **Real-time Subscriptions** - WebSocket-based live data streaming
- ✅ **File Storage** - S3-compatible object storage with signed URLs
- ✅ **Serverless Functions** - WASM-based function runtime
- ✅ **Observability** - Metrics, logging, and query explanation

### Admin Dashboard

- 🎨 **Modern Vue.js UI** - Beautiful, responsive interface built with Tailwind CSS
- 🔐 **First-Run Setup Wizard** - WordPress-style initialization flow
- 📊 **Database Management** - Table browser, SQL console, schema editor
- 👥 **User Management** - Create users, roles, and permissions
- 📁 **Storage Browser** - File upload, download, and bucket management
- ⚡ **Real-time Monitoring** - Live metrics and system health
- 🗂️ **Backup & Restore** - Point-in-time recovery interface
- 🌐 **Cluster Management** - Replication status and failover controls

---

## Quick Start

### Prerequisites

- **Rust** 1.70+ ([Install Rust](https://www.rust-lang.org/tools/install))
- **Node.js** 18+ ([Install Node.js](https://nodejs.org/))
- **npm** or **yarn**

### Installation

```bash
# Clone the repository
git clone https://github.com/eshanized/AeroDB.git
cd AeroDB

# Build the backend
cargo build --release

# Install frontend dependencies
cd dashboard
npm install
cd ..
```

### Running AeroDB

#### 1. Start the Backend Server

```bash
cargo run --release -- serve
```

The server will start on `http://localhost:54321`

#### 2. Start the Dashboard (Development)

```bash
cd dashboard
npm run dev
```

Access the dashboard at `http://localhost:5173`

#### 3. Complete the Setup Wizard

On first visit, you'll be guided through:
1. **Storage Configuration** - Set data, WAL, and snapshot directories
2. **Authentication Settings** - Configure JWT expiry and password policies
3. **Admin User Creation** - Create your first super-admin account
4. **Review & Confirm** - Verify configuration before finalizing

---

## Architecture

### Backend Stack

```
┌─────────────────────────────────────────┐
│         HTTP Server (Axum)              │
│  /api  /auth  /storage  /functions      │
│  /realtime  /backup  /cluster  /setup   │
└─────────────────────────────────────────┘
                    │
    ┌───────────────┼───────────────┐
    │               │               │
┌───▼────┐    ┌────▼─────┐   ┌────▼──────┐
│  MVCC  │    │   WAL    │   │  Storage  │
│ Engine │    │  Logger  │   │  Backend  │
└────────┘    └──────────┘   └───────────┘
```

- **Language**: Rust (async with Tokio)
- **HTTP Framework**: Axum
- **Serialization**: Serde JSON
- **Authentication**: JWT with Argon2 password hashing
- **WebSocket**: tokio-tungstenite
- **Functions Runtime**: Wasmtime

### Frontend Stack

- **Framework**: Vue.js 3 (Composition API)
- **State Management**: Pinia
- **Routing**: Vue Router
- **Styling**: Tailwind CSS 4
- **HTTP Client**: Axios
- **Build Tool**: Vite

### File Structure

```
AeroDB/
├── src/              # Backend Rust source
│   ├── core/         # MVCC, storage, WAL
│   ├── auth/         # Authentication & JWT
│   ├── http_server/  # API routes
│   ├── realtime/     # WebSocket subscriptions
│   ├── functions/    # WASM runtime
│   └── cli/          # Command-line interface
├── dashboard/        # Frontend Vue.js app
│   ├── src/
│   │   ├── setup/    # Setup wizard
│   │   ├── pages/    # Dashboard pages
│   │   ├── stores/   # Pinia stores
│   │   └── components/
│   └── package.json
├── docs/             # Specification documents
├── tests/            # Integration tests
└── Cargo.toml
```

---

## API Reference

### Core Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/query` | POST | Execute database query |
| `/api/collections` | GET | List collections |
| `/auth/login` | POST | Authenticate user |
| `/auth/register` | POST | Create new user |
| `/storage/upload` | POST | Upload file |
| `/functions/invoke/:id` | POST | Execute function |
| `/realtime/subscribe` | WS | Subscribe to changes |
| `/backup/create` | POST | Create snapshot |
| `/cluster/status` | GET | Cluster health |

Full API documentation: [API Spec](docs/CORE_API_SPEC.md)

---

## Testing

### Backend Tests

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test recovery_  # Recovery tests
cargo test mvcc_     # MVCC tests
cargo test storage_  # Storage tests
```

### Frontend Tests

```bash
cd dashboard

# Unit tests
npm run test

# E2E tests
npm run test:e2e

# Coverage
npm run test:coverage
```

---

## Documentation

Comprehensive documentation is available in the [`docs/`](docs/) directory:

### Core Specifications
- [Vision & Philosophy](docs/CORE_VISION.md)
- [Storage Model](docs/CORE_STORAGE.md)
- [WAL Specification](docs/CORE_WAL.md)
- [MVCC Model](docs/MVCC_MODEL.md)
- [Replication Architecture](docs/REPL_MODEL.md)

### Phase Documentation
- [Phase 8: Authentication](docs/PHASE8_AUTH_ARCHITECTURE.md)
- [Phase 10: Real-time](docs/PHASE10_ARCHITECTURE.md)
- [Phase 11: Storage](docs/PHASE11_ARCHITECTURE.md)
- [Phase 12: Functions](docs/PHASE12_ARCHITECTURE.md)
- [Phase 13: Admin UI](docs/PHASE13_ARCHITECTURE.md)

---

## Development

### Building from Source

```bash
# Debug build (faster compilation)
cargo build

# Release build (optimized)
cargo build --release
```

### Running Tests

```bash
# All tests
cargo test

# With output
cargo test -- --nocapture

# Specific test
cargo test test_mvcc_visibility
```

### Frontend Development

```bash
cd dashboard

# Development server (hot reload)
npm run dev

# Type checking
npm run build

# Linting
npm run lint
```

---

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests (`cargo test && cd dashboard && npm test`)
5. Commit your changes (`git commit -m 'Add amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

### Code Standards

- **Rust**: Follow `rustfmt` and `clippy` guidelines
- **TypeScript/Vue**: Follow the existing code style
- **Documentation**: Update relevant docs for any changes
- **Tests**: Add tests for new features

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## Acknowledgments

Built with discipline and a commitment to production-grade reliability.

**AeroDB**: Trust earned through correctness.

---

## Support

- 📧 **Issues**: [GitHub Issues](https://github.com/eshanized/AeroDB/issues)
- 📚 **Documentation**: [docs/](docs/)
- 💬 **Discussions**: [GitHub Discussions](https://github.com/eshanized/AeroDB/discussions)

---

<div align="center">

Made with ❤️ for engineers who value predictability

</div>
