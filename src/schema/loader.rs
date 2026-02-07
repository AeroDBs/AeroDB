//! Schema loader for loading schemas from disk at startup
//!
//! Per SCHEMA.md §75-90:
//! - Schemas stored at metadata/schemas/schema_<id>_<version>.json
//! - One file per schema version
//! - Missing schema files cause startup failure (FATAL)

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::errors::{SchemaError, SchemaResult};
use super::types::Schema;

/// Schema loader that reads schema files from disk and maintains an in-memory registry.
pub struct SchemaLoader {
    /// Directory containing schema files
    schema_dir: PathBuf,
    /// Loaded schemas indexed by (schema_id, schema_version)
    schemas: HashMap<(String, String), Schema>,
}

impl SchemaLoader {
    /// Creates a new schema loader for the given data directory.
    ///
    /// Schema files are expected at `<data_dir>/metadata/schemas/`.
    pub fn new(data_dir: &Path) -> Self {
        Self {
            schema_dir: data_dir.join("metadata").join("schemas"),
            schemas: HashMap::new(),
        }
    }

    /// Returns the schema directory path.
    pub fn schema_dir(&self) -> &Path {
        &self.schema_dir
    }

    /// Loads all schema files from the schema directory.
    ///
    /// Per SCHEMA.md, missing or malformed schema files cause FATAL errors.
    pub fn load_all(&mut self) -> SchemaResult<()> {
        // Create directory if it doesn't exist
        if !self.schema_dir.exists() {
            fs::create_dir_all(&self.schema_dir).map_err(|e| {
                SchemaError::malformed_schema(
                    self.schema_dir.display().to_string(),
                    format!("Failed to create schema directory: {}", e),
                )
            })?;
            return Ok(()); // No schemas to load
        }

        // Read all files in schema directory
        let entries = fs::read_dir(&self.schema_dir).map_err(|e| {
            SchemaError::malformed_schema(
                self.schema_dir.display().to_string(),
                format!("Failed to read schema directory: {}", e),
            )
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                SchemaError::malformed_schema(
                    self.schema_dir.display().to_string(),
                    format!("Failed to read directory entry: {}", e),
                )
            })?;

            let path = entry.path();

            // Skip non-JSON files
            if path.extension().map_or(true, |ext| ext != "json") {
                continue;
            }

            self.load_schema_file(&path)?;
        }

        Ok(())
    }

    /// Loads a single schema file.
    fn load_schema_file(&mut self, path: &Path) -> SchemaResult<()> {
        let content = fs::read_to_string(path).map_err(|e| {
            SchemaError::malformed_schema(
                path.display().to_string(),
                format!("Failed to read file: {}", e),
            )
        })?;

        let schema: Schema = serde_json::from_str(&content).map_err(|e| {
            SchemaError::malformed_schema(
                path.display().to_string(),
                format!("Invalid JSON: {}", e),
            )
        })?;

        // Validate schema structure
        schema
            .validate_structure()
            .map_err(|e| SchemaError::malformed_schema(path.display().to_string(), e))?;

        // Store in registry
        let key = (schema.schema_id.clone(), schema.schema_version.clone());
        self.schemas.insert(key, schema);

        Ok(())
    }

    /// Registers a schema directly (for testing or programmatic creation).
    pub fn register(&mut self, schema: Schema) -> SchemaResult<()> {
        schema
            .validate_structure()
            .map_err(|e| SchemaError::malformed_schema("<in-memory>", e))?;

        let key = (schema.schema_id.clone(), schema.schema_version.clone());

        // Check for immutability violation
        if self.schemas.contains_key(&key) {
            return Err(SchemaError::schema_immutable(
                &schema.schema_id,
                &schema.schema_version,
            ));
        }

        self.schemas.insert(key, schema);
        Ok(())
    }

    /// Gets a schema by ID and version.
    pub fn get(&self, schema_id: &str, schema_version: &str) -> Option<&Schema> {
        self.schemas
            .get(&(schema_id.to_string(), schema_version.to_string()))
    }

    /// Checks if a schema exists.
    pub fn exists(&self, schema_id: &str, schema_version: &str) -> bool {
        self.get(schema_id, schema_version).is_some()
    }

    /// Checks if any version of a schema ID exists.
    pub fn schema_id_exists(&self, schema_id: &str) -> bool {
        self.schemas.keys().any(|(id, _)| id == schema_id)
    }

    /// Returns all loaded schemas.
    pub fn all_schemas(&self) -> impl Iterator<Item = &Schema> {
        self.schemas.values()
    }

    /// Returns the number of loaded schemas.
    pub fn schema_count(&self) -> usize {
        self.schemas.len()
    }

    /// Saves a schema to disk.
    ///
    /// Creates the schema file at the standard location.
    pub fn save_schema(&self, schema: &Schema) -> SchemaResult<PathBuf> {
        let filename = format!("schema_{}_{}.json", schema.schema_id, schema.schema_version);
        let path = self.schema_dir.join(&filename);

        // Check if file already exists (immutability)
        if path.exists() {
            return Err(SchemaError::schema_immutable(
                &schema.schema_id,
                &schema.schema_version,
            ));
        }

        // Ensure directory exists
        if !self.schema_dir.exists() {
            fs::create_dir_all(&self.schema_dir).map_err(|e| {
                SchemaError::malformed_schema(
                    self.schema_dir.display().to_string(),
                    format!("Failed to create schema directory: {}", e),
                )
            })?;
        }

        // Write schema file
        let content = serde_json::to_string_pretty(schema).map_err(|e| {
            SchemaError::malformed_schema(
                path.display().to_string(),
                format!("Failed to serialize schema: {}", e),
            )
        })?;

        fs::write(&path, content).map_err(|e| {
            SchemaError::malformed_schema(
                path.display().to_string(),
                format!("Failed to write file: {}", e),
            )
        })?;

        Ok(path)
    }
}

// Implement planner's SchemaRegistry trait
impl crate::planner::SchemaRegistry for SchemaLoader {
    fn schema_exists(&self, schema_id: &str) -> bool {
        self.schema_id_exists(schema_id)
    }

    fn schema_version_exists(&self, schema_id: &str, version: &str) -> bool {
        self.exists(schema_id, version)
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::FieldDef;
    use super::*;
    use tempfile::TempDir;

    fn sample_schema() -> Schema {
        let mut fields = HashMap::new();
        fields.insert("_id".into(), FieldDef::required_string());
        fields.insert("name".into(), FieldDef::required_string());
        Schema::new("users", "v1", fields)
    }

    #[test]
    fn test_register_and_get() {
        let temp_dir = TempDir::new().unwrap();
        let mut loader = SchemaLoader::new(temp_dir.path());

        loader.register(sample_schema()).unwrap();

        let schema = loader.get("users", "v1");
        assert!(schema.is_some());
        assert_eq!(schema.unwrap().schema_id, "users");
    }

    #[test]
    fn test_schema_immutability() {
        let temp_dir = TempDir::new().unwrap();
        let mut loader = SchemaLoader::new(temp_dir.path());

        loader.register(sample_schema()).unwrap();

        // Attempt to register again should fail
        let result = loader.register(sample_schema());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code().code(), "AERO_SCHEMA_IMMUTABLE");
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let mut loader = SchemaLoader::new(temp_dir.path());

        let schema = sample_schema();
        loader.save_schema(&schema).unwrap();

        // Load from disk
        let mut loader2 = SchemaLoader::new(temp_dir.path());
        loader2.load_all().unwrap();

        assert!(loader2.exists("users", "v1"));
    }

    #[test]
    fn test_unknown_schema() {
        let temp_dir = TempDir::new().unwrap();
        let loader = SchemaLoader::new(temp_dir.path());

        assert!(loader.get("nonexistent", "v1").is_none());
        assert!(!loader.exists("nonexistent", "v1"));
    }

    #[test]
    fn test_load_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let mut loader = SchemaLoader::new(temp_dir.path());

        let result = loader.load_all();
        assert!(result.is_ok());
        assert_eq!(loader.schema_count(), 0);
    }
}
