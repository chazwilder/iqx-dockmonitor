use crate::analysis::AnalysisResult;
use reqwest::Client;
use tracing::info;
use crate::config::Settings;

/// The `AlertManager` is responsible for handling and sending alerts based on analysis results
pub struct AlertManager {
    /// The application settings, which may include alert destinations or configurations
    settings: Settings,
    /// The HTTP client used to send alert notifications
    client: Client
}

impl AlertManager {
    /// Creates a new `AlertManager`
    ///
    /// Initializes the `AlertManager` with the provided settings and creates an HTTP client for sending alerts
    /// Logs an informational message upon creation
    ///
    /// # Arguments
    ///
    /// * `settings`: The application settings
    pub fn new(settings: Settings) -> Self {
        info!("Initializing Alert Manager");
        let client = reqwest::Client::new();
        Self {
            settings,
            client
        }
    }

    /// Handles an alert based on the provided analysis result
    ///
    /// Currently, this method only logs the `alert_data` using the `tracing` crate's `info!` macro
    /// In a real-world scenario, this method would likely send the alert to the appropriate destination (e.g., email, webhook)
    /// based on the configuration in the `settings`
    ///
    /// # Arguments
    ///
    /// * `alert_data`: The `AnalysisResult` containing the details of the alert to be handled
    pub fn alert(alert_data: AnalysisResult) {
        info!("{:?}", alert_data)
    }

}