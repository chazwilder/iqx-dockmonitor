use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use crate::alerting::alert_manager::{AlertConfig, AlertManager};
use crate::analysis::create_default_analyzer;
use crate::config::Settings;
use crate::controllers::dock_door::DockDoorController;
use crate::event_handling;
use crate::monitoring::{MonitoringQueue, MonitoringWorker};
use crate::rules::{DynamicRuleManager, WmsShipmentStatus};
use crate::services::db::DatabaseService;
use crate::services::PlcService;
use crate::state_management::DockDoorStateManager;
use crate::utils::logging;


pub struct AppContext {
    pub settings: Arc<Settings>,
    pub plc_service: PlcService,
    pub alert_manager: Arc<AlertManager>,
    pub db_service: DatabaseService,
    pub state_manager: Arc<DockDoorStateManager>,
    pub event_handler: Arc<event_handling::EventHandler>,
    pub dock_door_controller: Arc<DockDoorController>,
    pub monitoring_worker: MonitoringWorker,
}

pub async fn initialize() -> Result<AppContext> {
    let settings = Arc::new(Settings::new()?);
    let log_file_path = settings.logging.path.clone();
    let _guard = logging::init_logger(log_file_path)?;

    let plc_service = PlcService::new();
    let alert_config = AlertConfig {
        suspended_door: settings.alerts.suspended_door.clone(),
        trailer_pattern: settings.alerts.trailer_pattern.clone(),
        long_loading_start: settings.alerts.long_loading_start.clone(),
        shipment_started_load_not_ready: settings.alerts.shipment_started_load_not_ready.clone(),
        trailer_hostage: settings.alerts.trailer_hostage.clone(),
        trailer_docked: settings.alerts.trailer_docked.clone(),
        dock_ready: settings.alerts.dock_ready.clone(),
        trailer_undocked: settings.alerts.trailer_undocked.clone(),
    };

    let webhook_url = settings.plants.first()
        .map(|plant| plant.alert_webhook_url.clone())
        .ok_or_else(|| anyhow::anyhow!("No plants configured"))?;

    let alert_manager = Arc::new(AlertManager::new(
        Arc::new(alert_config),
        webhook_url
    ));

    let db_service = DatabaseService::new(Arc::clone(&settings))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create DatabaseService: {}", e))?;

    let rule_manager = DynamicRuleManager::new(PathBuf::from("src/config/rules.json"));
    let rules = rule_manager.load_rules().expect("Failed to load rules");

    let mut context_analyzer = create_default_analyzer();
    for rule in rules {
        context_analyzer.add_rule(rule);
    }
    context_analyzer.add_rule(Arc::new(WmsShipmentStatus));

    let monitoring_queue = Arc::new(MonitoringQueue::new());

    let (state_manager, event_handler) = DockDoorStateManager::new(
        &settings,
        context_analyzer,
        Arc::clone(&alert_manager),
        Arc::clone(&monitoring_queue)
    );
    let state_manager = Arc::new(state_manager);
    let event_handler = Arc::new(event_handler);

    let dock_door_controller = Arc::new(DockDoorController::new(
        (*settings).clone(),
        plc_service.clone(),
        Arc::clone(&state_manager),
        Arc::clone(&event_handler),
        db_service.clone(),
    ));

    let monitoring_worker = MonitoringWorker::new(
        Arc::clone(&monitoring_queue),
        Arc::clone(&state_manager),
        Arc::clone(&alert_manager),
        Arc::clone(&settings),
    );

    Ok(AppContext {
        settings,
        plc_service,
        alert_manager,
        db_service,
        state_manager,
        event_handler,
        dock_door_controller,
        monitoring_worker,
    })
}