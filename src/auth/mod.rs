//! # AeroDB Auth Module
//!
//! Phase 8: Authentication & Authorization
//!
//! This module provides user authentication, session management,
//! JWT tokens, and Row-Level Security for AeroDB.

pub mod api;
pub mod crypto;
pub mod email;
pub mod errors;
pub mod jwt;
pub mod rls;
pub mod session;
pub mod user;

pub use errors::{AuthError, AuthResult};
pub use jwt::{JwtClaims, JwtManager};
pub use rls::{RlsContext, RlsEnforcer, RlsPolicy};
pub use session::{Session, SessionManager};
pub use user::{User, UserRepository};
