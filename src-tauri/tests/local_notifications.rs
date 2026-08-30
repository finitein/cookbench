use std::cell::RefCell;

use cookbench_core::{
    notifications::NotificationEventKind, persistence::LocalNotificationPreferences,
};
use cookbench_desktop_lib::notifications::local::{
    LocalAlertChannel, LocalAlertDispatcher, LocalAlertEffects, LocalAlertPayload,
    LocalAlertResult, SystemSoundCommand,
};

#[derive(Default)]
struct FakeEffects {
    calls: RefCell<Vec<LocalAlertChannel>>,
}

impl LocalAlertEffects for FakeEffects {
    fn play_sound(&self) -> LocalAlertResult {
        self.calls.borrow_mut().push(LocalAlertChannel::Sound);
        LocalAlertResult::Delivered
    }

    fn show_banner(&self, _: &LocalAlertPayload) -> LocalAlertResult {
        self.calls
            .borrow_mut()
            .push(LocalAlertChannel::SystemBanner);
        LocalAlertResult::Delivered
    }

    fn flash_stove(&self, _: &LocalAlertPayload) -> LocalAlertResult {
        self.calls.borrow_mut().push(LocalAlertChannel::BarFlash);
        LocalAlertResult::Delivered
    }

    fn request_attention(&self) -> LocalAlertResult {
        self.calls
            .borrow_mut()
            .push(LocalAlertChannel::SystemAttention);
        LocalAlertResult::Delivered
    }
}

fn payload(event: NotificationEventKind) -> LocalAlertPayload {
    LocalAlertPayload::new("stove-42", "Cookbench", event)
}

#[test]
fn default_preferences_deliver_only_sound_for_selected_events() {
    let effects = FakeEffects::default();
    let dispatcher = LocalAlertDispatcher::default();

    let delivered = dispatcher.dispatch(
        &LocalNotificationPreferences::default(),
        &payload(NotificationEventKind::Cooked),
        1_000,
        &effects,
    );

    assert_eq!(
        delivered,
        vec![(LocalAlertChannel::Sound, LocalAlertResult::Delivered)]
    );
    assert_eq!(*effects.calls.borrow(), vec![LocalAlertChannel::Sound]);
}

#[test]
fn disabled_event_does_not_deliver_any_channel() {
    let effects = FakeEffects::default();
    let dispatcher = LocalAlertDispatcher::default();

    let delivered = dispatcher.dispatch(
        &LocalNotificationPreferences::default(),
        &payload(NotificationEventKind::CookingStarted),
        1_000,
        &effects,
    );

    assert!(delivered.is_empty());
    assert!(effects.calls.borrow().is_empty());
}

#[test]
fn enabled_channels_are_independent_and_duplicate_events_are_suppressed() {
    let effects = FakeEffects::default();
    let dispatcher = LocalAlertDispatcher::default();
    let preferences = LocalNotificationPreferences {
        sound: true,
        system_banner: true,
        bar_flash: true,
        system_attention: true,
        events: vec![NotificationEventKind::NeedsHuman],
    };

    assert_eq!(
        dispatcher
            .dispatch(
                &preferences,
                &payload(NotificationEventKind::NeedsHuman),
                5_000,
                &effects,
            )
            .len(),
        4
    );
    assert!(dispatcher
        .dispatch(
            &preferences,
            &payload(NotificationEventKind::NeedsHuman),
            5_500,
            &effects,
        )
        .is_empty());
    assert_eq!(
        dispatcher
            .dispatch(
                &preferences,
                &payload(NotificationEventKind::NeedsHuman),
                6_001,
                &effects,
            )
            .len(),
        4
    );
}

#[test]
fn payload_is_bounded_and_excludes_conversation_fields() {
    let payload = LocalAlertPayload::new(
        "s".repeat(700),
        "项目".repeat(400),
        NotificationEventKind::Failed,
    );
    let serialized = serde_json::to_string(&payload).expect("serialize payload");

    assert!(payload.stove_id.chars().count() <= 128);
    assert!(payload.project.chars().count() <= 128);
    assert!(!serialized.contains("prompt"));
    assert!(!serialized.contains("command"));
    assert!(!serialized.contains("activity"));
}

#[test]
fn platform_sound_commands_use_fixed_programs_and_arguments() {
    let mac = SystemSoundCommand::for_platform("macos").expect("mac sound");
    assert_eq!(mac.program, "/usr/bin/osascript");
    assert_eq!(mac.args, ["-e", "beep"]);

    let windows = SystemSoundCommand::for_platform("windows").expect("windows sound");
    assert_eq!(windows.program, "powershell.exe");
    assert!(windows.args.iter().any(|arg| arg.contains("SystemSounds")));

    let linux = SystemSoundCommand::for_platform("linux").expect("linux sound");
    assert_eq!(linux.program, "canberra-gtk-play");
    assert_eq!(linux.args, ["--id", "message"]);

    assert!(SystemSoundCommand::for_platform("unknown").is_none());
}
