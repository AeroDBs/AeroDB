//! aerodb - A strict, deterministic, self-hostable database
//!
//! Core abstractions provide unified operation model and execution pipeline.

pub mod api;
pub mod auth;
pub mod backup;
pub mod checkpoint;
pub mod cli;
pub mod core;
pub mod crash_point;
pub mod dx;
pub mod executor;
pub mod file_storage;
pub mod functions;
pub mod http_server;
pub mod index;
pub mod mvcc;
pub mod observability;
pub mod performance;
pub mod planner;
pub mod promotion;
pub mod realtime;
pub mod recovery;
pub mod replication;
pub mod rest_api;
pub mod restore;
pub mod schema;
pub mod snapshot;
pub mod storage;
pub mod wal;
