//! # Multi-Factor Authentication (MFA)
//!
//! TOTP-based multi-factor authentication using RFC 6238.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

use super::errors::{AuthError, AuthResult};

// ==================
// TOTP Configuration
// ==================

/// TOTP configuration
#[derive(Debug, Clone)]
pub struct TotpConfig {
    /// Issuer name (shown in authenticator apps)
    pub issuer: String,
    /// Number of digits (default: 6)
    pub digits: u32,
    /// Time step in seconds (default: 30)
    pub period: u64,
    /// Algorithm (default: SHA1 for compatibility)
    pub algorithm: TotpAlgorithm,
    /// Number of periods to check before/after current (default: 1)
    pub skew: u32,
}

impl Default for TotpConfig {
    fn default() -> Self {
        Self {
            issuer: "AeroDB".to_string(),
            digits: 6,
            period: 30,
            algorithm: TotpAlgorithm::SHA1,
            skew: 1,
        }
    }
}

/// TOTP hash algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TotpAlgorithm {
    SHA1,
    SHA256,
    SHA512,
}

impl std::fmt::Display for TotpAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TotpAlgorithm::SHA1 => write!(f, "SHA1"),
            TotpAlgorithm::SHA256 => write!(f, "SHA256"),
            TotpAlgorithm::SHA512 => write!(f, "SHA512"),
        }
    }
}

// ==================
// MFA Factor Types
// ==================

/// Type of MFA factor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MfaFactorType {
    TOTP,
    // Future: SMS, Email, WebAuthn
}

/// Status of an MFA factor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MfaFactorStatus {
    /// Factor is set up but not yet verified
    Unverified,
    /// Factor is active and can be used
    Verified,
    /// Factor is disabled
    Disabled,
}

// ==================
// MFA Factor
// ==================

/// An MFA factor enrolled for a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaFactor {
    pub id: Uuid,
    pub user_id: Uuid,
    pub factor_type: MfaFactorType,
    pub friendly_name: Option<String>,
    pub status: MfaFactorStatus,
    /// Secret key (encrypted in storage)
    #[serde(skip_serializing)]
    pub secret: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl MfaFactor {
    pub fn new_totp(user_id: Uuid, friendly_name: Option<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            factor_type: MfaFactorType::TOTP,
            friendly_name,
            status: MfaFactorStatus::Unverified,
            secret: generate_secret(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn is_active(&self) -> bool {
        self.status == MfaFactorStatus::Verified
    }
}

// ==================
// TOTP Implementation
// ==================

/// Generate a random secret (Base32 encoded)
fn generate_secret() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 20] = rng.gen();
    base32_encode(&bytes)
}

/// Base32 encoding (RFC 4648)
fn base32_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut result = String::new();
    let mut buffer: u64 = 0;
    let mut bits_left = 0;

    for &byte in data {
        buffer = (buffer << 8) | (byte as u64);
        bits_left += 8;

        while bits_left >= 5 {
            bits_left -= 5;
            let index = ((buffer >> bits_left) & 0x1F) as usize;
            result.push(ALPHABET[index] as char);
        }
    }

    if bits_left > 0 {
        let index = ((buffer << (5 - bits_left)) & 0x1F) as usize;
        result.push(ALPHABET[index] as char);
    }

    result
}

/// Base32 decoding
fn base32_decode(encoded: &str) -> Option<Vec<u8>> {
    const ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut result = Vec::new();
    let mut buffer: u64 = 0;
    let mut bits_left = 0;

    for c in encoded.chars() {
        let c = c.to_ascii_uppercase();
        if c == '=' {
            continue;
        }
        let value = ALPHABET.find(c)? as u64;
        buffer = (buffer << 5) | value;
        bits_left += 5;

        if bits_left >= 8 {
            bits_left -= 8;
            result.push((buffer >> bits_left) as u8);
        }
    }

    Some(result)
}

/// Generate TOTP code for a given timestamp
pub fn generate_totp(secret: &str, timestamp: u64, config: &TotpConfig) -> AuthResult<String> {
    let secret_bytes = base32_decode(secret)
        .ok_or_else(|| AuthError::MfaError("Invalid secret".to_string()))?;

    let counter = timestamp / config.period;
    let counter_bytes = counter.to_be_bytes();

    // HMAC-SHA1/256/512
    let hash = compute_hmac(&secret_bytes, &counter_bytes, config.algorithm);

    // Dynamic truncation
    let offset = (hash[hash.len() - 1] & 0x0F) as usize;
    let binary = ((hash[offset] & 0x7F) as u32) << 24
        | (hash[offset + 1] as u32) << 16
        | (hash[offset + 2] as u32) << 8
        | (hash[offset + 3] as u32);

    let otp = binary % 10u32.pow(config.digits);
    Ok(format!("{:0>width$}", otp, width = config.digits as usize))
}

/// Compute HMAC with the specified algorithm
fn compute_hmac(key: &[u8], data: &[u8], algorithm: TotpAlgorithm) -> Vec<u8> {
    use hmac::{Hmac, Mac};
    use sha1::Sha1;
    use sha2::{Sha256, Sha512};

    match algorithm {
        TotpAlgorithm::SHA1 => {
            let mut mac = Hmac::<Sha1>::new_from_slice(key).expect("HMAC can accept any key size");
            mac.update(data);
            mac.finalize().into_bytes().to_vec()
        }
        TotpAlgorithm::SHA256 => {
            let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC can accept any key size");
            mac.update(data);
            mac.finalize().into_bytes().to_vec()
        }
        TotpAlgorithm::SHA512 => {
            let mut mac = Hmac::<Sha512>::new_from_slice(key).expect("HMAC can accept any key size");
            mac.update(data);
            mac.finalize().into_bytes().to_vec()
        }
    }
}

/// Verify a TOTP code
pub fn verify_totp(secret: &str, code: &str, config: &TotpConfig) -> AuthResult<bool> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AuthError::MfaError("System time error".to_string()))?
        .as_secs();

    // Check current and adjacent time periods
    for offset in 0..=config.skew {
        // Check current + offset
        let ts = now + (offset as u64 * config.period);
        if generate_totp(secret, ts, config)? == code {
            return Ok(true);
        }

        // Check current - offset (skip 0 to avoid duplicate)
        if offset > 0 {
            let ts = now.saturating_sub(offset as u64 * config.period);
            if generate_totp(secret, ts, config)? == code {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

/// Generate otpauth:// URI for QR code
pub fn generate_totp_uri(
    secret: &str,
    email: &str,
    config: &TotpConfig,
) -> String {
    format!(
        "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm={}&digits={}&period={}",
        urlencoding::encode(&config.issuer),
        urlencoding::encode(email),
        secret,
        urlencoding::encode(&config.issuer),
        config.algorithm,
        config.digits,
        config.period
    )
}

// ==================
// Recovery Codes
// ==================

/// Generate recovery codes
pub fn generate_recovery_codes(count: usize) -> Vec<String> {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    (0..count)
        .map(|_| {
            let code: [u8; 5] = rng.gen();
            format!(
                "{:02x}{:02x}-{:02x}{:02x}-{:02x}",
                code[0], code[1], code[2], code[3], code[4]
            )
        })
        .collect()
}

/// Hash a recovery code for storage
pub fn hash_recovery_code(code: &str) -> String {
    use sha2::{Sha256, Digest};
    let normalized = code.replace("-", "").to_lowercase();
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    hex::encode(hasher.finalize())
}

// ==================
// MFA Repository Trait
// ==================

/// Repository for MFA factors
pub trait MfaRepository: Send + Sync {
    /// Find all factors for a user
    fn find_by_user_id(&self, user_id: Uuid) -> AuthResult<Vec<MfaFactor>>;

    /// Find factor by ID
    fn find_by_id(&self, factor_id: Uuid) -> AuthResult<Option<MfaFactor>>;

    /// Create a new factor
    fn create(&self, factor: MfaFactor) -> AuthResult<MfaFactor>;

    /// Update factor status
    fn update_status(&self, factor_id: Uuid, status: MfaFactorStatus) -> AuthResult<()>;

    /// Delete a factor
    fn delete(&self, factor_id: Uuid) -> AuthResult<()>;
}

/// In-memory MFA repository for testing
pub struct InMemoryMfaRepository {
    factors: std::sync::RwLock<Vec<MfaFactor>>,
}

impl InMemoryMfaRepository {
    pub fn new() -> Self {
        Self {
            factors: std::sync::RwLock::new(Vec::new()),
        }
    }
}

impl Default for InMemoryMfaRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MfaRepository for InMemoryMfaRepository {
    fn find_by_user_id(&self, user_id: Uuid) -> AuthResult<Vec<MfaFactor>> {
        let factors = self.factors.read().unwrap();
        Ok(factors.iter().filter(|f| f.user_id == user_id).cloned().collect())
    }

    fn find_by_id(&self, factor_id: Uuid) -> AuthResult<Option<MfaFactor>> {
        let factors = self.factors.read().unwrap();
        Ok(factors.iter().find(|f| f.id == factor_id).cloned())
    }

    fn create(&self, factor: MfaFactor) -> AuthResult<MfaFactor> {
        let mut factors = self.factors.write().unwrap();
        factors.push(factor.clone());
        Ok(factor)
    }

    fn update_status(&self, factor_id: Uuid, status: MfaFactorStatus) -> AuthResult<()> {
        let mut factors = self.factors.write().unwrap();
        if let Some(f) = factors.iter_mut().find(|f| f.id == factor_id) {
            f.status = status;
            f.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    fn delete(&self, factor_id: Uuid) -> AuthResult<()> {
        let mut factors = self.factors.write().unwrap();
        factors.retain(|f| f.id != factor_id);
        Ok(())
    }
}

// ==================
// MFA Service
// ==================

/// MFA service for managing factors
pub struct MfaService<R: MfaRepository> {
    repo: std::sync::Arc<R>,
    config: TotpConfig,
}

impl<R: MfaRepository> MfaService<R> {
    pub fn new(repo: std::sync::Arc<R>, config: TotpConfig) -> Self {
        Self { repo, config }
    }

    /// Enroll a new TOTP factor
    pub fn enroll_totp(
        &self,
        user_id: Uuid,
        friendly_name: Option<String>,
        email: &str,
    ) -> AuthResult<(MfaFactor, String)> {
        let factor = MfaFactor::new_totp(user_id, friendly_name);
        let uri = generate_totp_uri(&factor.secret, email, &self.config);

        let created = self.repo.create(factor)?;
        Ok((created, uri))
    }

    /// Verify and activate a TOTP factor
    pub fn verify_enrollment(&self, factor_id: Uuid, code: &str) -> AuthResult<bool> {
        let factor = self.repo.find_by_id(factor_id)?
            .ok_or_else(|| AuthError::MfaError("Factor not found".to_string()))?;

        if factor.status != MfaFactorStatus::Unverified {
            return Err(AuthError::MfaError("Factor already verified".to_string()));
        }

        if verify_totp(&factor.secret, code, &self.config)? {
            self.repo.update_status(factor_id, MfaFactorStatus::Verified)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Verify a TOTP code for authentication
    pub fn verify_code(&self, user_id: Uuid, code: &str) -> AuthResult<bool> {
        let factors = self.repo.find_by_user_id(user_id)?;
        let active_totp = factors.iter()
            .find(|f| f.factor_type == MfaFactorType::TOTP && f.is_active());

        match active_totp {
            Some(factor) => verify_totp(&factor.secret, code, &self.config),
            None => Err(AuthError::MfaError("No active TOTP factor".to_string())),
        }
    }

    /// Disable a factor
    pub fn disable_factor(&self, factor_id: Uuid) -> AuthResult<()> {
        self.repo.update_status(factor_id, MfaFactorStatus::Disabled)
    }

    /// Remove a factor
    pub fn remove_factor(&self, factor_id: Uuid) -> AuthResult<()> {
        self.repo.delete(factor_id)
    }

    /// Check if user has MFA enabled
    pub fn is_mfa_enabled(&self, user_id: Uuid) -> AuthResult<bool> {
        let factors = self.repo.find_by_user_id(user_id)?;
        Ok(factors.iter().any(|f| f.is_active()))
    }

    /// Get enrolled factors for a user
    pub fn get_factors(&self, user_id: Uuid) -> AuthResult<Vec<MfaFactor>> {
        self.repo.find_by_user_id(user_id)
    }
}

// ==================
// Tests
// ==================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_secret() {
        let secret = generate_secret();
        assert_eq!(secret.len(), 32); // 20 bytes -> 32 base32 chars
        assert!(secret.chars().all(|c| "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567".contains(c)));
    }

    #[test]
    fn test_base32_roundtrip() {
        let original = b"Hello, World!";
        let encoded = base32_encode(original);
        let decoded = base32_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_generate_totp() {
        let secret = "JBSWY3DPEHPK3PXP"; // Test secret
        let config = TotpConfig::default();

        // Generate code for a known timestamp
        let code = generate_totp(secret, 59, &config).unwrap();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_verify_totp() {
        let secret = generate_secret();
        let config = TotpConfig::default();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let valid_code = generate_totp(&secret, now, &config).unwrap();
        assert!(verify_totp(&secret, &valid_code, &config).unwrap());

        // Invalid code should fail
        assert!(!verify_totp(&secret, "000000", &config).unwrap());
    }

    #[test]
    fn test_generate_recovery_codes() {
        let codes = generate_recovery_codes(10);
        assert_eq!(codes.len(), 10);

        for code in &codes {
            // Format: xxxx-xxxx-xx (4 + 1 + 4 + 1 + 2 = 12)
            assert_eq!(code.len(), 12);
            assert!(code.contains("-"));
        }

        // All codes should be unique
        let unique: std::collections::HashSet<_> = codes.iter().collect();
        assert_eq!(unique.len(), codes.len());
    }

    #[test]
    fn test_totp_uri() {
        let config = TotpConfig::default();
        let uri = generate_totp_uri("JBSWY3DPEHPK3PXP", "user@example.com", &config);

        assert!(uri.starts_with("otpauth://totp/"));
        assert!(uri.contains("user%40example.com"));
        assert!(uri.contains("secret=JBSWY3DPEHPK3PXP"));
        assert!(uri.contains("issuer=AeroDB"));
    }

    #[test]
    fn test_mfa_factor_creation() {
        let user_id = Uuid::new_v4();
        let factor = MfaFactor::new_totp(user_id, Some("My Phone".to_string()));

        assert_eq!(factor.user_id, user_id);
        assert_eq!(factor.factor_type, MfaFactorType::TOTP);
        assert_eq!(factor.status, MfaFactorStatus::Unverified);
        assert!(!factor.is_active());
    }

    #[test]
    fn test_mfa_service_enroll() {
        let repo = std::sync::Arc::new(InMemoryMfaRepository::new());
        let service = MfaService::new(repo, TotpConfig::default());

        let user_id = Uuid::new_v4();
        let (factor, uri) = service.enroll_totp(user_id, None, "user@example.com").unwrap();

        assert_eq!(factor.status, MfaFactorStatus::Unverified);
        assert!(uri.starts_with("otpauth://totp/"));
    }

    #[test]
    fn test_mfa_service_verify() {
        let repo = std::sync::Arc::new(InMemoryMfaRepository::new());
        let config = TotpConfig::default();
        let service = MfaService::new(repo, config.clone());

        let user_id = Uuid::new_v4();
        let (factor, _) = service.enroll_totp(user_id, None, "user@example.com").unwrap();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let valid_code = generate_totp(&factor.secret, now, &config).unwrap();
        assert!(service.verify_enrollment(factor.id, &valid_code).unwrap());

        // Factor should now be verified
        assert!(service.is_mfa_enabled(user_id).unwrap());
    }
}
