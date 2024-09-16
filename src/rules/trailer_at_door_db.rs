use tokio::sync::Mutex;
use crate::analysis::context_analyzer::{AnalysisRule, AnalysisResult};
use crate::models::{DockDoor, DockDoorEvent};
use crate::services::db::DatabaseService;
use serde::{Deserialize, Serialize};
use crate::config::Settings;
use once_cell::sync::OnceCell;
use anyhow::Result;

static DB_SERVICE: OnceCell<Mutex<DatabaseService>> = OnceCell::new();

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TrailerAtDoorUpdateRuleConfig {
    // Add any configuration parameters here if needed
}

pub struct TrailerAtDoorUpdateRule {
    config: TrailerAtDoorUpdateRuleConfig,
}

impl TrailerAtDoorUpdateRule {
    pub fn new(config: TrailerAtDoorUpdateRuleConfig) -> Self {
        Self { config }
    }

    async fn get_db_service() -> &'static Mutex<DatabaseService> {
        DB_SERVICE.get_or_init(|| {
            let settings = Settings::new().expect("Failed to load settings");
            let db_service = tokio::task::block_in_place(|| {
                tokio::runtime::Runtime::new()
                    .expect("Failed to create runtime")
                    .block_on(async {
                        DatabaseService::new(settings).await.expect("Failed to create DatabaseService")
                    })
            });
            Mutex::new(db_service)
        })
    }

    async fn update_trailer_at_door(door_name: &str, trailer_at_door: u8) -> Result<()> {
        let query = r#"
        UPDATE [NETWORK].[RCH].[DOCK_DOOR_PLCS]
        SET TRAILER_AT_DOOR = @P1,
            UPDATE_DTTM = GETDATE()
        WHERE DOOR_NAME = @P2
    "#;

        let db_service = Self::get_db_service().await?;
        let db_service = db_service.lock().await;

        sqlx_oldapi::query(query)
            .bind(trailer_at_door as i32)  // @P1
            .bind(door_name)               // @P2
            .execute(&*db_service.local_client.pool)
            .await?;

        Ok(())
    }
}

impl AnalysisRule for TrailerAtDoorUpdateRule {
    fn apply(&self, _door: &DockDoor, event: &DockDoorEvent) -> Vec<AnalysisResult> {
        let _ = self.config.clone();
        match event {
            DockDoorEvent::SensorStateChanged(e) if e.sensor_name == "TRAILER_AT_DOOR" => {
                let trailer_at_door = e.new_value.unwrap_or(0);
                let door_name = e.dock_name.clone();

                tokio::spawn(async move {
                    if let Err(err) = Self::update_trailer_at_door(&door_name, trailer_at_door).await {
                        log::error!("Failed to update TRAILER_AT_DOOR for door {}: {:?}", door_name, err);
                    }
                });

                vec![]
            },
            _ => vec![],
        }
    }
}