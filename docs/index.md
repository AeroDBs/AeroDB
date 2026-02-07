# AeroDB Documentation

Welcome to the official documentation for **AeroDB**, a strict, deterministic, and production-grade database system.

<div class="grid cards" markdown>

-   :fontawesome-solid-rocket: __Getting Started__
    
    Learn how to install, configure, and run AeroDB in minutes.
    
    [:octicons-arrow-right-24: Quick Start](getting-started/quickstart.md)

-   :fontawesome-solid-layer-group: __Core Concepts__
    
    Understand the philosophy of correctness, determinism, and reliability.
    
    [:octicons-arrow-right-24: Principles](CORE_VISION.md)

-   :fontawesome-solid-database: __Architecture__
    
    Deep dive into MVCC, WAL, Storage, and Replication/Failover models.
    
    [:octicons-arrow-right-24: Architecture](CORE_SCOPE.md)

-   :fontawesome-solid-code: __API Reference__
    
    Comprehensive guide to HTTP endpoints, Authentication, and Real-time APIs.
    
    [:octicons-arrow-right-24: API Spec](CORE_API_SPEC.md)

</div>

---

## Key Features

!!! note "Production Readiness"
    AeroDB is designed for mission-critical systems where data loss or corruption is unacceptable.

### :material-check-decagram: Correctness First
- **Strict Schema Enforcement**: No schemaless writes
- **Deterministic Query Execution**: Predictable performance
- **Crash Safety**: WAL-backed durability for every write

### :material-server-network: Robust Infrastructure
- **Leader-Follower Replication**: Atomic failover with split-brain protection
- **Point-in-Time Recovery**: Consistent snapshots and restoration
- **Zero-Dependency**: Self-contained binary, easy self-hosting

### :material-view-dashboard: Modern Developer Experience
- **Admin Dashboard**: Visual management of data, users, and cluster
- **Real-time Subscriptions**: WebSocket-based live updates
- **Serverless Functions**: Integrated WASM runtime

---

## Documentation Structure

The documentation is organized into clear sections matching the system's architecture:

1.  **Core System**: Fundamental invariants, storage engine, and lifecycle.
2.  **MVCC**: Multi-Version Concurrency Control implementation details.
3.  **Replication**: Distributed consensus, failover, and data synchronization.
4.  **Performance**: Optimization strategies, proof rules, and benchmarks.
5.  **Developer Experience**: SDKs, UI, and observability tools.
6.  **Implementation Phases**: Detailed roadmap and architectural decisions for each phase.

---

## Contributing

AeroDB is open source software. We welcome contributions that align with our core values of reliability and correctness.

[Contribution Guidelines](CONTRIBUTING.md){ .md-button .md-button--primary }
[Security Policy](SECURITY.md){ .md-button }
