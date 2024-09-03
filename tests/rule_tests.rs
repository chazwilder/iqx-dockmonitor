use chrono::{Duration, Local};
use std::collections::HashMap;
use std::sync::Arc;

use iqx_dockmonitor::analysis::context_analyzer::{AnalysisResult, AlertType};
use iqx_dockmonitor::analysis::{AnalysisRule, LogEntry};
use iqx_dockmonitor::config::{AlertSettings, AlertThresholds, DatabaseSettings, LoggingSettings, MonitoringSettings, MonitoringThresholds, PlcSettings, Queries, RabbitMQSettings, Settings};
use iqx_dockmonitor::models::{AssignedShipment, DockDoor, DockDoorEvent, DockLockState, DockSensor, DoorPosition, DoorState, FaultState, LevelerPosition, LoadingStatus, LoadingStatusChangedEvent, ManualMode, SensorStateChangedEvent, ShipmentAssignedEvent, TrailerState, TrailerStateChangedEvent, WmsEventWrapper};
use iqx_dockmonitor::rules::{
    SuspendedDoorRule, TrailerHostageRule,
    ShipmentStartedLoadNotReadyRule, TrailerPatternRule, TrailerDockingRule,
    NewShipmentPreviousTrailerPresentRule, TrailerUndockingRule,
};
use iqx_dockmonitor::rules::long_loading_start_rule::LongLoadingStartRule;
use iqx_dockmonitor::rules::manual_intervention_rule::ManualInterventionRule;

fn create_mock_settings() -> Settings {
    Settings {
        database: DatabaseSettings {
            host: "localhost".to_string(),
            port: 5432,
            username: Some("user".to_string()),
            password: None,
            database_name: "test_db".to_string(),
            app_name: "test_app".to_string(),
            win_auth: false,
            trusted: true,
        },
        plc: PlcSettings {
            poll_interval_secs: 10,
            timeout_ms: 5000,
            max_retries: 3,
        },
        logging: LoggingSettings {
            level: "info".to_string(),
            file: None,
            path: None,
        },
        rabbitmq: RabbitMQSettings {
            host: "localhost".to_string(),
            port: 5672,
            username: "guest".to_string(),
            password: None,
            exchange: "test_exchange".to_string(),
            vhost: "/".to_string(),
        },
        queries: Queries {
            wms_door_status: "SELECT * FROM wms_door_status".to_string(),
            wms_events: "SELECT * FROM wms_events".to_string(),
        },
        plants: vec![],
        alerts: AlertSettings {
            suspended_door: AlertThresholds { initial_threshold: 300, repeat_interval: 600 },
            trailer_pattern: AlertThresholds { initial_threshold: 0, repeat_interval: 300 },
            long_loading_start: AlertThresholds { initial_threshold: 600, repeat_interval: 300 },
            shipment_started_load_not_ready: AlertThresholds { initial_threshold: 60, repeat_interval: 300 },
            trailer_hostage: AlertThresholds { initial_threshold: 300, repeat_interval: 300 },
            trailer_docked: AlertThresholds { initial_threshold: 0, repeat_interval: 300 },
            dock_ready: AlertThresholds { initial_threshold: 0, repeat_interval: 300 },
        },
        monitoring: MonitoringSettings {
            check_interval: 60,
            suspended_shipment: MonitoringThresholds { alert_threshold: 300, repeat_interval: 300 },
            trailer_docked_not_started: MonitoringThresholds { alert_threshold: 900, repeat_interval: 600 },
            shipment_started_load_not_ready: MonitoringThresholds { alert_threshold: 300, repeat_interval: 300 },
        },
    }
}

fn create_mock_door() -> DockDoor {
    DockDoor {
        plant_id: "TEST_PLANT".to_string(),
        dock_name: "TEST_DOOR".to_string(),
        dock_ip: "127.0.0.1".to_string(),
        manual_intervention_count: 0,
        loading_status: LoadingStatus::Idle,
        wms_shipment_status: None,
        previous_loading_status: LoadingStatus::Idle,
        door_state: DoorState::Unassigned,
        previous_door_state: DoorState::Unassigned,
        trailer_state: TrailerState::Undocked,
        previous_trailer_state: TrailerState::Undocked,
        trailer_state_changed: None,
        manual_mode: ManualMode::Disabled,
        dock_lock_state: DockLockState::Disengaged,
        door_position: DoorPosition::Closed,
        leveler_position: LevelerPosition::Stored,
        fault_state: FaultState::NoFault,
        assigned_shipment: AssignedShipment::default(),
        sensors: HashMap::new(),
        last_updated: Local::now().naive_local(),
    }
}


#[test]
fn test_long_loading_start_rule() {
    let config = serde_json::json!({
        "alert_threshold": 3600,
        "repeat_interval": 1800
    });
    let rule = Arc::new(LongLoadingStartRule::new(config));
    let mut door = create_mock_door();
    let now = Local::now().naive_local();

    // Test case 1: Loading just started, should not trigger alert
    let event = DockDoorEvent::LoadingStatusChanged(LoadingStatusChangedEvent {
        dock_name: door.dock_name.clone(),
        old_status: LoadingStatus::Idle,
        new_status: LoadingStatus::Loading,
        timestamp: now,
    });
    door.loading_status = LoadingStatus::Loading;
    let results = rule.apply(&door, &event);
    assert!(results.is_empty(), "No alert should be generated when loading just started");

    // Test case 2: Loading started long ago, should trigger alert
    let long_ago = now - Duration::hours(2);
    let event = DockDoorEvent::WmsEvent(WmsEventWrapper {
        dock_name: door.dock_name.clone(),
        shipment_id: "TEST_SHIPMENT".to_string(),
        event_type: "STARTED_SHIPMENT".to_string(),
        timestamp: long_ago,
        message_source: "WMS".to_string(),
        message_notes: None,
        result_code: 0,
    });
    let results = rule.apply(&door, &event);
    assert_eq!(results.len(), 1, "One alert should be generated for long loading start");
    match &results[0] {
        AnalysisResult::Alert(AlertType::LongLoadingStart { door_name, shipment_id, duration }) => {
            assert_eq!(door_name, &door.dock_name);
            assert_eq!(shipment_id, "TEST_SHIPMENT");
            assert!(duration.num_seconds() > 3600);
        },
        _ => panic!("Unexpected analysis result"),
    }
}

#[test]
fn test_suspended_door_rule() {
    let config = serde_json::json!({
        "alert_threshold": 1800,
        "repeat_interval": 900
    });
    let rule = Arc::new(SuspendedDoorRule::new(config));
    let mut door = create_mock_door();
    let now = Local::now().naive_local();

    // Test case 1: Door just suspended, should not trigger alert
    let event = DockDoorEvent::LoadingStatusChanged(LoadingStatusChangedEvent {
        dock_name: door.dock_name.clone(),
        old_status: LoadingStatus::Loading,
        new_status: LoadingStatus::Suspended,
        timestamp: now,
    });
    door.loading_status = LoadingStatus::Suspended;
    let results = rule.apply(&door, &event);
    assert!(results.is_empty(), "No alert should be generated when door just suspended");

    // Test case 2: Door suspended long ago, should trigger alert
    let long_ago = now - Duration::minutes(40);
    let event = DockDoorEvent::WmsEvent(WmsEventWrapper {
        dock_name: door.dock_name.clone(),
        shipment_id: "TEST_SHIPMENT".to_string(),
        event_type: "SUSPENDED_SHIPMENT".to_string(),
        timestamp: long_ago,
        message_source: "WMS".to_string(),
        message_notes: None,
        result_code: 0,
    });
    door.assigned_shipment.current_shipment = Some("TEST_SHIPMENT".to_string());
    let results = rule.apply(&door, &event);
    assert_eq!(results.len(), 1, "One alert should be generated for long suspended door");
    match &results[0] {
        AnalysisResult::Alert(AlertType::SuspendedDoor { door_name, duration, shipment_id }) => {
            assert_eq!(door_name, &door.dock_name);
            assert_eq!(shipment_id.as_ref().unwrap(), "TEST_SHIPMENT");
            assert!(duration.num_seconds() > 1800);
        },
        _ => panic!("Unexpected analysis result"),
    }

    let results = rule.apply(&door, &event);
    assert_eq!(results.len(), 2, "Two results should be generated: an alert and a log entry");

    let alert = results.iter().find(|r| matches!(r, AnalysisResult::Alert(_))).unwrap();
    let log = results.iter().find(|r| matches!(r, AnalysisResult::Log(_))).unwrap();

    match alert {
        AnalysisResult::Alert(AlertType::SuspendedDoor { door_name, duration, shipment_id }) => {
            assert_eq!(door_name, &door.dock_name);
            assert_eq!(shipment_id.as_ref().unwrap(), "TEST_SHIPMENT");
            assert!(duration.num_seconds() > 1800);
        },
        _ => panic!("Unexpected alert type"),
    }

    match log {
        AnalysisResult::Log(LogEntry::SuspendedDoor { door_name, shipment_id, event_type, severity, .. }) => {
            assert_eq!(door_name, &door.dock_name);
            assert_eq!(shipment_id.as_ref().unwrap(), "TEST_SHIPMENT");
            assert_eq!(event_type, "SUSPENDED_DOOR");
            assert_eq!(*severity, 2); // Assuming severity 2 for suspended door, adjust if different
        },
        _ => panic!("Unexpected log entry type"),
    }
}



#[test]
fn test_trailer_hostage_rule() {
    let config = serde_json::json!({
        "alert_threshold": 3600,
        "repeat_interval": 1800
    });
    let rule = Arc::new(TrailerHostageRule::new(config));
    let mut door = create_mock_door();
    let now = Local::now().naive_local();

    // Set up hostage situation
    door.loading_status = LoadingStatus::Completed;
    door.trailer_state = TrailerState::Docked;
    door.manual_mode = ManualMode::Enabled;
    door.trailer_state_changed = Some(now - Duration::hours(2));
    door.assigned_shipment.current_shipment = Some("TEST_SHIPMENT".to_string());

    let event = DockDoorEvent::SensorStateChanged(SensorStateChangedEvent {
        dock_name: door.dock_name.clone(),
        sensor_name: "RH_MANUAL_MODE".to_string(),
        old_value: Some(0),
        new_value: Some(1),
        timestamp: now,
    });

    let results = rule.apply(&door, &event);
    assert_eq!(results.len(), 1, "One alert should be generated for trailer hostage");
    match &results[0] {
        AnalysisResult::Alert(AlertType::TrailerHostage { door_name, shipment_id, duration }) => {
            assert_eq!(door_name, &door.dock_name);
            assert_eq!(shipment_id.as_ref().unwrap(), "TEST_SHIPMENT");
            assert!(duration.num_seconds() > 3600);
        },
        _ => panic!("Unexpected analysis result"),
    }
}

#[test]
fn test_shipment_started_load_not_ready_rule() {
    let config = serde_json::json!({
        "check_restraint": true,
        "check_leveler": true,
        "check_door_open": true
    });
    let rule = Arc::new(ShipmentStartedLoadNotReadyRule::new(config));
    let mut door = create_mock_door();
    let now = Local::now().naive_local();

    // Set up not ready situation
    door.dock_lock_state = DockLockState::Disengaged;
    door.leveler_position = LevelerPosition::Stored;
    door.door_position = DoorPosition::Closed;

    let event = DockDoorEvent::WmsEvent(WmsEventWrapper {
        dock_name: door.dock_name.clone(),
        shipment_id: "TEST_SHIPMENT".to_string(),
        event_type: "STARTED_SHIPMENT".to_string(),
        timestamp: now,
        message_source: "WMS".to_string(),
        message_notes: None,
        result_code: 0,
    });

    let results = rule.apply(&door, &event);
    assert_eq!(results.len(), 1, "One alert should be generated for shipment started load not ready");
    match &results[0] {
        AnalysisResult::Alert(AlertType::ShipmentStartedLoadNotReady { door_name, shipment_id, reason }) => {
            assert_eq!(door_name, &door.dock_name);
            assert_eq!(shipment_id, "TEST_SHIPMENT");
            assert!(reason.contains("Dock restraint is not engaged"));
            assert!(reason.contains("Dock leveler is not extended"));
            assert!(reason.contains("Dock door is not open"));
        },
        _ => panic!("Unexpected analysis result"),
    }
}

#[test]
fn test_trailer_pattern_rule() {
    let config = serde_json::json!({
        "severity_threshold": 2
    });
    let rule = Arc::new(TrailerPatternRule::new(config));
    let door = create_mock_door();
    let now = Local::now().naive_local();

    let event = DockDoorEvent::WmsEvent(WmsEventWrapper {
        dock_name: door.dock_name.clone(),
        shipment_id: "TEST_SHIPMENT".to_string(),
        event_type: "TRL_PTN".to_string(),
        timestamp: now,
        message_source: "WMS".to_string(),
        message_notes: Some("3".to_string()),
        result_code: 0,
    });

    let results = rule.apply(&door, &event);
    assert_eq!(results.len(), 1, "One alert should be generated for trailer pattern issue");
    match &results[0] {
        AnalysisResult::Alert(AlertType::TrailerPatternIssue { door_name, issue, severity, shipment_id }) => {
            assert_eq!(door_name, &door.dock_name);
            assert!(issue.contains("Trailer pattern issue detected: 3"));
            assert_eq!(*severity, 3);
            assert_eq!(shipment_id.as_ref().unwrap(), "TEST_SHIPMENT");
        },
        _ => panic!("Unexpected analysis result"),
    }

    let results = rule.apply(&door, &event);
    assert_eq!(results.len(), 2, "Two results should be generated: an alert and a log entry");

    let alert = results.iter().find(|r| matches!(r, AnalysisResult::Alert(_))).unwrap();
    let log = results.iter().find(|r| matches!(r, AnalysisResult::Log(_))).unwrap();

    match alert {
        AnalysisResult::Alert(AlertType::TrailerPatternIssue { door_name, issue, severity, shipment_id }) => {
            assert_eq!(door_name, &door.dock_name);
            assert!(issue.contains("Trailer pattern issue detected: 3"));
            assert_eq!(*severity, 3);
            assert_eq!(shipment_id.as_ref().unwrap(), "TEST_SHIPMENT");
        },
        _ => panic!("Unexpected alert type"),
    }

    match log {
        AnalysisResult::Log(LogEntry::TrailerPatternIssue { door_name, shipment_id, event_type, severity, .. }) => {
            assert_eq!(door_name, &door.dock_name);
            assert_eq!(shipment_id.as_ref().unwrap(), "TEST_SHIPMENT");
            assert_eq!(event_type, "TRAILER_PATTERN_ISSUE");
            assert_eq!(*severity, 3);
        },
        _ => panic!("Unexpected log entry type"),
    }
}


#[test]
fn test_trailer_docking_rule() {
    let config = serde_json::json!({
        "sensors_to_monitor": [
            {"name": "TRAILER_AT_DOOR", "success_value": 1},
            {"name": "TRAILER_ANGLE", "success_value": 0},
            {"name": "TRAILER_CENTERING", "success_value": 0},
            {"name": "TRAILER_DISTANCE", "success_value": 0}
        ],
        "fields_to_monitor": ["loading_status", "wms_shipment_status"],
        "successful_dock_conditions": {
            "loading_status": ["CSO", "WhseInspection"],
            "wms_shipment_status": ["WaitingInfo", "NewOrder", "WaitQtyCheck", "WaitDockCnfrm", "Scheduled"]
        }
    });
    let rule = Arc::new(TrailerDockingRule::new(config));
    let mut door = create_mock_door();
    let now = Local::now().naive_local();

    // Set up successful docking conditions
    door.loading_status = LoadingStatus::CSO;
    door.wms_shipment_status = Some("WaitingInfo".to_string());
    door.assigned_shipment.current_shipment = Some("TEST_SHIPMENT".to_string());
    door.sensors.insert("TRAILER_AT_DOOR".to_string(), DockSensor::new("TEST_DOOR", "127.0.0.1", "TRAILER_AT_DOOR", "B9:1/5"));
    door.sensors.insert("TRAILER_ANGLE".to_string(), DockSensor::new("TEST_DOOR", "127.0.0.1", "TRAILER_ANGLE", "B9:1/6"));
    door.sensors.insert("TRAILER_CENTERING".to_string(), DockSensor::new("TEST_DOOR", "127.0.0.1", "TRAILER_CENTERING", "B9:1/7"));
    door.sensors.insert("TRAILER_DISTANCE".to_string(), DockSensor::new("TEST_DOOR", "127.0.0.1", "TRAILER_DISTANCE", "B9:1/8"));

    door.sensors.get_mut("TRAILER_AT_DOOR").unwrap().update_value(Some(1));
    door.sensors.get_mut("TRAILER_ANGLE").unwrap().update_value(Some(0));
    door.sensors.get_mut("TRAILER_CENTERING").unwrap().update_value(Some(0));
    door.sensors.get_mut("TRAILER_DISTANCE").unwrap().update_value(Some(0));

    let event = DockDoorEvent::TrailerStateChanged(TrailerStateChangedEvent {
        dock_name: door.dock_name.clone(),
        old_state: TrailerState::Undocked,
        new_state: TrailerState::Docked,
        timestamp: now,
    });

    let results = rule.apply(&door, &event);
    assert_eq!(results.len(), 1, "One log entry should be generated for successful docking");
    match &results[0] {
        AnalysisResult::Log(LogEntry::DockingTime { door_name, shipment_id, event_type, success, .. }) => {
            assert_eq!(door_name, &door.dock_name);
            assert_eq!(shipment_id.as_ref().unwrap(), "TEST_SHIPMENT");
            assert_eq!(event_type, "TRAILER_DOCKING");
            assert!(success);
        },
        _ => panic!("Unexpected analysis result"),
    }
}

#[test]
fn test_new_shipment_previous_trailer_present_rule() {
    let config = serde_json::json!({
        "completion_statuses": ["Completed", "Shipped"]
    });
    let rule = Arc::new(NewShipmentPreviousTrailerPresentRule::new(config));
    let mut door = create_mock_door();
    let now = Local::now().naive_local();

    // Set up scenario
    door.trailer_state = TrailerState::Docked;
    door.wms_shipment_status = Some("Completed".to_string());
    door.assigned_shipment.current_shipment = Some("OLD_SHIPMENT".to_string());

    let event = DockDoorEvent::ShipmentAssigned(ShipmentAssignedEvent {
        dock_name: door.dock_name.clone(),
        shipment_id: "NEW_SHIPMENT".to_string(),
        timestamp: now,
        previous_shipment: Some("OLD_SHIPMENT".to_string()),
    });

    let results = rule.apply(&door, &event);
    assert_eq!(results.len(), 2, "Two results should be generated: an alert and a log entry");

    let alert = results.iter().find(|r| matches!(r, AnalysisResult::Alert(_))).unwrap();
    let log = results.iter().find(|r| matches!(r, AnalysisResult::Log(_))).unwrap();

    match alert {
        AnalysisResult::Alert(AlertType::NewShipmentPreviousTrailerPresent { dock_name, new_shipment, previous_shipment, timestamp }) => {
            assert_eq!(dock_name, &door.dock_name);
            assert_eq!(new_shipment, "NEW_SHIPMENT");
            assert_eq!(previous_shipment.as_ref().unwrap(), "OLD_SHIPMENT");
            assert_eq!(timestamp, &now);
        },
        _ => panic!("Unexpected alert type"),
    }

    match log {
        AnalysisResult::Log(LogEntry::NewShipmentPreviousTrailerPresent { door_name, shipment_id, event_type, success, .. }) => {
            assert_eq!(door_name, &door.dock_name);
            assert_eq!(shipment_id.as_ref().unwrap(), "NEW_SHIPMENT");
            assert_eq!(event_type, "NEW_SHIPMENT_PREVIOUS_TRAILER_PRESENT");
            assert!(!success);
        },
        _ => panic!("Unexpected log entry type"),
    }
}

#[test]
fn test_manual_intervention_rule() {
    let config = serde_json::json!({
        "check_interval": 5,
        "max_checks": 3
    });
    let rule = Arc::new(ManualInterventionRule::new(config));
    let door = create_mock_door();
    let now = Local::now().naive_local();

    // Test manual mode activation
    let event = DockDoorEvent::SensorStateChanged(SensorStateChangedEvent {
        dock_name: door.dock_name.clone(),
        sensor_name: "RH_MANUAL_MODE".to_string(),
        old_value: Some(0),
        new_value: Some(1),
        timestamp: now,
    });

    let results = rule.apply(&door, &event);
    assert_eq!(results.len(), 1, "One log entry should be generated for manual mode activation");
    match &results[0] {
        AnalysisResult::Log(LogEntry::ManualInterventionStarted { door_name, event_type, success, .. }) => {
            assert_eq!(door_name, &door.dock_name);
            assert_eq!(event_type, "MANUAL_INTERVENTION_STARTED");
            assert!(success);
        },
        _ => panic!("Unexpected analysis result"),
    }

    // TODO: Add more tests for manual intervention timeout and success scenarios
}

#[test]
fn test_trailer_undocking_rule() {
    let rule = Arc::new(TrailerUndockingRule);
    let door = create_mock_door();
    let now = Local::now().naive_local();

    let event = DockDoorEvent::TrailerStateChanged(TrailerStateChangedEvent {
        dock_name: door.dock_name.clone(),
        old_state: TrailerState::Docked,
        new_state: TrailerState::Undocked,
        timestamp: now,
    });

    let results = rule.apply(&door, &event);
    assert_eq!(results.len(), 1, "One log entry should be generated for trailer undocking");
    match &results[0] {
        AnalysisResult::Log(LogEntry::DockingTime { door_name, event_type, success, .. }) => {
            assert_eq!(door_name, &door.dock_name);
            assert_eq!(event_type, "TRAILER_UNDOCKING");
            assert!(success);
        },
        _ => panic!("Unexpected analysis result"),
    }
}

// You can add more tests for edge cases and different scenarios for each rule


