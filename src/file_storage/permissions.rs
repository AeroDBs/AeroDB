//! # Storage Permissions

use super::bucket::{Bucket, BucketPolicy};
use super::errors::{StorageError, StorageResult};
use crate::auth::rls::RlsContext;

/// Permission checker for storage operations
#[derive(Debug, Default)]
pub struct StoragePermissions;

impl StoragePermissions {
    pub fn new() -> Self {
        Self
    }

    /// Check read permission
    pub fn check_read(&self, bucket: &Bucket, context: &RlsContext) -> StorageResult<()> {
        // Service role can read anything
        if context.can_bypass_rls() {
            return Ok(());
        }

        match bucket.config.policy {
            BucketPolicy::Public => Ok(()),
            BucketPolicy::Authenticated => {
                if context.is_authenticated {
                    Ok(())
                } else {
                    Err(StorageError::Unauthorized)
                }
            }
            BucketPolicy::Private => {
                if let Some(user_id) = &context.user_id {
                    if bucket.owner_id.as_ref() == Some(user_id) {
                        Ok(())
                    } else {
                        Err(StorageError::Forbidden)
                    }
                } else {
                    Err(StorageError::Unauthorized)
                }
            }
        }
    }

    /// Check write permission
    pub fn check_write(&self, bucket: &Bucket, context: &RlsContext) -> StorageResult<()> {
        // Service role can write anything
        if context.can_bypass_rls() {
            return Ok(());
        }

        // Must be authenticated to write
        if !context.is_authenticated {
            return Err(StorageError::Unauthorized);
        }

        match bucket.config.policy {
            BucketPolicy::Public | BucketPolicy::Authenticated => Ok(()),
            BucketPolicy::Private => {
                if let Some(user_id) = &context.user_id {
                    if bucket.owner_id.as_ref() == Some(user_id) {
                        Ok(())
                    } else {
                        Err(StorageError::Forbidden)
                    }
                } else {
                    Err(StorageError::Unauthorized)
                }
            }
        }
    }

    /// Check delete permission
    pub fn check_delete(&self, bucket: &Bucket, context: &RlsContext) -> StorageResult<()> {
        // Same as write
        self.check_write(bucket, context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_storage::bucket::BucketConfig;
    use uuid::Uuid;

    #[test]
    fn test_public_bucket_read() {
        let permissions = StoragePermissions::new();
        let mut config = BucketConfig::default();
        config.policy = BucketPolicy::Public;
        let bucket = Bucket::new("public".to_string(), None, config);

        // Anonymous can read public
        let anon = RlsContext::anonymous();
        assert!(permissions.check_read(&bucket, &anon).is_ok());
    }

    #[test]
    fn test_private_bucket_read() {
        let permissions = StoragePermissions::new();
        let owner_id = Uuid::new_v4();
        let mut config = BucketConfig::default();
        config.policy = BucketPolicy::Private;
        let bucket = Bucket::new("private".to_string(), Some(owner_id), config);

        // Anonymous cannot read
        let anon = RlsContext::anonymous();
        assert!(permissions.check_read(&bucket, &anon).is_err());

        // Owner can read
        let owner = RlsContext::authenticated(owner_id);
        assert!(permissions.check_read(&bucket, &owner).is_ok());

        // Other user cannot read
        let other = RlsContext::authenticated(Uuid::new_v4());
        assert!(permissions.check_read(&bucket, &other).is_err());
    }

    #[test]
    fn test_authenticated_bucket() {
        let permissions = StoragePermissions::new();
        let mut config = BucketConfig::default();
        config.policy = BucketPolicy::Authenticated;
        let bucket = Bucket::new("auth".to_string(), None, config);

        // Anonymous cannot read
        let anon = RlsContext::anonymous();
        assert!(permissions.check_read(&bucket, &anon).is_err());

        // Any authenticated user can read
        let user = RlsContext::authenticated(Uuid::new_v4());
        assert!(permissions.check_read(&bucket, &user).is_ok());
    }
}
