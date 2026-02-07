//! # Function Registry

use std::collections::HashMap;
use std::sync::RwLock;

use super::errors::{FunctionError, FunctionResult};
use super::function::Function;
use super::trigger::TriggerType;

/// Registry of deployed functions
#[derive(Debug, Default)]
pub struct FunctionRegistry {
    /// Functions by ID
    by_id: RwLock<HashMap<String, Function>>,

    /// Function IDs by name
    by_name: RwLock<HashMap<String, String>>,

    /// Function IDs by trigger identifier
    by_trigger: RwLock<HashMap<String, Vec<String>>>,
}

impl FunctionRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a function
    pub fn register(&self, function: Function) -> FunctionResult<()> {
        let id = function.id.to_string();
        let name = function.name.clone();
        let trigger_id = function.trigger.identifier();

        // Check for duplicate name
        {
            let by_name = self
                .by_name
                .read()
                .map_err(|_| FunctionError::Internal("Lock poisoned".into()))?;
            if by_name.contains_key(&name) {
                return Err(FunctionError::AlreadyExists(name));
            }
        }

        // Insert into all indexes
        {
            let mut by_id = self
                .by_id
                .write()
                .map_err(|_| FunctionError::Internal("Lock poisoned".into()))?;
            by_id.insert(id.clone(), function);
        }

        {
            let mut by_name = self
                .by_name
                .write()
                .map_err(|_| FunctionError::Internal("Lock poisoned".into()))?;
            by_name.insert(name, id.clone());
        }

        {
            let mut by_trigger = self
                .by_trigger
                .write()
                .map_err(|_| FunctionError::Internal("Lock poisoned".into()))?;
            by_trigger.entry(trigger_id).or_default().push(id);
        }

        Ok(())
    }

    /// Get function by name
    pub fn get(&self, name: &str) -> FunctionResult<Function> {
        let id = {
            let by_name = self
                .by_name
                .read()
                .map_err(|_| FunctionError::Internal("Lock poisoned".into()))?;
            by_name
                .get(name)
                .cloned()
                .ok_or_else(|| FunctionError::NotFound(name.to_string()))?
        };

        let by_id = self
            .by_id
            .read()
            .map_err(|_| FunctionError::Internal("Lock poisoned".into()))?;
        by_id
            .get(&id)
            .cloned()
            .ok_or_else(|| FunctionError::NotFound(name.to_string()))
    }

    /// Get functions matching a trigger
    pub fn get_by_trigger(&self, trigger: &TriggerType) -> Vec<Function> {
        let trigger_id = trigger.identifier();

        let ids: Vec<String> = {
            if let Ok(by_trigger) = self.by_trigger.read() {
                by_trigger.get(&trigger_id).cloned().unwrap_or_default()
            } else {
                return Vec::new();
            }
        };

        let mut functions = Vec::new();
        if let Ok(by_id) = self.by_id.read() {
            for id in ids {
                if let Some(func) = by_id.get(&id) {
                    if func.enabled {
                        functions.push(func.clone());
                    }
                }
            }
        }

        functions
    }

    /// Unregister a function
    pub fn unregister(&self, name: &str) -> FunctionResult<()> {
        let function = self.get(name)?;
        let id = function.id.to_string();
        let trigger_id = function.trigger.identifier();

        // Remove from all indexes
        {
            let mut by_id = self
                .by_id
                .write()
                .map_err(|_| FunctionError::Internal("Lock poisoned".into()))?;
            by_id.remove(&id);
        }

        {
            let mut by_name = self
                .by_name
                .write()
                .map_err(|_| FunctionError::Internal("Lock poisoned".into()))?;
            by_name.remove(name);
        }

        {
            let mut by_trigger = self
                .by_trigger
                .write()
                .map_err(|_| FunctionError::Internal("Lock poisoned".into()))?;
            if let Some(ids) = by_trigger.get_mut(&trigger_id) {
                ids.retain(|i| i != &id);
            }
        }

        Ok(())
    }

    /// List all functions
    pub fn list(&self) -> Vec<Function> {
        self.by_id
            .read()
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Get function count
    pub fn len(&self) -> usize {
        self.by_id.read().map(|m| m.len()).unwrap_or(0)
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get() {
        let registry = FunctionRegistry::new();

        let func = Function::new(
            "hello".to_string(),
            TriggerType::http("/hello".to_string()),
            vec![1, 2, 3],
        );

        registry.register(func).unwrap();
        assert_eq!(registry.len(), 1);

        let fetched = registry.get("hello").unwrap();
        assert_eq!(fetched.name, "hello");
    }

    #[test]
    fn test_duplicate_name() {
        let registry = FunctionRegistry::new();

        let func1 = Function::new(
            "test".to_string(),
            TriggerType::http("/test".to_string()),
            vec![1],
        );
        let func2 = Function::new(
            "test".to_string(),
            TriggerType::http("/test2".to_string()),
            vec![2],
        );

        registry.register(func1).unwrap();
        assert!(registry.register(func2).is_err());
    }

    #[test]
    fn test_unregister() {
        let registry = FunctionRegistry::new();

        let func = Function::new(
            "removeme".to_string(),
            TriggerType::http("/remove".to_string()),
            vec![1],
        );

        registry.register(func).unwrap();
        assert_eq!(registry.len(), 1);

        registry.unregister("removeme").unwrap();
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_get_by_trigger() {
        let registry = FunctionRegistry::new();

        let func = Function::new(
            "on_insert".to_string(),
            TriggerType::database(
                "users".to_string(),
                super::super::trigger::DbEventType::Insert,
            ),
            vec![1],
        );

        registry.register(func).unwrap();

        let trigger = TriggerType::database(
            "users".to_string(),
            super::super::trigger::DbEventType::Insert,
        );
        let functions = registry.get_by_trigger(&trigger);

        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "on_insert");
    }
}
