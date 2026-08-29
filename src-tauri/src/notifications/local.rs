use cookbench_core::notifications::event::{LocalNotificationEvent, LocalNotificationKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationPermission {
    Granted,
    Denied,
    NotDetermined,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalNotification {
    pub title: String,
    pub body: String,
}

pub trait LocalNotificationBackend {
    fn permission(&self) -> NotificationPermission;
    fn show(&self, notification: LocalNotification) -> Result<(), String>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalNotificationOutcome {
    Delivered,
    PermissionDenied,
    Unavailable,
}

/// Delivers local feedback without influencing the authoritative stove state.
///
/// Platform permission requests and backend failures are intentionally folded
/// into outcomes rather than propagated to callers as errors.
pub fn notify_best_effort(
    backend: &impl LocalNotificationBackend,
    event: &LocalNotificationEvent,
) -> LocalNotificationOutcome {
    if event.kind != LocalNotificationKind::Cooked {
        return LocalNotificationOutcome::Unavailable;
    }

    match backend.permission() {
        NotificationPermission::Denied => LocalNotificationOutcome::PermissionDenied,
        NotificationPermission::NotDetermined => LocalNotificationOutcome::Unavailable,
        NotificationPermission::Granted => backend
            .show(LocalNotification {
                title: "Cookbench".to_owned(),
                body: "A stove finished cooking.".to_owned(),
            })
            .map_or(LocalNotificationOutcome::Unavailable, |_| {
                LocalNotificationOutcome::Delivered
            }),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use cookbench_core::notifications::event::LocalNotificationEvent;

    use super::{
        notify_best_effort, LocalNotification, LocalNotificationBackend, LocalNotificationOutcome,
        NotificationPermission,
    };

    struct FakeBackend {
        permission: NotificationPermission,
        deliveries: Cell<u8>,
        fails: bool,
    }

    impl LocalNotificationBackend for FakeBackend {
        fn permission(&self) -> NotificationPermission {
            self.permission
        }

        fn show(&self, _: LocalNotification) -> Result<(), String> {
            self.deliveries.set(self.deliveries.get() + 1);
            if self.fails {
                Err("platform backend unavailable".to_owned())
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn permission_denial_is_best_effort_and_never_attempts_delivery() {
        let backend = FakeBackend {
            permission: NotificationPermission::Denied,
            deliveries: Cell::new(0),
            fails: false,
        };

        let outcome = notify_best_effort(
            &backend,
            &LocalNotificationEvent::cooked("local:codex:session-42", 42),
        );

        assert_eq!(outcome, LocalNotificationOutcome::PermissionDenied);
        assert_eq!(backend.deliveries.get(), 0);
    }

    #[test]
    fn backend_failure_does_not_escape_into_state_handling() {
        let backend = FakeBackend {
            permission: NotificationPermission::Granted,
            deliveries: Cell::new(0),
            fails: true,
        };

        let outcome = notify_best_effort(
            &backend,
            &LocalNotificationEvent::cooked("local:codex:session-42", 42),
        );

        assert_eq!(outcome, LocalNotificationOutcome::Unavailable);
        assert_eq!(backend.deliveries.get(), 1);
    }
}
