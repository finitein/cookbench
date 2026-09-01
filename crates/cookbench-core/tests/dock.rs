use cookbench_core::persistence::{
    dock_threshold_physical, resolve_top_dock, top_dock_decision, DockMonitorWorkArea,
    GlobalBarTopDock, MonitorIdentity, MonitorWorkArea, TopDockDecision, TopDockInput,
    WindowPosition, WindowSize, TOP_DOCK_THRESHOLD_LOGICAL_PX, TOP_DOCK_TRIGGER_LOGICAL_PX,
    TOP_UNDOCK_THRESHOLD_LOGICAL_PX,
};

fn monitor(id: &str, x: i32, width: u32, primary: bool, scale: f64) -> DockMonitorWorkArea {
    DockMonitorWorkArea::new(
        MonitorWorkArea {
            identity: MonitorIdentity {
                id: id.into(),
                name: None,
            },
            x,
            y: 10,
            width,
            height: 900,
            primary,
        },
        scale,
    )
}

fn input<'a>(
    position: WindowPosition,
    monitors: &'a [DockMonitorWorkArea],
    prior_dock: Option<&'a GlobalBarTopDock>,
) -> TopDockInput<'a> {
    TopDockInput {
        position,
        size: WindowSize {
            width: 300,
            height: 80,
        },
        monitors,
        prior_dock,
        reliable_positioning: true,
    }
}

#[test]
fn docks_at_exact_threshold_and_not_one_physical_pixel_beyond_at_all_scales() {
    for scale in [1.0, 1.25, 2.0] {
        let monitors = [monitor("main", 0, 1200, true, scale)];
        let threshold = dock_threshold_physical(TOP_DOCK_THRESHOLD_LOGICAL_PX, scale) as i32;
        assert!(matches!(
            top_dock_decision(input(
                WindowPosition {
                    x: 200,
                    y: 10 + threshold
                },
                &monitors,
                None
            )),
            TopDockDecision::Dock(_)
        ));
        assert!(matches!(
            top_dock_decision(input(
                WindowPosition {
                    x: 200,
                    y: 11 + threshold
                },
                &monitors,
                None
            )),
            TopDockDecision::Freeform
        ));
    }
}

#[test]
fn docked_bar_undocks_at_exact_threshold_and_stays_just_under_at_all_scales() {
    for scale in [1.0, 1.25, 2.0] {
        let monitors = [monitor("main", 0, 1200, true, scale)];
        let dock = GlobalBarTopDock::from_absolute(
            &monitors[0].work_area,
            WindowPosition { x: 200, y: 10 },
            WindowSize {
                width: 300,
                height: 80,
            },
        );
        let threshold = dock_threshold_physical(TOP_UNDOCK_THRESHOLD_LOGICAL_PX, scale) as i32;
        assert!(matches!(
            top_dock_decision(input(
                WindowPosition {
                    x: 200,
                    y: 10 + threshold - 1
                },
                &monitors,
                Some(&dock)
            )),
            TopDockDecision::RemainDocked(_)
        ));
        assert!(matches!(
            top_dock_decision(input(
                WindowPosition {
                    x: 200,
                    y: 10 + threshold
                },
                &monitors,
                Some(&dock)
            )),
            TopDockDecision::Undock
        ));
    }
}

#[test]
fn collapsed_position_leaves_the_scaled_trigger_strip_for_any_bar_height() {
    let monitors = [monitor("main", 0, 1200, true, 1.25)];
    for height in [48, 104] {
        let size = WindowSize { width: 300, height };
        let dock = GlobalBarTopDock::from_absolute(
            &monitors[0].work_area,
            WindowPosition { x: 200, y: 10 },
            size,
        );
        let resolved = resolve_top_dock(&dock, size, &monitors).unwrap();
        assert_eq!(resolved.expanded_position.y, 10);
        assert_eq!(
            i64::from(resolved.collapsed_position.y) + i64::from(height) - 10,
            i64::from(dock_threshold_physical(TOP_DOCK_TRIGGER_LOGICAL_PX, 1.25))
        );
    }
}

#[test]
fn horizontal_position_clamps_and_restores_relatively_after_resize() {
    let wide = [monitor("main", 0, 1600, true, 1.0)];
    let size = WindowSize {
        width: 300,
        height: 80,
    };
    let dock = GlobalBarTopDock::from_absolute(
        &wide[0].work_area,
        WindowPosition { x: 9999, y: 10 },
        size,
    );
    assert_eq!(dock.relative_x, 10_000);
    let narrow = [monitor("main", 0, 900, true, 1.0)];
    assert_eq!(
        resolve_top_dock(&dock, size, &narrow)
            .unwrap()
            .expanded_position
            .x,
        600
    );
}

#[test]
fn missing_dock_monitor_uses_greatest_intersection_then_primary() {
    let monitors = [
        monitor("left", 0, 800, true, 1.0),
        monitor("right", 800, 1200, false, 1.0),
    ];
    assert!(matches!(
        top_dock_decision(input(WindowPosition { x: 1000, y: 10 }, &monitors, None)),
        TopDockDecision::Dock(dock) if dock.monitor.id == "right"
    ));
    assert!(matches!(
        top_dock_decision(input(WindowPosition { x: 3000, y: 10 }, &monitors, None)),
        TopDockDecision::Dock(dock) if dock.monitor.id == "left"
    ));
}

#[test]
fn unreliable_coordinates_never_infer_a_dock() {
    let monitors = [monitor("main", 0, 1200, true, 1.0)];
    let mut request = input(WindowPosition { x: 200, y: 10 }, &monitors, None);
    request.reliable_positioning = false;
    assert_eq!(top_dock_decision(request), TopDockDecision::Freeform);
}

#[test]
fn dock_deserialization_clamps_corrupted_relative_position() {
    let dock: GlobalBarTopDock =
        serde_json::from_str(r#"{"monitor":{"id":"main"},"relative_x":999999}"#).unwrap();
    assert_eq!(dock.relative_x, 10_000);
}
