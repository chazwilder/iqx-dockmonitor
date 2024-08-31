use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use tracing::{error, info};
use tokio::signal::ctrl_c;
use tokio::time::interval;
use iqx_dockmonitor::alerting::alert_manager::AlertManager;
use iqx_dockmonitor::analysis::create_default_analyzer;
use iqx_dockmonitor::config::Settings;
use iqx_dockmonitor::controllers::dock_door::DockDoorController;
use iqx_dockmonitor::rules::DynamicRuleManager;
use iqx_dockmonitor::rules::wms_shipment_status_rule::WmsShipmentStatus;
use iqx_dockmonitor::services::db::DatabaseService;
use iqx_dockmonitor::services::plc::PlcService;
use iqx_dockmonitor::state_management::DockDoorStateManager;
use iqx_dockmonitor::utils::logging;

/// The main entry point of the IQX Dock Manager application
///
/// This function initializes the application, sets up logging, creates necessary services and controllers,
/// and runs the main event loop that handles periodic polling and WMS updates
#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Application error: {}", e);
        std::process::exit(1);
    }
}


/// The core logic of the IQX Dock Monitor
///
/// This asynchronous function performs the following steps:
/// 1. Loads application settings from configuration files
/// 2. Initializes the logging system
/// 3. Creates the `PlcService`, `AlertManager`, and `DatabaseService`
/// 4. Loads dynamic rules using the `DynamicRuleManager`
/// 5. Creates a `ContextAnalyzer` and adds the loaded rules and the `WmsShipmentStatus` rule to it
/// 6. Creates the `DockDoorStateManager` and its associated `EventHandler`
/// 7. Creates the `DockDoorController`
/// 8. Sets up intervals for PLC polling, WMS event polling, and WMS door status polling
/// 9. Spawns a task to run the `EventHandler`
/// 10. Enters the main event loop, handling polling and WMS updates until a shutdown signal is received
///
/// # Returns
///
/// * `Ok(())` if the application runs successfully and shuts down gracefully
/// * `Err(anyhow::Error)` if any errors occur during initialization or the main loop
async fn run() -> Result<()> {
    let settings = Settings::new()?;
    let log_file_path = settings.logging.path.clone();
    let _guard = logging::init_logger(log_file_path)?;

    let plc_service = PlcService::new();
    let alert_manager = AlertManager::new(settings.clone());
    let db_service = DatabaseService::new(
        Arc::new(settings.clone())
    ).await?;
    let rule_manager = DynamicRuleManager::new(PathBuf::from("src/config/rules.json"));
    let rules = rule_manager.load_rules().expect("Failed to load rules");

    let mut context_analyzer = create_default_analyzer();
    for rule in rules {
        context_analyzer.add_rule(rule);
    }
    context_analyzer.add_rule(Arc::new(WmsShipmentStatus));

    let (state_manager, event_handler) = DockDoorStateManager::new(&settings, context_analyzer);
    let state_manager = Arc::new(state_manager);
    let event_handler = Arc::new(event_handler);

    let dock_door_controller = Arc::new(DockDoorController::new(
        settings.clone(),
        plc_service,
        Arc::clone(&state_manager),
        Arc::clone(&event_handler),
        db_service,
    ));

    let mut polling_interval = interval(Duration::from_secs(20));
    let mut wms_event_interval = interval(Duration::from_secs(60));
    let mut wms_door_status_interval = interval(Duration::from_secs(25));

    let event_handler_clone = Arc::clone(&event_handler);
    tokio::spawn(async move {
        if let Err(e) = event_handler_clone.run().await {
            error!("EventHandler error: {:?}", e);
        }
    });

    loop {
        tokio::select! {
            _ = polling_interval.tick() => {
                info!("Starting new PLC polling cycle...");
                if let Err(e) = dock_door_controller.run_polling_cycle().await {
                    error!("Error during polling cycle: {}", e);
                }
            }
            _ = wms_event_interval.tick() => {
                info!("Starting WMS event polling cycle...");
                if let Err(e) = dock_door_controller.update_wms_events().await {
                        error!("Error during WMS event update cycle: {}", e);
                }
            }
            _ = wms_door_status_interval.tick() => {
                info!("Starting WMS door status polling cycle...");
                if let Err(e) = dock_door_controller.update_wms_door_status().await {
                    error!("Error during WMS door status update cycle: {}", e);
                }
            }
            _ = ctrl_c() => {
                info!("Received shutdown signal. Shutting down gracefully...");
                break;
            }
        }
    }
    Ok(())
}