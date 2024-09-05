
use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use tracing::{error, info};
use tokio::signal::ctrl_c;
use tokio::time::interval;
use iqx_dockmonitor::init;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Application error: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let context = init::initialize().await?;

    let mut polling_interval = interval(Duration::from_secs(20));
    let mut wms_event_interval = interval(Duration::from_secs(60));
    let mut wms_door_status_interval = interval(Duration::from_secs(25));

    let event_handler_clone = Arc::clone(&context.event_handler);
    tokio::spawn(async move {
        if let Err(e) = event_handler_clone.run().await {
            error!("EventHandler error: {:?}", e);
        }
    });

    let monitoring_worker_clone = context.monitoring_worker.clone();
    tokio::spawn(async move {
        monitoring_worker_clone.run().await;
    });

    loop {
        tokio::select! {
            _ = polling_interval.tick() => {
                info!("Starting new PLC polling cycle...");
                if let Err(e) = context.dock_door_controller.run_polling_cycle().await {
                    error!("Error during polling cycle: {}", e);
                }
            }
            _ = wms_event_interval.tick() => {
                info!("Starting WMS event polling cycle...");
                if let Err(e) = context.dock_door_controller.update_wms_events().await {
                        error!("Error during WMS event update cycle: {}", e);
                }
            }
            _ = wms_door_status_interval.tick() => {
                info!("Starting WMS door status polling cycle...");
                if let Err(e) = context.dock_door_controller.update_wms_door_status().await {
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