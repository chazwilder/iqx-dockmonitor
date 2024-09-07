use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use log::info;
use crate::analysis::context_analyzer::AnalysisRule;
use crate::rules::rule_factory::RuleFactory;

/// Represents the configuration for a dynamically loaded analysis rule
#[derive(Debug, Serialize, Deserialize)]
pub struct RuleConfig {
    /// The type of rule to be created (e.g., "DockingTimeRule")
    pub rule_type: String,
    /// The parameters specific to the rule type, serialized as a JSON value
    pub parameters: serde_json::Value,
}

/// Manages the dynamic loading and configuration of analysis rules from a JSON file
pub struct DynamicRuleManager {
    /// The factory responsible for creating rule instances based on their configurations
    rule_factory: RuleFactory,
    /// The path to the JSON file containing the rule configurations
    config_path: PathBuf,
}

impl DynamicRuleManager {
    /// Creates a new `DynamicRuleManager`
    ///
    /// # Arguments
    ///
    /// * `config_path`: The path to the JSON file containing rule configurations
    pub fn new(config_path: PathBuf) -> Self {
        DynamicRuleManager {
            rule_factory: RuleFactory::new(),
            config_path,
        }
    }

    /// Loads analysis rules from the configuration file
    ///
    /// This method reads the JSON configuration file, parses the rule configurations, and uses the `RuleFactory` 
    /// to create instances of the specified rule types with their corresponding parameters
    /// It logs informational messages about the loading process and returns the loaded rules or an error if any occur
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Arc<dyn AnalysisRule>>)`: A vector of dynamically loaded analysis rules
    /// * `Err(anyhow::Error)`: If there's an error opening, reading, parsing the configuration file, or creating the rules
    pub fn load_rules(&self) -> Result<Vec<Arc<dyn AnalysisRule>>> {
        info!("Loading rules from config file: {:?}", self.config_path);
        let file = File::open(&self.config_path)
            .with_context(|| format!("Failed to open config file: {:?}", self.config_path))?;
        let reader = BufReader::new(file);
        let configs: Vec<RuleConfig> = serde_json::from_reader(reader)
            .with_context(|| "Failed to parse rule configurations")?;
        info!("Loaded {} rule configurations", configs.len());
        configs.into_iter()
            .map(|config| {
                info!("Creating rule: {}", config.rule_type);
                self.rule_factory.create_rule(&config.rule_type, &config.parameters)
            })
            .collect()
    }

    /// Adds a new rule configuration to the existing ones and saves them
    ///
    /// # Arguments
    /// * `rule_config`: The new `RuleConfig` to add
    ///
    /// # Returns
    /// * `Ok(())` if the rule was added and saved successfully
    /// * `Err(anyhow::Error)` if there is an error loading or saving the configurations
    pub fn add_rule(&self, rule_config: RuleConfig) -> Result<()> {
        let mut configs = self.load_rule_configs()?;
        configs.push(rule_config);
        self.save_rule_configs(&configs)
    }

    /// Loads rule configurations from the JSON file
    ///
    /// # Returns
    /// * `Ok(Vec<RuleConfig>)`: The loaded rule configurations
    /// * `Err(anyhow::Error)` if there is an error opening, reading, or parsing the file
    fn load_rule_configs(&self) -> Result<Vec<RuleConfig>> {
        let file = File::open(&self.config_path)
            .with_context(|| format!("Failed to open config file: {:?}", self.config_path))?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader)
            .with_context(|| "Failed to parse rule configurations")
    }

    /// Saves the provided rule configurations to the JSON file
    ///
    /// # Arguments
    /// * `configs`: A slice of `RuleConfig` to be saved
    ///
    /// # Returns:
    /// * `Ok(())` if the configurations were saved successfully
    /// * `Err(anyhow::Error)` if there is an error creating or writing to the file
    fn save_rule_configs(&self, configs: &[RuleConfig]) -> Result<()> {
        let file = File::create(&self.config_path)
            .with_context(|| format!("Failed to create config file: {:?}", self.config_path))?;
        serde_json::to_writer_pretty(file, configs)
            .with_context(|| "Failed to write rule configurations")
    }
}