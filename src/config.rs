//! # Configuration Management

//! This module handles the configuration loading and management for the IQX Dock Manager application. 
//! It leverages the `config` crate to provide a flexible and structured way to define and access configuration settings from various sources, including:

//! * YAML configuration files (default.yaml, development.yaml, production.yaml, queries.yaml, dock_doors.yaml)
//! * Environment variables

//! The core of this module is the `Settings` struct, which encapsulates all the configuration settings required by the application.

use serde::{Deserialize, Serialize};
use config::{Config, Environment, File};
use std::{env, fmt};
use std::path::PathBuf;
use secrecy::{Secret, ExposeSecret};
use log::{debug};
use url::Url;
use crate::errors::DockManagerError;

/// Represents the complete set of configuration settings for the IQX Dock Manager.
/// It's populated by reading from various configuration sources and provides convenient access to the settings throughout the application.
#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    /// Settings for connecting to the local database
    pub database: DatabaseSettings,
    /// Settings related to PLC (Programmable Logic Controller) communication
    pub plc: PlcSettings,
    /// Settings for application logging
    pub logging: LoggingSettings,
    /// Settings for connecting to the RabbitMQ message broker
    pub rabbitmq: RabbitMQSettings,
    /// SQL queries used to fetch data from the WMS database
    pub queries: Queries,
    /// Configuration settings for each plant
    pub plants: Vec<PlantSettings>,
    pub alerts: AlertSettings,
    pub monitoring: MonitoringSettings,
    pub batch_size: usize
}

/// Represents the configuration settings for a specific plant
#[derive(Debug, Deserialize, Clone)]
pub struct PlantSettings {
    /// The unique identifier for the plant
    pub plant_id: String,
    /// The webhook URL for sending alerts related to this plant
    pub alert_webhook_url: String,
    /// Settings for connecting to the LGV WMS database for this plant
    pub lgv_wms_database: LgvWmsDatabaseSettings,
    /// Configuration for dock doors and their associated PLC tags at this plant
    pub dock_doors: DockDoorSettings,
}

/// # Database Settings

/// This struct holds the configuration settings required to establish a connection to the local database
#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseSettings {
    /// The hostname or IP address of the database server
    pub host: String,
    /// The port number on which the database server is listening
    pub port: u16,
    /// The username for database authentication (optional if using Windows authentication)
    pub username: Option<String>,
    /// The password for database authentication (optional if using Windows authentication)
    #[serde(deserialize_with = "deserialize_optional_secret")]
    pub password: Option<Secret<String>>,
    /// The name of the database to connect to
    pub database_name: String,
    /// The application name to be used in the connection string
    pub app_name: String,
    /// Whether to use Windows authentication (true) or SQL Server authentication (false)
    pub win_auth: bool,
    /// Whether to trust the server certificate (relevant for encrypted connections)
    pub trusted: bool,
}

impl DatabaseSettings {
    /// Constructs a connection string for the local database based on the settings
    ///
    /// This method dynamically builds the connection string, handling both Windows authentication and SQL Server authentication scenarios
    ///
    /// # Returns
    ///
    /// A `Secret<String>` containing the constructed connection string. The connection string is kept secret for security reasons
    pub fn connection_string(&self) -> Secret<String> {
        if self.username.is_none() | self.password.is_none() && self.win_auth {
            let connection_string = format!(
                "mssql://{}:{}/{}",
                self.host,
                self.port,
                self.database_name
            );
            Secret::new(connection_string)
        } else {
            let connection_string = format!(
                "mssql://{}:{}@{}:{}/{}",
                self.username.clone().unwrap(),
                self.password.clone().unwrap().expose_secret(),
                self.host,
                self.port,
                self.database_name
            );
            Secret::new(connection_string)
        }
    }
}

/// # LGV WMS Database Settings

/// This struct holds the configuration settings required to establish a connection to the LGV WMS database
#[derive(Debug, Deserialize, Clone)]
pub struct LgvWmsDatabaseSettings {
    /// The hostname or IP address of the LGV WMS database server
    pub host: String,
    /// The port number on which the LGV WMS database server is listening
    pub port: u16,
    /// The username for LGV WMS database authentication (optional if using Windows authentication)
    pub username: Option<String>,
    /// The password for LGV WMS database authentication (optional if using Windows authentication)
    #[serde(deserialize_with = "deserialize_optional_secret")]
    pub password: Option<Secret<String>>,
    /// The name of the LGV WMS database to connect to
    pub database_name: String,
    /// The application name to be used in the connection string for the LGV WMS database
    pub app_name: String,
    /// Whether to use Windows authentication (true) or SQL Server authentication (false) for the LGV WMS database
    pub win_auth: bool,
    /// Whether to trust the server certificate for the LGV WMS database (relevant for encrypted connections)
    pub trusted: bool,
}


impl LgvWmsDatabaseSettings {
    /// Constructs a connection string for the local database based on the settings
    ///
    /// This method dynamically builds the connection string, handling both Windows authentication and SQL Server authentication scenarios
    ///
    /// # Returns
    ///
    /// A `Secret<String>` containing the constructed connection string. The connection string is kept secret for security reasons
    pub fn connection_string(&self) -> Secret<String> {
        if self.username.is_none() | self.password.is_none() && self.win_auth {
            let connection_string = format!(
                "mssql://{}:{}/{}",
                self.host,
                self.port,
                self.database_name
            );
            Secret::new(connection_string)
        } else {
            let connection_string = format!(
                "mssql://{}:{}@{}:{}/{}",
                self.username.clone().unwrap(),
                self.password.clone().unwrap().expose_secret(),
                self.host,
                self.port,
                self.database_name
            );
            Secret::new(connection_string)
        }
    }
}

/// Holds the SQL queries used to fetch data from the WMS database.
#[derive(Debug, Deserialize, Clone)]
pub struct Queries {
    /// The SQL query to retrieve the current status of dock doors from the WMS
    pub wms_door_status: String,
    /// The SQL query to retrieve events related to shipments from the WMS
    pub wms_events: String,
    pub wms_rack_space: String
}

/// Holds the configuration settings related to PLC (Programmable Logic Controller) communication
#[derive(Debug, Deserialize, Clone)]
pub struct PlcSettings {
    /// The interval (in seconds) at which the PLC will be polled for sensor data
    pub poll_interval_secs: u64,
    /// The timeout (in milliseconds) for PLC communication operations
    pub timeout_ms: u64,
    /// The maximum number of retries allowed for failed PLC communication attempts
    pub max_retries: u64
}

/// Holds the configuration settings for application logging
#[derive(Debug, Deserialize, Clone)]
pub struct LoggingSettings {
    /// The logging level (e.g., "info", "debug", "error")
    pub level: String,
    /// The name of the log file (optional)
    pub file: Option<String>,
    /// The directory path where log files will be stored (optional)
    pub path: Option<PathBuf>,
}

/// Holds the configuration settings required to establish a connection to the RabbitMQ message broker.
#[derive(Debug, Deserialize, Clone)]
pub struct RabbitMQSettings {
    /// The hostname or IP address of the RabbitMQ server
    pub host: String,
    /// The port number on which the RabbitMQ server is listening
    pub port: u16,
    /// The username for RabbitMQ authentication
    pub username: String,
    /// The password for RabbitMQ authentication
    #[serde(deserialize_with = "deserialize_optional_secret")]
    pub password: Option<Secret<String>>,
    /// The name of the RabbitMQ exchange to use
    pub exchange: String,
    /// The virtual host to connect to on the RabbitMQ server
    pub vhost: String,
}

impl RabbitMQSettings {
    /// Constructs a connection string for RabbitMQ based on the settings.
    ///
    /// # Returns
    ///
    /// A `Secret<String>` containing the constructed connection string. The connection string is kept secret for security reasons.
    pub fn connection_string(&self) -> Secret<String> {
        let mut url = Url::parse(&format!("amqp://{}:{}", self.host, self.port))
            .expect("Failed to parse RabbitMQ URL");

        url.set_username(&self.username)
            .expect("Failed to set RabbitMQ username");
        if let Some(password) = &self.password {
            url.set_password(Some(password.expose_secret()))
                .expect("Failed to set RabbitMQ password");
        }
        url.set_path(&self.vhost);

        Secret::new(url.to_string())
    }
}


// Holds the configuration for dock doors and their associated PLC tags.
#[derive(Debug, Deserialize, Clone)]
pub struct DockDoorSettings {
    /// Configuration details for each individual dock door
    pub dock_door_config: Vec<DockDoorConfig>,
    /// Configuration for the PLC tags associated with the dock doors
    pub dock_plc_tags: Vec<DockPlcTag>,
}

/// Represents the configuration for a single dock door
#[derive(Debug, Deserialize, Clone)]
pub struct DockDoorConfig {
    /// The name or identifier of the dock door
    pub dock_name: String,
    /// The IP address of the PLC controlling the dock door
    pub dock_ip: String,
}

/// Represents the configuration of a PLC tag associated with a dock door
#[derive(Debug, Deserialize, Clone)]
pub struct DockPlcTag {
    /// The name of the PLC tag (e.g., "RH_DOOR_OPEN")
    pub tag_name: String,
    /// The address of the PLC tag in the PLC's memory (e.g., "B9:0/9")
    pub address: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AlertSettings {
    pub suspended_door: AlertThresholds,
    pub trailer_pattern: AlertThresholds,
    pub long_loading_start: AlertThresholds,
    pub shipment_started_load_not_ready: AlertThresholds,
    pub trailer_hostage: AlertThresholds,
    pub trailer_docked: AlertThresholds,
    pub dock_ready: AlertThresholds,
    pub trailer_undocked: AlertThresholds,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AlertThresholds {
    pub initial_threshold: u64,  // in seconds
    pub repeat_interval: u64,    // in seconds
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MonitoringSettings {
    pub check_interval: u64,  // in seconds
    pub suspended_shipment: MonitoringThresholds,
    pub trailer_docked_not_started: MonitoringThresholds,
    pub shipment_started_load_not_ready: MonitoringThresholds,
    pub trailer_hostage: MonitoringThresholds,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MonitoringThresholds {
    pub alert_threshold: u64,  // in seconds
    pub repeat_interval: u64,  // in seconds
}


/// # Settings Initialization
///
/// The `Settings` implementation provides a `new` function to load and construct the configuration settings.
impl Settings {
    /// Loads and constructs the application settings from various configuration sources.
    ///
    /// This function reads configuration settings from the following sources, in order of precedence:
    ///
    /// 1. `default.yaml`: Contains default settings for the application
    /// 2. `queries.yaml`: Contains SQL queries used to fetch data from the WMS database (required)
    /// 3. `dock_doors.yaml`: Contains the configuration of dock doors and their associated PLC tags
    /// 4. Environment-specific YAML file (e.g., `development.yaml` or `production.yaml`) based on the `RUN_MODE` environment variable
    /// 5. Environment variables prefixed with `APP` (e.g., `APP__DATABASE__HOST`)
    ///
    /// The `CONFIG_DIR` environment variable can be used to specify the directory where the YAML configuration files are located (defaults to "src/config").
    ///
    /// # Returns
    ///
    /// * `Ok(Settings)`: If the settings were loaded and constructed successfully
    /// * `Err(DockManagerError)`: If there was an error during the loading or construction process
    pub fn new() -> Result<Self, DockManagerError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());
        let config_dir = env::var("CONFIG_DIR").unwrap_or_else(|_| "src/config".into());
        debug!("Run Mode: {:?}, Config Dir: {:?}", run_mode, config_dir);

        let s = Config::builder()
            .add_source(File::with_name(&format!("{}/default", config_dir)))
            .add_source(File::with_name(&format!("{}/{}", config_dir, run_mode)).required(false))
            .add_source(Environment::with_prefix("APP").separator("__"))
            .build()?;

        debug!("{:#?}", s);
        let mut s: Self = s.try_deserialize::<Settings>()
            .map_err(DockManagerError::from)?;

        if let Some(ref mut path) = s.logging.path {
            *path = env::current_dir()?.join(path.clone());
        }

        s.batch_size = 1;

        Ok(s)
    }

    pub fn get_plant(&self, plant_id: &str) -> Option<&PlantSettings> {
        self.plants.iter().find(|plant| plant.plant_id == plant_id)
    }
}

/// Helper struct for deserializing secret strings from configuration
#[derive(Debug,Clone, Deserialize)]
struct SecretString(Option<String>);

impl From<SecretString> for Secret<Option<String>> {
    fn from(secret: SecretString) -> Self {
        Secret::new(secret.0)
    }
}

/// Deserializes a secret string from configuration into a `Secret<String>`
fn deserialize_optional_secret<'de, D>(deserializer: D) -> Result<Option<Secret<String>>, D::Error>
    where
        D: serde::Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    Ok(opt.map(Secret::new))
}

impl fmt::Display for DatabaseSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DatabaseSettings {{ host: {}, port: {}, username: {:?}, database_name: {}, app_name: {}, win_auth: {}, trusted: {} }}",
            self.host, self.port, self.username, self.database_name, self.app_name, self.win_auth, self.trusted
        )
    }
}

impl fmt::Display for LgvWmsDatabaseSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LgvWmsDatabaseSettings {{ host: {}, port: {}, username: {:?}, database_name: {}, app_name: {}, win_auth: {}, trusted: {} }}",
            self.host, self.port, self.username, self.database_name, self.app_name, self.win_auth, self.trusted
        )
    }
}