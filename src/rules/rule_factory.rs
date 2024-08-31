use std::sync::Arc;
use anyhow::{Result, Context};
use crate::analysis::context_analyzer::AnalysisRule;
use crate::rules::dynamic_rule_manager::RuleConfig;
use crate::rules::manual_intervention_rule::{ManualInterventionRule, ManualInterventionRuleConfig};
use crate::rules::new_shipment_old_trailer_rule::{NewShipmentPreviousTrailerPresentRule, NewShipmentPreviousTrailerPresentRuleConfig};
use crate::rules::trailer_docking_rule::{TrailerDockingRule, TrailerDockingRuleConfig};

/// A factory for creating analysis rules based on their configuration
#[derive(Debug, Default)]
pub struct RuleFactory;

impl RuleFactory {
    /// Creates a new `RuleFactory`
    pub fn new() -> Self {
        RuleFactory
    }

    /// Creates an analysis rule based on the provided `RuleConfig`
    ///
    /// This method matches the `rule_type` in the `config` and calls the corresponding 
    /// rule creation method. If the `rule_type` is unknown, it returns an error
    ///
    /// # Arguments
    ///
    /// * `config`: The `RuleConfig` containing the rule type and parameters
    ///
    /// # Returns
    ///
    /// * `Ok(Arc<dyn AnalysisRule>)`: The created analysis rule wrapped in an `Arc`
    /// * `Err(anyhow::Error)`: If the rule type is unknown or there's an error parsing the parameters
    pub fn create_rule(&self, config: &RuleConfig) -> Result<Arc<dyn AnalysisRule>> {
        match config.rule_type.as_str() {
            "DockingTimeRule" => self.create_docking_time_rule(&config.parameters),
            "NewShipmentPreviousTrailerPresentRule" => self.create_new_shipment_previous_trailer_present_rule(&config.parameters),
            "ManualInterventionRule" => self.create_manual_intervention_rule(&config.parameters),
            _ => Err(anyhow::anyhow!("Unknown rule type: {}", config.rule_type)),
        }
    }
    
    /// Creates a `TrailerDockingRule` based on the provided parameters
    ///
    /// # Arguments
    ///
    /// * `parameters`: The JSON value containing the rule's parameters
    ///
    /// # Returns
    ///
    /// * `Ok(Arc<dyn AnalysisRule>)`: The created `TrailerDockingRule` wrapped in an `Arc`
    /// * `Err(anyhow::Error)`: If there's an error parsing the parameters
    fn create_docking_time_rule(&self, parameters: &serde_json::Value) -> Result<Arc<dyn AnalysisRule>> {
        let config: TrailerDockingRuleConfig = serde_json::from_value(parameters.clone())
            .context("Failed to parse DockingTimeRule parameters")?;
        Ok(Arc::new(TrailerDockingRule::new(config)))
    }

    /// Creates a `NewShipmentPreviousTrailerPresentRule` based on the provided parameters
    ///
    /// # Arguments
    ///
    /// * `parameters`: The JSON value containing the rule's parameters
    ///
    /// # Returns
    ///
    /// * `Ok(Arc<dyn AnalysisRule>)`: The created `NewShipmentPreviousTrailerPresentRule` wrapped in an `Arc`
    /// * `Err(anyhow::Error)`: If there's an error parsing the parameters
    fn create_new_shipment_previous_trailer_present_rule(&self, parameters: &serde_json::Value) -> Result<Arc<dyn AnalysisRule>> {
        let config: NewShipmentPreviousTrailerPresentRuleConfig = serde_json::from_value(parameters.clone())
            .context("Failed to parse NewShipmentPreviousTrailerPresentRule parameters")?;
        Ok(Arc::new(NewShipmentPreviousTrailerPresentRule::new(config)))
    }

    /// Creates a `ManualInterventionRule` based on the provided parameters
    ///
    /// # Arguments
    ///
    /// * `parameters`: The JSON value containing the rule's parameters
    ///
    /// # Returns
    ///
    /// * `Ok(Arc<dyn AnalysisRule>)`: The created `ManualInterventionRule` wrapped in an `Arc`
    /// * `Err(anyhow::Error)`: If there's an error parsing the parameters
    fn create_manual_intervention_rule(&self, parameters: &serde_json::Value) -> Result<Arc<dyn AnalysisRule>> {
        let config: ManualInterventionRuleConfig = serde_json::from_value(parameters.clone())
            .context("Failed to parse ManualInterventionRule parameters")?;
        Ok(Arc::new(ManualInterventionRule::new(config)))
    }
}