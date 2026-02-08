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
pub mod magic_link;
pub mod mfa;
pub mod oauth;
pub mod rls;
pub mod security;
pub mod session;
pub mod user;

pub use errors::{AuthError, AuthResult};
pub use jwt::{JwtClaims, JwtManager};
pub use magic_link::{AuthEvent, AuthHookPayload, AuthHooks, MagicLinkConfig, MagicLinkService};
pub use mfa::{MfaFactor, MfaFactorType, MfaService, TotpConfig};
pub use oauth::{OAuthProvider, OAuthProviderConfig, OAuthService, OAuthUserInfo};
pub use rls::{RlsContext, RlsEnforcer, RlsPolicy};
pub use security::SecurityConfig;
pub use session::{Session, SessionManager};
pub use user::{User, UserRepository};
