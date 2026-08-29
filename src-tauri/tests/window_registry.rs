use cookbench_core::persistence::{
    DetachedStoveLayout, MonitorIdentity, MonitorWorkArea, WindowPosition, WindowSize,
};
use cookbench_desktop_lib::window_registry::{
    DetachOutcome, DetachedWindowHost, DetachedWindowRecord, WindowRegistry,
};

#[derive(Default)]
struct FakeWindows {
    created: Vec<(String, WindowPosition)>,
    presented: Vec<String>,
    closed: Vec<String>,
}

impl DetachedWindowHost for FakeWindows {
    type Error = String;

    fn create(
        &mut self,
        record: &DetachedWindowRecord,
        position: WindowPosition,
    ) -> Result<(), Self::Error> {
        self.created.push((record.label.clone(), position));
        Ok(())
    }

    fn present(&mut self, label: &str) -> Result<(), Self::Error> {
        self.presented.push(label.to_owned());
        Ok(())
    }

    fn close(&mut self, label: &str) -> Result<(), Self::Error> {
        self.closed.push(label.to_owned());
        Ok(())
    }
}

fn monitor(id: &str, x: i32, width: u32, primary: bool) -> MonitorWorkArea {
    MonitorWorkArea {
        identity: MonitorIdentity {
            id: id.into(),
            name: Some(id.into()),
        },
        x,
        y: 0,
        width,
        height: 900,
        primary,
    }
}

fn layout(
    stove_key: &str,
    monitor: &MonitorWorkArea,
    position: WindowPosition,
) -> DetachedStoveLayout {
    DetachedStoveLayout::from_absolute(
        stove_key,
        monitor,
        position,
        WindowSize {
            width: 360,
            height: 104,
        },
    )
}

#[test]
fn one_detached_window_per_stove_coexists_with_global_bar() {
    let display = monitor("primary", 0, 1440, true);
    let mut registry = WindowRegistry::new(true);
    let mut windows = FakeWindows::default();

    let first = registry
        .detach(
            &mut windows,
            layout("session-a", &display, WindowPosition { x: 100, y: 100 }),
            std::slice::from_ref(&display),
        )
        .unwrap();
    let second = registry
        .detach(
            &mut windows,
            layout("session-a", &display, WindowPosition { x: 200, y: 200 }),
            std::slice::from_ref(&display),
        )
        .unwrap();
    let third = registry
        .detach(
            &mut windows,
            layout("session-b", &display, WindowPosition { x: 300, y: 300 }),
            &[display],
        )
        .unwrap();

    assert!(registry.global_bar_visible());
    assert!(matches!(first, DetachOutcome::Created(_)));
    assert!(matches!(second, DetachOutcome::PresentedExisting(_)));
    assert!(matches!(third, DetachOutcome::Created(_)));
    assert_eq!(windows.created.len(), 2);
    assert_eq!(windows.presented.len(), 1);
    assert_eq!(registry.layouts().len(), 2);
}

#[test]
fn restores_using_relative_position_and_falls_back_to_primary_monitor() {
    let missing = monitor("missing", 1920, 2560, false);
    let saved = layout("session-a", &missing, WindowPosition { x: 3900, y: 700 });
    let primary = monitor("primary", 0, 1280, true);
    let mut registry = WindowRegistry::new(true);
    let mut windows = FakeWindows::default();

    registry
        .restore_all(&mut windows, vec![saved], &[primary])
        .unwrap();

    assert_eq!(windows.created[0].1, WindowPosition { x: 828, y: 700 });
    assert_eq!(registry.layouts()[0].monitor.id, "primary");
}

#[test]
fn manual_clear_closes_and_removes_only_the_matching_detached_window() {
    let display = monitor("primary", 0, 1440, true);
    let mut registry = WindowRegistry::new(true);
    let mut windows = FakeWindows::default();
    registry
        .detach(
            &mut windows,
            layout("session-a", &display, WindowPosition { x: 100, y: 100 }),
            std::slice::from_ref(&display),
        )
        .unwrap();
    registry
        .detach(
            &mut windows,
            layout("session-b", &display, WindowPosition { x: 500, y: 100 }),
            &[display],
        )
        .unwrap();

    assert!(registry.clear_stove(&mut windows, "session-a").unwrap());
    assert!(!registry.clear_stove(&mut windows, "session-a").unwrap());

    assert_eq!(windows.closed.len(), 1);
    assert!(registry.detached("session-a").is_none());
    assert!(registry.detached("session-b").is_some());
    assert!(registry.global_bar_visible());
}
