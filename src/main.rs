use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use log::{error, info};
use tokio::signal::ctrl_c;
use tokio::time::interval;
use iqx_dockmonitor::alerting::alert_manager::{Alert, AlertType};
use iqx_dockmonitor::init;
use iqx_dockmonitor::init::AppContext;

#[tokio::main]
async fn main() {
    init_logger().expect("Failed to initialize logger");
    if let Err(e) = run().await {
        eprintln!("Application error: {}", e);
        std::process::exit(1);
    }
}

fn init_logger() -> Result<(), Box<dyn Error>> {
    log4rs::init_file("src/config/log4rs.yaml", Default::default())?;
    Ok(())
}

async fn run() -> Result<()> {
    let context = Arc::new(init::initialize().await?);

    // Spawn PLC polling task
    let plc_context = Arc::clone(&context);
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(20));
        loop {
            interval.tick().await;
            info!("Starting new PLC polling cycle...");
            if let Err(e) = plc_context.dock_door_controller.run_polling_cycle().await {
                error!("Error during PLC polling cycle: {}", e);
            }
        }
    });

    // Spawn WMS event polling task
    let wms_event_context = Arc::clone(&context);
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            info!("Starting WMS event polling cycle...");
            if let Err(e) = wms_event_context.dock_door_controller.update_wms_events().await {
                error!("Error during WMS event update cycle: {}", e);
            }
        }
    });

    // Spawn WMS door status polling task
    let wms_door_context = Arc::clone(&context);
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(25));
        loop {
            interval.tick().await;
            info!("Starting WMS door status polling cycle...");
            if let Err(e) = wms_door_context.dock_door_controller.update_wms_door_status().await {
                error!("Error during WMS door status update cycle: {}", e);
            }
        }
    });

    let trailer_pattern_context = Arc::clone(&context);
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(300)); // 5 minutes
        loop {
            interval.tick().await;
            info!("Starting trailer pattern check cycle...");
            if let Err(e) = check_trailer_pattern_issues(Arc::clone(&trailer_pattern_context)).await {
                error!("Error during trailer pattern check cycle: {}", e);
            }
        }
    });

    // Hourly rack space utilization check
    let rack_space_context = Arc::clone(&context);
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(3600));
        loop {
            interval.tick().await;
            info!("Starting rack space utilization check...");
            for plant in &rack_space_context.settings.plants {
                let plant_id = &plant.plant_id;
                match rack_space_context.db_service.fetch_rack_space_counts(plant_id).await {
                    Ok((total_count, top_count)) => {
                        if total_count < 9 {
                            let alert = Alert::new(AlertType::RackSpace, "RackSpace".to_string())
                                .add_info("plant".to_string(), plant_id.clone())
                                .add_info("empty_spaces".to_string(), total_count.to_string())
                                .build();
                            if let Err(e) = rack_space_context.alert_manager.handle_alert(alert).await {
                                error!("Failed to send rack space alert: {:?}", e);
                            }
                        } else {
                            let alert = Alert::new(AlertType::RackSpace, "RackSpace".to_string())
                                .add_info("info".to_string(), "true".to_string())
                                .add_info("plant".to_string(), plant_id.clone())
                                .add_info("empty_spaces".to_string(), total_count.to_string())
                                .build();
                            if let Err(e) = rack_space_context.alert_manager.handle_alert(alert).await {
                                error!("Failed to send rack space alert: {:?}", e);
                            }
                        }
                        if top_count <= 4 {
                            let alert = Alert::new(AlertType::LowTopRackSpace, "LowTopRackSpace".to_string())
                                .add_info("plant".to_string(), plant_id.clone())
                                .add_info("top_empty_spaces".to_string(), top_count.to_string())
                                .build();
                            if let Err(e) = rack_space_context.alert_manager.handle_alert(alert).await {
                                error!("Failed to send low top rack space alert: {:?}", e);
                            }
                        } else {
                            let alert = Alert::new(AlertType::LowTopRackSpace, "LowTopRackSpace".to_string())
                                .add_info("info".to_string(), "true".to_string())
                                .add_info("plant".to_string(), plant_id.clone())
                                .add_info("empty_spaces".to_string(), top_count.to_string())
                                .build();
                            if let Err(e) = rack_space_context.alert_manager.handle_alert(alert).await {
                                error!("Failed to send rack space alert: {:?}", e);
                            }
                        }
                    },
                    Err(e) => {
                        error!("Error fetching rack space counts for plant {}: {:?}", plant_id, e);
                    }
                }
            }
        }
    });

    // Spawn EventHandler task
    let event_handler_context = Arc::clone(&context);
    tokio::spawn(async move {
        if let Err(e) = event_handler_context.event_handler.run().await {
            error!("EventHandler error: {:?}", e);
        }
    });

    // Spawn MonitoringWorker task
    let monitoring_context = Arc::clone(&context);
    tokio::spawn(async move {
        monitoring_context.monitoring_worker.run().await;
    });

    // Wait for shutdown signal
    ctrl_c().await?;
    info!("Received shutdown signal. Shutting down gracefully...");

    Ok(())
}

async fn check_trailer_pattern_issues(context: Arc<AppContext>) -> Result<(), Box<dyn Error>> {
    info!("Starting trailer pattern check...");
    match context.db_service.fetch_trailer_pattern_data().await {
        Ok(patterns) => {
            for pattern in patterns {
                if pattern.send_trl_ptrn_alert == 1 {
                    let alert = Alert::new(AlertType::TrailerPatternIssue, pattern.dock_door.clone())
                        .shipment_id(pattern.shipmentnumber.clone())
                        .add_info("load_pattern_position".to_string(), pattern.load_pattern_position.to_string())
                        .add_info("expected_pallet_count".to_string(), pattern.expected_pallet_count.to_string())
                        .build();
                    if let Err(e) = context.alert_manager.handle_alert(alert).await {
                        error!("Failed to send trailer pattern alert: {:?}", e);
                    }
                }
            }
        },
        Err(e) => {
            error!("Error fetching trailer pattern data: {:?}", e);
        }
    }
    info!("Trailer pattern check completed");
    Ok(())
}