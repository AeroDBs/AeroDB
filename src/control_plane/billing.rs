//! # Billing Service
//!
//! Invoice generation and cost calculation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::metering::UsageMetrics;
use super::tenant::Plan;

/// Pricing per unit (in USD)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pricing {
    /// Price per 1M API requests
    pub api_requests_per_million: f64,
    /// Price per GB storage per month
    pub storage_per_gb_month: f64,
    /// Price per GB file storage per month
    pub file_storage_per_gb_month: f64,
    /// Price per GB egress
    pub egress_per_gb: f64,
    /// Price per 1000 function invocations
    pub functions_per_thousand: f64,
    /// Price per 1M ms function execution
    pub function_execution_per_million_ms: f64,
}

impl Pricing {
    /// Default pricing
    pub fn default_pricing() -> Self {
        Self {
            api_requests_per_million: 2.50,
            storage_per_gb_month: 0.25,
            file_storage_per_gb_month: 0.023,
            egress_per_gb: 0.09,
            functions_per_thousand: 0.20,
            function_execution_per_million_ms: 0.50,
        }
    }

    /// Get free tier limits (included in base plan)
    pub fn free_tier_limits() -> FreeTierLimits {
        FreeTierLimits {
            api_requests: 10_000,
            storage_gb: 0.5,
            file_storage_gb: 1.0,
            egress_gb: 2.0,
            function_invocations: 100_000,
        }
    }
}

impl Default for Pricing {
    fn default() -> Self {
        Self::default_pricing()
    }
}

/// Free tier limits (included without charge)
#[derive(Debug, Clone)]
pub struct FreeTierLimits {
    pub api_requests: u64,
    pub storage_gb: f64,
    pub file_storage_gb: f64,
    pub egress_gb: f64,
    pub function_invocations: u64,
}

/// Line item on an invoice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineItem {
    /// Description
    pub description: String,
    /// Quantity
    pub quantity: f64,
    /// Unit
    pub unit: String,
    /// Unit price
    pub unit_price: f64,
    /// Total
    pub total: f64,
}

/// Monthly invoice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    /// Invoice ID
    pub invoice_id: Uuid,
    /// Tenant ID
    pub tenant_id: Uuid,
    /// Billing month (YYYY-MM)
    pub month: String,
    /// Plan
    pub plan: Plan,
    /// Base price for plan
    pub base_price: f64,
    /// Usage-based charges
    pub usage_charges: Vec<LineItem>,
    /// Total usage charges
    pub usage_total: f64,
    /// Credits applied
    pub credits: f64,
    /// Total amount due
    pub total: f64,
    /// Currency
    pub currency: String,
    /// Generated at
    pub generated_at: DateTime<Utc>,
    /// Paid at (if paid)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paid_at: Option<DateTime<Utc>>,
}

/// Billing calculator
#[derive(Debug, Clone)]
pub struct BillingCalculator {
    pricing: Pricing,
}

impl BillingCalculator {
    /// Create a new billing calculator
    pub fn new() -> Self {
        Self {
            pricing: Pricing::default(),
        }
    }

    /// Create with custom pricing
    pub fn with_pricing(pricing: Pricing) -> Self {
        Self { pricing }
    }

    /// Get base price for a plan
    pub fn base_price(&self, plan: &Plan) -> f64 {
        match plan {
            Plan::Free => 0.0,
            Plan::Pro => 25.0,
            Plan::Enterprise => 599.0,
        }
    }

    /// Calculate usage-based charges
    pub fn calculate_usage_charges(&self, usage: &UsageMetrics, plan: &Plan) -> Vec<LineItem> {
        let mut items = Vec::new();
        let free_limits = Pricing::free_tier_limits();

        // API Requests (charge above free tier)
        let api_requests_above_free = usage.api_requests.saturating_sub(free_limits.api_requests);
        if api_requests_above_free > 0 {
            let millions = api_requests_above_free as f64 / 1_000_000.0;
            items.push(LineItem {
                description: "API Requests".to_string(),
                quantity: millions,
                unit: "millions".to_string(),
                unit_price: self.pricing.api_requests_per_million,
                total: millions * self.pricing.api_requests_per_million,
            });
        }

        // Storage
        let storage_gb = usage.storage_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let storage_above_free = (storage_gb - free_limits.storage_gb).max(0.0);
        if storage_above_free > 0.0 {
            items.push(LineItem {
                description: "Database Storage".to_string(),
                quantity: storage_above_free,
                unit: "GB".to_string(),
                unit_price: self.pricing.storage_per_gb_month,
                total: storage_above_free * self.pricing.storage_per_gb_month,
            });
        }

        // File Storage
        let file_storage_gb = usage.file_storage_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let file_above_free = (file_storage_gb - free_limits.file_storage_gb).max(0.0);
        if file_above_free > 0.0 {
            items.push(LineItem {
                description: "File Storage".to_string(),
                quantity: file_above_free,
                unit: "GB".to_string(),
                unit_price: self.pricing.file_storage_per_gb_month,
                total: file_above_free * self.pricing.file_storage_per_gb_month,
            });
        }

        // Egress
        let egress_gb = usage.egress_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let egress_above_free = (egress_gb - free_limits.egress_gb).max(0.0);
        if egress_above_free > 0.0 {
            items.push(LineItem {
                description: "Bandwidth Egress".to_string(),
                quantity: egress_above_free,
                unit: "GB".to_string(),
                unit_price: self.pricing.egress_per_gb,
                total: egress_above_free * self.pricing.egress_per_gb,
            });
        }

        // Function Invocations
        let invocations_above_free = usage
            .function_invocations
            .saturating_sub(free_limits.function_invocations);
        if invocations_above_free > 0 {
            let thousands = invocations_above_free as f64 / 1000.0;
            items.push(LineItem {
                description: "Function Invocations".to_string(),
                quantity: thousands,
                unit: "thousands".to_string(),
                unit_price: self.pricing.functions_per_thousand,
                total: thousands * self.pricing.functions_per_thousand,
            });
        }

        // Function Execution Time
        if usage.function_execution_ms > 0 {
            let million_ms = usage.function_execution_ms as f64 / 1_000_000.0;
            items.push(LineItem {
                description: "Function Execution Time".to_string(),
                quantity: million_ms,
                unit: "million ms".to_string(),
                unit_price: self.pricing.function_execution_per_million_ms,
                total: million_ms * self.pricing.function_execution_per_million_ms,
            });
        }

        items
    }

    /// Generate an invoice for a tenant
    pub fn generate_invoice(
        &self,
        tenant_id: Uuid,
        month: &str,
        plan: &Plan,
        usage: &UsageMetrics,
    ) -> Invoice {
        let base_price = self.base_price(plan);
        let usage_charges = self.calculate_usage_charges(usage, plan);
        let usage_total: f64 = usage_charges.iter().map(|i| i.total).sum();

        // Enterprise plan includes all usage
        let effective_usage_total = if *plan == Plan::Enterprise {
            0.0
        } else {
            usage_total
        };

        Invoice {
            invoice_id: Uuid::new_v4(),
            tenant_id,
            month: month.to_string(),
            plan: *plan,
            base_price,
            usage_charges,
            usage_total: effective_usage_total,
            credits: 0.0,
            total: base_price + effective_usage_total,
            currency: "USD".to_string(),
            generated_at: Utc::now(),
            paid_at: None,
        }
    }
}

impl Default for BillingCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_pricing() {
        let calc = BillingCalculator::new();
        assert_eq!(calc.base_price(&Plan::Free), 0.0);
        assert_eq!(calc.base_price(&Plan::Pro), 25.0);
        assert_eq!(calc.base_price(&Plan::Enterprise), 599.0);
    }

    #[test]
    fn test_free_tier_no_charges() {
        let calc = BillingCalculator::new();
        let tenant_id = Uuid::new_v4();

        let usage = UsageMetrics {
            tenant_id,
            month: "2024-08".to_string(),
            api_requests: 5000, // Under free tier
            storage_bytes: 100 * 1024 * 1024, // 100 MB, under 500 MB
            file_storage_bytes: 500 * 1024 * 1024, // 500 MB, under 1 GB
            egress_bytes: 1 * 1024 * 1024 * 1024, // 1 GB, under 2 GB
            function_invocations: 50_000, // Under 100k
            ..Default::default()
        };

        let charges = calc.calculate_usage_charges(&usage, &Plan::Free);
        assert!(charges.is_empty(), "Should have no charges under free tier");
    }

    #[test]
    fn test_usage_charges() {
        let calc = BillingCalculator::new();
        let tenant_id = Uuid::new_v4();

        let usage = UsageMetrics {
            tenant_id,
            month: "2024-08".to_string(),
            api_requests: 2_010_000, // 2M above free tier
            storage_bytes: 10 * 1024 * 1024 * 1024, // 10 GB
            ..Default::default()
        };

        let charges = calc.calculate_usage_charges(&usage, &Plan::Pro);
        assert!(!charges.is_empty());

        // Check API charges
        let api_charge = charges.iter().find(|c| c.description == "API Requests");
        assert!(api_charge.is_some());
    }

    #[test]
    fn test_invoice_generation() {
        let calc = BillingCalculator::new();
        let tenant_id = Uuid::new_v4();

        let usage = UsageMetrics {
            tenant_id,
            month: "2024-08".to_string(),
            api_requests: 1_000_000,
            storage_bytes: 5 * 1024 * 1024 * 1024,
            ..Default::default()
        };

        let invoice = calc.generate_invoice(tenant_id, "2024-08", &Plan::Pro, &usage);
        assert_eq!(invoice.tenant_id, tenant_id);
        assert_eq!(invoice.month, "2024-08");
        assert_eq!(invoice.base_price, 25.0);
        assert!(invoice.total >= invoice.base_price);
    }
}
