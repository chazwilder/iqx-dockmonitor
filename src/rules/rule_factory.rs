use std::sync::Arc;
use anyhow::{Result};
use serde_json::Value;
use crate::analysis::context_analyzer::AnalysisRule;
use crate::rules::{suspended_door_rule::{SuspendedDoorRule}, long_loading_start_rule::{LongLoadingStartRule}, trailer_hostage_rule::{TrailerHostageRule}, shipment_started_load_not_ready_rule::{ShipmentStartedLoadNotReadyRule}, trailer_pattern_rule::{TrailerPatternRule}, trailer_docking_rule::{TrailerDockingRule}, manual_intervention_rule::{ManualInterventionRule}, NewShipmentPreviousTrailerPresentRule, TrailerUndockingRule};
use crate::rules::consolidated_data_rule::ConsolidatedDataRule;
use crate::rules::dock_ready_rule::DockReadyRule;

/// A factory for creating analysis rules based on their configuration
#[derive(Debug, Default)]
pub struct RuleFactory;

impl RuleFactory {
    /// Creates a new `RuleFactory`
    pub fn new() -> Self {
        RuleFactory
    }

    /// Creates an analysis rule based on the provided rule type and configuration
    ///
    /// # Arguments
    ///
    /// * `rule_type`: The type of rule to create
    /// * `config`: The configuration for the rule
    ///
    /// # Returns
    ///
    /// * `Ok(Arc<dyn AnalysisRule>)`: The created analysis rule wrapped in an `Arc`
    /// * `Err(anyhow::Error)`: If the rule type is unknown or there's an error parsing the configuration
    pub fn create_rule(&self, rule_type: &str, config: &Value) -> Result<Arc<dyn AnalysisRule>> {
        match rule_type {
            "SuspendedDoorRule" => self.create_suspended_door_rule(config),
            "LongLoadingStartRule" => self.create_long_loading_start_rule(config),
            "TrailerHostageRule" => self.create_trailer_hostage_rule(config),
            "ShipmentStartedLoadNotReadyRule" => self.create_shipment_started_load_not_ready_rule(config),
            "TrailerPatternRule" => self.create_trailer_pattern_rule(config),
            "TrailerDockingRule" => self.create_trailer_docking_rule(config),
            "NewShipmentPreviousTrailerPresentRule" => self.create_new_shipment_previous_trailer_present_rule(config),
            "ManualInterventionRule" => self.create_manual_intervention_rule(config),
            "TrailerUndockingRule" => self.create_trailer_undocking_rule(config),
            "DockReadyRule" => Ok(Arc::new(DockReadyRule)),
            "ConsolidatedDataRule" => Ok(Arc::new(ConsolidatedDataRule::new())),
            _ => Err(anyhow::anyhow!("Unknown rule type: {}", rule_type)),
        }
    }

    /// Creates a `SuspendedDoorRule` based on the provided configuration
    fn create_suspended_door_rule(&self, config: &Value) -> Result<Arc<dyn AnalysisRule>> {
        Ok(Arc::new(SuspendedDoorRule::new(config.clone())))
    }

    /// Creates a `LongLoadingStartRule` based on the provided configuration
    fn create_long_loading_start_rule(&self, config: &Value) -> Result<Arc<dyn AnalysisRule>> {
        Ok(Arc::new(LongLoadingStartRule::new(config.clone())))
    }

    /// Creates a `TrailerHostageRule` based on the provided configuration
    fn create_trailer_hostage_rule(&self, config: &Value) -> Result<Arc<dyn AnalysisRule>> {
        Ok(Arc::new(TrailerHostageRule::new(config.clone())))
    }

    /// Creates a `ShipmentStartedLoadNotReadyRule` based on the provided configuration
    fn create_shipment_started_load_not_ready_rule(&self, config: &Value) -> Result<Arc<dyn AnalysisRule>> {
        Ok(Arc::new(ShipmentStartedLoadNotReadyRule::new(config.clone())))
    }

    /// Creates a `TrailerPatternRule` based on the provided configuration
    fn create_trailer_pattern_rule(&self, config: &Value) -> Result<Arc<dyn AnalysisRule>> {
        Ok(Arc::new(TrailerPatternRule::new(config.clone())))
    }

    /// Creates a `TrailerDockingRule` based on the provided configuration
    fn create_trailer_docking_rule(&self, config: &Value) -> Result<Arc<dyn AnalysisRule>> {
        Ok(Arc::new(TrailerDockingRule::new(config.clone())))
    }

    /// Creates a `NewShipmentPreviousTrailerPresentRule` based on the provided configuration
    fn create_new_shipment_previous_trailer_present_rule(&self, config: &Value) -> Result<Arc<dyn AnalysisRule>> {
        Ok(Arc::new(NewShipmentPreviousTrailerPresentRule::new(config.clone())))
    }

    /// Creates a `ManualInterventionRule` based on the provided configuration
    fn create_manual_intervention_rule(&self, config: &Value) -> Result<Arc<dyn AnalysisRule>> {
        Ok(Arc::new(ManualInterventionRule::new(config.clone())))
    }

    fn create_trailer_undocking_rule(&self, config: &Value) -> Result<Arc<dyn AnalysisRule>> {
        Ok(Arc::new(TrailerUndockingRule::new(config.clone())))
    }
}