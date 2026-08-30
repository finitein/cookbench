use cookbench_core::{notifications::NotificationEventKind, persistence::PersistedConfig};
use cookbench_desktop_lib::commands::notifications::{
    apply_local_notification_input, local_notification_settings_wire, LocalNotificationInput,
    NotificationEventWire,
};

#[test]
fn settings_wire_defaults_to_sound_only() {
    let wire = local_notification_settings_wire(&PersistedConfig::default());

    assert!(wire.sound);
    assert!(!wire.system_banner);
    assert!(!wire.bar_flash);
    assert!(!wire.system_attention);
    assert_eq!(
        wire.events,
        vec![
            NotificationEventWire::NeedsHuman,
            NotificationEventWire::Cooked,
            NotificationEventWire::Failed,
            NotificationEventWire::Disconnected,
        ]
    );
}

#[test]
fn settings_input_is_bounded_deduplicated_and_updates_only_local_preferences() {
    let mut config = PersistedConfig::default();
    config.preferences.notifications_enabled = true;
    let input = LocalNotificationInput {
        sound: false,
        system_banner: true,
        bar_flash: true,
        system_attention: false,
        events: vec![
            NotificationEventWire::Cooked,
            NotificationEventWire::NeedsHuman,
            NotificationEventWire::Cooked,
        ],
    };

    apply_local_notification_input(&mut config, input).expect("valid local settings");

    assert!(config.preferences.notifications_enabled);
    assert!(!config.preferences.local_notifications.sound);
    assert!(config.preferences.local_notifications.system_banner);
    assert!(config.preferences.local_notifications.bar_flash);
    assert_eq!(
        config.preferences.local_notifications.events,
        vec![
            NotificationEventKind::NeedsHuman,
            NotificationEventKind::Cooked
        ]
    );
}

#[test]
fn settings_reject_more_than_the_approved_event_vocabulary() {
    let mut config = PersistedConfig::default();
    let input = LocalNotificationInput {
        sound: true,
        system_banner: false,
        bar_flash: false,
        system_attention: false,
        events: vec![NotificationEventWire::Cooked; 11],
    };

    assert!(apply_local_notification_input(&mut config, input).is_err());
}
