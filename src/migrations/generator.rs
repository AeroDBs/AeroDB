//! # Migration Generator
//!
//! MANIFESTO ALIGNMENT: Deterministic migration file generation.
//!
//! This module generates new migration files with proper structure,
//! versioning, and checksumming.

use super::checksum::compute_checksum;
use super::errors::{MigrationError, MigrationResult};
use super::MigrationVersion;
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};

/// Migration generator
pub struct MigrationGenerator {
    migrations_dir: PathBuf,
}

impl MigrationGenerator {
    /// Create a new migration generator
    pub fn new(migrations_dir: PathBuf) -> Self {
        Self { migrations_dir }
    }

    /// Generate a new migration file
    ///
    /// MANIFESTO ALIGNMENT: Creates properly structured, checksummed migration.
    pub fn create(&self, name: &str) -> MigrationResult<PathBuf> {
        // Ensure migrations directory exists
        if !self.migrations_dir.exists() {
            fs::create_dir_all(&self.migrations_dir).map_err(|e| MigrationError::FileWrite {
                path: self.migrations_dir.clone(),
                source: e,
            })?;
        }

        // Determine next version number
        let next_version = self.next_version()?;

        // Sanitize name (lowercase, underscores)
        let sanitized_name = name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect::<String>();

        // Generate filename
        let filename = format!("{:03}_{}.yaml", next_version, sanitized_name);
        let file_path = self.migrations_dir.join(&filename);

        // Generate content
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let content_without_checksum = format!(
            r#"# Migration: {}
# Created: {}
#
# MANIFESTO ALIGNMENT: Deterministic, reversible migration.
# Edit the 'up' and 'down' sections to define your schema changes.

version: {}
timestamp: "{}"
up:
  # Add your forward migration operations here
  # Example:
  # - create_collection:
  #     name: my_collection
  #     schema:
  #       properties:
  #         field_name:
  #           type: string
  #           required: true
  - create_collection:
      name: placeholder
      schema: {{}}

down:
  # Add your rollback operations here (reverse of 'up')
  # Example:
  # - drop_collection:
  #     name: my_collection
  - drop_collection:
      name: placeholder
"#,
            sanitized_name, timestamp, next_version, timestamp
        );

        // Compute checksum
        let checksum = compute_checksum(&content_without_checksum);

        // Insert checksum into content
        let content = format!(
            r#"# Migration: {}
# Created: {}
#
# MANIFESTO ALIGNMENT: Deterministic, reversible migration.
# Edit the 'up' and 'down' sections to define your schema changes.

version: {}
checksum: "{}"
timestamp: "{}"
up:
  # Add your forward migration operations here
  # Example:
  # - create_collection:
  #     name: my_collection
  #     schema:
  #       properties:
  #         field_name:
  #           type: string
  #           required: true
  - create_collection:
      name: placeholder
      schema: {{}}

down:
  # Add your rollback operations here (reverse of 'up')
  # Example:
  # - drop_collection:
  #     name: my_collection
  - drop_collection:
      name: placeholder
"#,
            sanitized_name, timestamp, next_version, checksum, timestamp
        );

        // Write file
        fs::write(&file_path, &content).map_err(|e| MigrationError::FileWrite {
            path: file_path.clone(),
            source: e,
        })?;

        Ok(file_path)
    }

    /// Determine the next version number
    fn next_version(&self) -> MigrationResult<MigrationVersion> {
        if !self.migrations_dir.exists() {
            return Ok(1);
        }

        let mut max_version: MigrationVersion = 0;

        for entry in fs::read_dir(&self.migrations_dir).map_err(|e| MigrationError::FileRead {
            path: self.migrations_dir.clone(),
            source: e,
        })? {
            let entry = entry.map_err(|e| MigrationError::FileRead {
                path: self.migrations_dir.clone(),
                source: e,
            })?;

            let path = entry.path();
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                // Parse version from filename: 001_name -> 1
                if let Some(idx) = stem.find('_') {
                    if let Ok(version) = stem[..idx].parse::<MigrationVersion>() {
                        max_version = max_version.max(version);
                    }
                } else if let Ok(version) = stem.parse::<MigrationVersion>() {
                    max_version = max_version.max(version);
                }
            }
        }

        Ok(max_version + 1)
    }

    /// Generate a blank migration template
    pub fn template() -> String {
        r#"# Migration template
#
# MANIFESTO ALIGNMENT: Deterministic, reversible migration.

version: 1
checksum: ""
timestamp: "2026-01-01T00:00:00Z"

up:
  # Forward migration operations
  - create_collection:
      name: example
      schema:
        properties:
          id:
            type: string
            required: true
          name:
            type: string
        indexes:
          - fields: [id]
            unique: true

down:
  # Rollback operations (reverse of 'up')
  - drop_collection:
      name: example
"#
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_first_migration() {
        let temp_dir = TempDir::new().unwrap();
        let migrations_dir = temp_dir.path().join("migrations");

        let generator = MigrationGenerator::new(migrations_dir.clone());
        let path = generator.create("create_users").unwrap();

        assert!(path.exists());
        assert!(path.to_string_lossy().contains("001_create_users.yaml"));

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("version: 1"));
        assert!(content.contains("checksum:"));
    }

    #[test]
    fn test_create_sequential_migrations() {
        let temp_dir = TempDir::new().unwrap();
        let migrations_dir = temp_dir.path().join("migrations");

        let generator = MigrationGenerator::new(migrations_dir.clone());

        let path1 = generator.create("create_users").unwrap();
        let path2 = generator.create("create_posts").unwrap();
        let path3 = generator.create("add_comments").unwrap();

        assert!(path1.to_string_lossy().contains("001_"));
        assert!(path2.to_string_lossy().contains("002_"));
        assert!(path3.to_string_lossy().contains("003_"));
    }

    #[test]
    fn test_sanitize_name() {
        let temp_dir = TempDir::new().unwrap();
        let migrations_dir = temp_dir.path().join("migrations");

        let generator = MigrationGenerator::new(migrations_dir);

        // Name with special characters should be sanitized
        let path = generator.create("Add User's Table!").unwrap();
        assert!(path.to_string_lossy().contains("add_user_s_table_"));
    }

    #[test]
    fn test_template() {
        let template = MigrationGenerator::template();
        assert!(template.contains("version:"));
        assert!(template.contains("up:"));
        assert!(template.contains("down:"));
        assert!(template.contains("create_collection"));
        assert!(template.contains("drop_collection"));
    }
}
