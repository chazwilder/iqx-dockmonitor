use dashmap::DashSet;
use std::sync::Arc;
use chrono::NaiveDateTime;
use serde::{Serialize, Deserialize};

/// Represents different types of items that can be monitored in the dock monitoring system.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
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

/// A thread-safe queue for monitoring items in the dock monitoring system.
pub struct MonitoringQueue {
    /// The internal DashSet storing the monitoring items.
    queue: Arc<DashSet<MonitoringItem>>,
}

impl MonitoringQueue {
    /// Creates a new, empty `MonitoringQueue`.
    ///
    /// # Returns
    ///
    /// A new `MonitoringQueue` instance.
    pub fn new() -> Self {
        Self {
            queue: Arc::new(DashSet::new()),
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
    pub fn add(&self, item: MonitoringItem) {
        self.queue.insert(item);
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
    pub fn remove(&self, item: &MonitoringItem) -> bool {
        self.queue.remove(item).is_some()
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
    pub fn contains(&self, item: &MonitoringItem) -> bool {
        self.queue.contains(item)
    }

    /// Returns the number of items in the monitoring queue.
    ///
    /// # Returns
    ///
    /// The number of items currently in the queue.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Checks if the monitoring queue is empty.
    ///
    /// # Returns
    ///
    /// `true` if the queue is empty, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Removes all items from the monitoring queue.
    pub fn clear(&self) {
        self.queue.clear();
    }

    /// Returns an iterator over the items in the monitoring queue.
    ///
    /// # Returns
    ///
    /// An iterator that yields references to `MonitoringItem`s.
    pub fn iter(&self) -> impl Iterator<Item = dashmap::setref::multiple::RefMulti<'_, MonitoringItem>> {
        self.queue.iter()
    }
}

impl Default for MonitoringQueue {
    fn default() -> Self {
        Self::new()
    }
}