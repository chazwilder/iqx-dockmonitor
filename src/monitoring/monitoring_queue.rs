use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use chrono::NaiveDateTime;
use serde::{Serialize, Deserialize};
use tokio::sync::Mutex;

/// Represents different types of items that can be monitored in the dock monitoring system.
#[derive(Debug, Clone, Serialize, Deserialize, Eq)]
pub enum MonitoringItem {
    /// Represents a suspended shipment.
    SuspendedShipment {
        /// The ID of the plant where the suspension occurred.
        plant_id: String,
        /// The name of the dock door where the shipment is suspended.
        door_name: String,
        /// The ID of the suspended shipment.
        shipment_id: String,
        /// The timestamp when the shipment was suspended.
        suspended_at: NaiveDateTime,
        /// The user who suspended the shipment.
        user: String
    },
    /// Represents a trailer that has docked but loading hasn't started.
    TrailerDockedNotStarted {
        /// The ID of the plant where the trailer is docked.
        plant_id: String,
        /// The name of the dock door where the trailer is docked.
        door_name: String,
        /// The timestamp when the trailer docked.
        docked_at: NaiveDateTime,
    },
    /// Represents a shipment that has started loading but the dock is not ready.
    ShipmentStartedLoadNotReady {
        /// The ID of the plant where the shipment is located.
        plant_id: String,
        /// The name of the dock door where the shipment is located.
        door_name: String,
        /// The ID of the shipment.
        shipment_id: String,
        /// The timestamp when the shipment started loading.
        started_at: NaiveDateTime,
    },
}

impl Hash for MonitoringItem {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            MonitoringItem::SuspendedShipment { plant_id, door_name, shipment_id, .. } => {
                plant_id.hash(state);
                door_name.hash(state);
                shipment_id.hash(state);
                "SuspendedShipment".hash(state);
            },
            MonitoringItem::TrailerDockedNotStarted { plant_id, door_name, .. } => {
                plant_id.hash(state);
                door_name.hash(state);
                "TrailerDockedNotStarted".hash(state);
            },
            MonitoringItem::ShipmentStartedLoadNotReady { plant_id, door_name, shipment_id, .. } => {
                plant_id.hash(state);
                door_name.hash(state);
                shipment_id.hash(state);
                "ShipmentStartedLoadNotReady".hash(state);
            },
        }
    }
}

impl PartialEq for MonitoringItem {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MonitoringItem::SuspendedShipment { plant_id: p1, door_name: d1, shipment_id: s1, .. },
                MonitoringItem::SuspendedShipment { plant_id: p2, door_name: d2, shipment_id: s2, .. }) => {
                p1 == p2 && d1 == d2 && s1 == s2
            },
            (MonitoringItem::TrailerDockedNotStarted { plant_id: p1, door_name: d1, .. },
                MonitoringItem::TrailerDockedNotStarted { plant_id: p2, door_name: d2, .. }) => {
                p1 == p2 && d1 == d2
            },
            (MonitoringItem::ShipmentStartedLoadNotReady { plant_id: p1, door_name: d1, shipment_id: s1, .. },
                MonitoringItem::ShipmentStartedLoadNotReady { plant_id: p2, door_name: d2, shipment_id: s2, .. }) => {
                p1 == p2 && d1 == d2 && s1 == s2
            },
            _ => false,
        }
    }
}

/// A thread-safe queue for monitoring items in the dock monitoring system.
pub struct MonitoringQueue {
    /// The internal HashSet storing the monitoring items, protected by a Mutex for thread-safety.
    queue: Mutex<HashSet<MonitoringItem>>,
}

impl MonitoringQueue {
    /// Creates a new, empty `MonitoringQueue`.
    ///
    /// # Returns
    ///
    /// A new `MonitoringQueue` instance.
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(HashSet::new()),
        }
    }

    /// Adds a new item to the monitoring queue.
    ///
    /// If the item already exists in the queue (based on its hash and equality),
    /// it will not be added again.
    ///
    /// # Arguments
    ///
    /// * `item` - The `MonitoringItem` to be added to the queue.
    pub async fn add(&self, item: MonitoringItem) {
        let mut queue = self.queue.lock().await;
        queue.insert(item);
    }

    /// Removes an item from the monitoring queue.
    ///
    /// # Arguments
    ///
    /// * `item` - The `MonitoringItem` to be removed from the queue.
    ///
    /// # Returns
    ///
    /// `true` if the item was present in the queue and removed, `false` otherwise.
    pub async fn remove(&self, item: &MonitoringItem) -> bool {
        let mut queue = self.queue.lock().await;
        queue.remove(item)
    }

    /// Checks if an item is present in the monitoring queue.
    ///
    /// # Arguments
    ///
    /// * `item` - The `MonitoringItem` to check for in the queue.
    ///
    /// # Returns
    ///
    /// `true` if the item is present in the queue, `false` otherwise.
    pub async fn contains(&self, item: &MonitoringItem) -> bool {
        let queue = self.queue.lock().await;
        queue.contains(item)
    }

    /// Returns the number of items in the monitoring queue.
    ///
    /// # Returns
    ///
    /// The number of items currently in the queue.
    pub async fn len(&self) -> usize {
        let queue = self.queue.lock().await;
        queue.len()
    }

    /// Checks if the monitoring queue is empty.
    ///
    /// # Returns
    ///
    /// `true` if the queue is empty, `false` otherwise.
    pub async fn is_empty(&self) -> bool {
        let queue = self.queue.lock().await;
        queue.is_empty()
    }

    /// Removes all items from the monitoring queue.
    pub async fn clear(&self) {
        let mut queue = self.queue.lock().await;
        queue.clear();
    }

    /// Returns an iterator over the items in the monitoring queue.
    ///
    /// # Returns
    ///
    /// An iterator that yields cloned `MonitoringItem`s.
    pub async fn iter(&self) -> impl Iterator<Item = MonitoringItem> + '_ {
        let queue = self.queue.lock().await;
        queue.iter().cloned().collect::<Vec<_>>().into_iter()
    }
}