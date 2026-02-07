//! # Signed URL Generation

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};

use super::errors::{StorageError, StorageResult};

/// Signed URL generator
#[derive(Debug)]
pub struct SignedUrlGenerator {
    secret: Vec<u8>,
    default_expiry: Duration,
}

impl SignedUrlGenerator {
    /// Create a new generator
    pub fn new(secret: &[u8]) -> Self {
        Self {
            secret: secret.to_vec(),
            default_expiry: Duration::hours(1),
        }
    }

    /// Generate a signed URL
    pub fn generate(
        &self,
        bucket: &str,
        path: &str,
        expires_at: Option<DateTime<Utc>>,
    ) -> SignedUrl {
        let expires = expires_at.unwrap_or_else(|| Utc::now() + self.default_expiry);
        let expires_ts = expires.timestamp();

        let message = format!("{}/{}/{}", bucket, path, expires_ts);
        let signature = self.sign(&message);

        SignedUrl {
            bucket: bucket.to_string(),
            path: path.to_string(),
            expires_at: expires,
            signature,
        }
    }

    /// Verify a signed URL
    pub fn verify(&self, url: &SignedUrl) -> StorageResult<()> {
        // Check expiry
        if Utc::now() > url.expires_at {
            return Err(StorageError::UrlExpired);
        }

        // Verify signature
        let message = format!("{}/{}/{}", url.bucket, url.path, url.expires_at.timestamp());
        let expected = self.sign(&message);

        if url.signature != expected {
            return Err(StorageError::InvalidSignature);
        }

        Ok(())
    }

    fn sign(&self, message: &str) -> String {
        // Simple SHA-256 based signature (secret + message)
        let mut hasher = Sha256::new();
        hasher.update(&self.secret);
        hasher.update(message.as_bytes());
        let result = hasher.finalize();
        URL_SAFE_NO_PAD.encode(result)
    }
}

/// A signed URL
#[derive(Debug, Clone)]
pub struct SignedUrl {
    pub bucket: String,
    pub path: String,
    pub expires_at: DateTime<Utc>,
    pub signature: String,
}

impl SignedUrl {
    /// Generate the URL string
    pub fn to_url(&self, base_url: &str) -> String {
        format!(
            "{}/storage/v1/object/sign/{}/{}?token={}&expires={}",
            base_url,
            self.bucket,
            self.path,
            self.signature,
            self.expires_at.timestamp()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_verify() {
        let generator = SignedUrlGenerator::new(b"test-secret");

        let signed = generator.generate("avatars", "user/123.png", None);
        assert!(!signed.signature.is_empty());

        // Should verify successfully
        assert!(generator.verify(&signed).is_ok());
    }

    #[test]
    fn test_expired_url() {
        let generator = SignedUrlGenerator::new(b"test-secret");

        let expired = SignedUrl {
            bucket: "test".to_string(),
            path: "file.txt".to_string(),
            expires_at: Utc::now() - Duration::hours(1),
            signature: "fake".to_string(),
        };

        assert!(matches!(
            generator.verify(&expired),
            Err(StorageError::UrlExpired)
        ));
    }

    #[test]
    fn test_invalid_signature() {
        let generator = SignedUrlGenerator::new(b"test-secret");

        let invalid = SignedUrl {
            bucket: "test".to_string(),
            path: "file.txt".to_string(),
            expires_at: Utc::now() + Duration::hours(1),
            signature: "bad-signature".to_string(),
        };

        assert!(matches!(
            generator.verify(&invalid),
            Err(StorageError::InvalidSignature)
        ));
    }

    #[test]
    fn test_to_url() {
        let generator = SignedUrlGenerator::new(b"secret");
        let signed = generator.generate("bucket", "path/file.txt", None);

        let url = signed.to_url("https://api.example.com");
        assert!(url.contains("/storage/v1/object/sign/"));
        assert!(url.contains("bucket"));
        assert!(url.contains("token="));
    }
}
