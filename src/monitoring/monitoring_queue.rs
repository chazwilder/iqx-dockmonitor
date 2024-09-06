use std::collections::VecDeque;
use chrono::{NaiveDateTime};
use serde::{Serialize, Deserialize};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MonitoringItem {
    SuspendedShipment {
        plant_id: String,
        door_name: String,
        shipment_id: String,
        suspended_at: NaiveDateTime,
        user: String
    },
    TrailerDockedNotStarted {
        plant_id: String,
        door_name: String,
        docked_at: NaiveDateTime,
    },
    ShipmentStartedLoadNotReady {
        plant_id: String,
        door_name: String,
        shipment_id: String,
        started_at: NaiveDateTime,
    },
}


pub struct MonitoringQueue {
    queue: Mutex<VecDeque<MonitoringItem>>,
}

impl MonitoringQueue {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
        }
    }

    pub async fn add(&self, item: MonitoringItem) {
        let mut queue = self.queue.lock().await;
        queue.push_back(item);
    }

    pub async fn get(&self) -> Option<MonitoringItem> {
        let mut queue = self.queue.lock().await;
        queue.pop_front()
    }

    pub async fn len(&self) -> usize {
        let queue = self.queue.lock().await;
        queue.len()
    }
}
