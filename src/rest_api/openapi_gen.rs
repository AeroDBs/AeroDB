//! # OpenAPI Specification Generator
//!
//! MANIFESTO ALIGNMENT: Explicit API documentation.
//!
//! Auto-generates OpenAPI 3.0 specifications from registered
//! schema endpoints, making the API self-documenting.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

use super::generator::{EndpointRegistry, FieldType, SchemaDef};

/// OpenAPI 3.0 specification generator
pub struct OpenApiGenerator {
    /// API title
    title: String,

    /// API version
    version: String,

    /// API base URL
    base_url: String,
}

impl Default for OpenApiGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenApiGenerator {
    /// Create a new OpenAPI generator
    pub fn new() -> Self {
        Self {
            title: "AeroDB REST API".to_string(),
            version: "1.0.0".to_string(),
            base_url: "http://localhost:54321".to_string(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(title: String, version: String, base_url: String) -> Self {
        Self {
            title,
            version,
            base_url,
        }
    }

    /// Generate OpenAPI 3.0 spec from endpoint registry
    pub fn generate(&self, registry: &EndpointRegistry) -> Value {
        let mut paths = HashMap::new();

        for collection in registry.collections() {
            if let Some(endpoint) = registry.get(&collection) {
                // Generate paths for this collection
                let (list_path, item_path) = self.generate_collection_paths(
                    &collection,
                    &endpoint.schema,
                );
                
                paths.insert(format!("/rest/v1/{}", collection), list_path);
                paths.insert(format!("/rest/v1/{}/{{id}}", collection), item_path);
            }
        }

        json!({
            "openapi": "3.0.3",
            "info": {
                "title": self.title,
                "version": self.version,
                "description": "Auto-generated REST API for AeroDB collections"
            },
            "servers": [
                {
                    "url": self.base_url,
                    "description": "AeroDB Server"
                }
            ],
            "paths": paths,
            "components": {
                "securitySchemes": {
                    "bearerAuth": {
                        "type": "http",
                        "scheme": "bearer",
                        "bearerFormat": "JWT"
                    },
                    "apiKey": {
                        "type": "apiKey",
                        "in": "header",
                        "name": "apikey"
                    }
                }
            },
            "security": [
                { "bearerAuth": [] },
                { "apiKey": [] }
            ]
        })
    }

    /// Generate paths for a collection
    fn generate_collection_paths(
        &self,
        collection: &str,
        schema: &SchemaDef,
    ) -> (Value, Value) {
        let schema_ref = self.field_type_to_json_schema(schema);

        // Collection-level operations (list, create)
        let list_path = json!({
            "get": {
                "summary": format!("List all {}", collection),
                "operationId": format!("list_{}", collection),
                "tags": [collection],
                "parameters": [
                    {
                        "name": "limit",
                        "in": "query",
                        "required": false,
                        "schema": { "type": "integer", "default": 100 }
                    },
                    {
                        "name": "offset",
                        "in": "query",
                        "required": false,
                        "schema": { "type": "integer", "default": 0 }
                    },
                    {
                        "name": "select",
                        "in": "query",
                        "required": false,
                        "schema": { "type": "string" }
                    },
                    {
                        "name": "order",
                        "in": "query",
                        "required": false,
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": format!("List of {}", collection),
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "data": {
                                            "type": "array",
                                            "items": schema_ref.clone()
                                        },
                                        "count": { "type": "integer" },
                                        "has_more": { "type": "boolean" }
                                    }
                                }
                            }
                        }
                    },
                    "401": { "description": "Unauthorized" },
                    "403": { "description": "Forbidden" }
                }
            },
            "post": {
                "summary": format!("Create new {} record", collection),
                "operationId": format!("create_{}", collection),
                "tags": [collection],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": schema_ref.clone()
                        }
                    }
                },
                "responses": {
                    "201": {
                        "description": "Created",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "data": schema_ref.clone(),
                                        "id": { "type": "string" }
                                    }
                                }
                            }
                        }
                    },
                    "400": { "description": "Bad Request" },
                    "401": { "description": "Unauthorized" },
                    "403": { "description": "Forbidden" }
                }
            }
        });

        // Item-level operations (get, update, delete)
        let item_path = json!({
            "get": {
                "summary": format!("Get {} by ID", collection),
                "operationId": format!("get_{}", collection),
                "tags": [collection],
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": format!("Single {} record", collection),
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "data": schema_ref.clone()
                                    }
                                }
                            }
                        }
                    },
                    "404": { "description": "Not Found" },
                    "401": { "description": "Unauthorized" },
                    "403": { "description": "Forbidden" }
                }
            },
            "patch": {
                "summary": format!("Update {} by ID", collection),
                "operationId": format!("update_{}", collection),
                "tags": [collection],
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": schema_ref.clone()
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Updated",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "data": schema_ref.clone()
                                    }
                                }
                            }
                        }
                    },
                    "404": { "description": "Not Found" },
                    "400": { "description": "Bad Request" },
                    "401": { "description": "Unauthorized" },
                    "403": { "description": "Forbidden" }
                }
            },
            "delete": {
                "summary": format!("Delete {} by ID", collection),
                "operationId": format!("delete_{}", collection),
                "tags": [collection],
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Deleted",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "deleted": { "type": "boolean" },
                                        "id": { "type": "string" }
                                    }
                                }
                            }
                        }
                    },
                    "404": { "description": "Not Found" },
                    "401": { "description": "Unauthorized" },
                    "403": { "description": "Forbidden" }
                }
            }
        });

        (list_path, item_path)
    }

    /// Convert schema to JSON Schema format
    fn field_type_to_json_schema(&self, schema: &SchemaDef) -> Value {
        let mut properties = HashMap::new();
        let mut required = Vec::new();

        for field in &schema.fields {
            let field_schema = match field.field_type {
                FieldType::Uuid => json!({ "type": "string", "format": "uuid" }),
                FieldType::String => json!({ "type": "string" }),
                FieldType::Number => json!({ "type": "number" }),
                FieldType::Boolean => json!({ "type": "boolean" }),
                FieldType::Datetime => json!({ "type": "string", "format": "date-time" }),
                FieldType::Json => json!({ "type": "object" }),
            };
            
            properties.insert(field.name.clone(), field_schema);
            
            if field.required {
                required.push(field.name.clone());
            }
        }

        json!({
            "type": "object",
            "properties": properties,
            "required": required
        })
    }
}

/// Route information for introspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteInfo {
    /// HTTP method
    pub method: String,

    /// Route path
    pub path: String,

    /// Route description
    pub description: String,

    /// Whether authentication is required
    pub requires_auth: bool,
}

/// Generate route list for introspection
pub fn generate_routes(registry: &EndpointRegistry) -> Vec<RouteInfo> {
    let mut routes = vec![
        // System routes
        RouteInfo {
            method: "GET".to_string(),
            path: "/_routes".to_string(),
            description: "List all available routes".to_string(),
            requires_auth: false,
        },
        RouteInfo {
            method: "GET".to_string(),
            path: "/_spec".to_string(),
            description: "OpenAPI 3.0 specification".to_string(),
            requires_auth: false,
        },
    ];

    // Collection routes
    for collection in registry.collections() {
        routes.push(RouteInfo {
            method: "GET".to_string(),
            path: format!("/rest/v1/{}", collection),
            description: format!("List all {} records", collection),
            requires_auth: true,
        });
        routes.push(RouteInfo {
            method: "POST".to_string(),
            path: format!("/rest/v1/{}", collection),
            description: format!("Create new {} record", collection),
            requires_auth: true,
        });
        routes.push(RouteInfo {
            method: "GET".to_string(),
            path: format!("/rest/v1/{}/{{id}}", collection),
            description: format!("Get {} by ID", collection),
            requires_auth: true,
        });
        routes.push(RouteInfo {
            method: "PATCH".to_string(),
            path: format!("/rest/v1/{}/{{id}}", collection),
            description: format!("Update {} by ID", collection),
            requires_auth: true,
        });
        routes.push(RouteInfo {
            method: "DELETE".to_string(),
            path: format!("/rest/v1/{}/{{id}}", collection),
            description: format!("Delete {} by ID", collection),
            requires_auth: true,
        });
    }

    routes
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::generator::{FieldDef, SchemaDef, SchemaEndpoint};

    fn create_test_schema() -> SchemaDef {
        SchemaDef {
            name: "users".to_string(),
            fields: vec![
                FieldDef {
                    name: "id".to_string(),
                    field_type: FieldType::Uuid,
                    required: true,
                    primary: true,
                    default: None,
                },
                FieldDef {
                    name: "name".to_string(),
                    field_type: FieldType::String,
                    required: true,
                    primary: false,
                    default: None,
                },
                FieldDef {
                    name: "email".to_string(),
                    field_type: FieldType::String,
                    required: true,
                    primary: false,
                    default: None,
                },
            ],
            rls_policy: None,
        }
    }

    #[test]
    fn test_openapi_generation() {
        let registry = EndpointRegistry::new();
        let endpoint = SchemaEndpoint::from_schema(create_test_schema());
        registry.register(endpoint).unwrap();

        let generator = OpenApiGenerator::new();
        let spec = generator.generate(&registry);

        assert_eq!(spec["openapi"], "3.0.3");
        assert_eq!(spec["info"]["title"], "AeroDB REST API");
        assert!(spec["paths"].as_object().is_some());
    }

    #[test]
    fn test_routes_generation() {
        let registry = EndpointRegistry::new();
        let endpoint = SchemaEndpoint::from_schema(create_test_schema());
        registry.register(endpoint).unwrap();

        let routes = generate_routes(&registry);

        // Should have system routes + 5 per collection
        assert!(routes.len() >= 7);

        // Check system routes exist
        assert!(routes.iter().any(|r| r.path == "/_routes"));
        assert!(routes.iter().any(|r| r.path == "/_spec"));
    }

    #[test]
    fn test_generator_config() {
        let generator = OpenApiGenerator::with_config(
            "My API".to_string(),
            "2.0.0".to_string(),
            "https://api.example.com".to_string(),
        );

        let registry = EndpointRegistry::new();
        let spec = generator.generate(&registry);

        assert_eq!(spec["info"]["title"], "My API");
        assert_eq!(spec["info"]["version"], "2.0.0");
        assert_eq!(spec["servers"][0]["url"], "https://api.example.com");
    }
}
