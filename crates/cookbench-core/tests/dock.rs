use cookbench_core::persistence::{
    dock_threshold_physical, dock_upper_threshold_physical, resolve_top_dock, top_dock_decision,
    DockMonitorWorkArea, GlobalBarTopDock, MonitorIdentity, MonitorWorkArea, TopDockDecision,
    TopDockInput, WindowPosition, WindowSize, TOP_DOCK_THRESHOLD_LOGICAL_PX,
    TOP_DOCK_TRIGGER_LOGICAL_PX, TOP_UNDOCK_THRESHOLD_LOGICAL_PX,
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
        let threshold = dock_upper_threshold_physical(TOP_DOCK_THRESHOLD_LOGICAL_PX, scale) as i32;
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
fn fractional_scale_rounds_dock_down_and_undock_up() {
    let monitors = [monitor("main", 0, 1200, true, 1.1)];
    assert_eq!(
        dock_upper_threshold_physical(TOP_DOCK_THRESHOLD_LOGICAL_PX, 1.1),
        13
    );
    assert_eq!(
        dock_threshold_physical(TOP_UNDOCK_THRESHOLD_LOGICAL_PX, 1.1),
        27
    );
    assert!(matches!(
        top_dock_decision(input(WindowPosition { x: 200, y: 23 }, &monitors, None)),
        TopDockDecision::Dock(_)
    ));
    assert!(matches!(
        top_dock_decision(input(WindowPosition { x: 200, y: 24 }, &monitors, None)),
        TopDockDecision::Freeform
    ));

    let dock = GlobalBarTopDock::from_absolute(
        &monitors[0].work_area,
        WindowPosition { x: 200, y: 10 },
        WindowSize {
            width: 300,
            height: 80,
        },
    );
    assert!(matches!(
        top_dock_decision(input(
            WindowPosition { x: 200, y: 36 },
            &monitors,
            Some(&dock)
        )),
        TopDockDecision::RemainDocked(_)
    ));
    assert!(matches!(
        top_dock_decision(input(
            WindowPosition { x: 200, y: 37 },
            &monitors,
            Some(&dock)
        )),
        TopDockDecision::Undock
    ));
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
fn collapsed_position_never_leaves_more_than_the_window_height_visible() {
    let mut work_area = monitor("main", 0, 1200, true, 2.0);
    work_area.work_area.y = -120;
    let monitors = [work_area];
    for height in [0, 2, 5] {
        let size = WindowSize { width: 300, height };
        let dock = GlobalBarTopDock::from_absolute(
            &monitors[0].work_area,
            WindowPosition { x: 200, y: -120 },
            size,
        );
        let resolved = resolve_top_dock(&dock, size, &monitors).unwrap();
        let visible = i64::from(resolved.collapsed_position.y) + i64::from(height) + 120;
        assert_eq!(visible, i64::from(height.min(6)));
        assert!(resolved.collapsed_position.y <= -120);
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
fn dragging_a_docked_bar_to_another_monitor_uses_its_current_monitor() {
    let monitors = [
        monitor("left", 0, 800, true, 1.0),
        monitor("right", 800, 1200, false, 1.0),
    ];
    let dock = GlobalBarTopDock::from_absolute(
        &monitors[0].work_area,
        WindowPosition { x: 200, y: 10 },
        WindowSize {
            width: 300,
            height: 80,
        },
    );
    assert!(matches!(
        top_dock_decision(input(WindowPosition { x: 1000, y: 10 }, &monitors, Some(&dock))),
        TopDockDecision::RemainDocked(updated) if updated.monitor.id == "right"
    ));
}

#[test]
fn unreliable_coordinates_leave_the_current_placement_unchanged() {
    let monitors = [monitor("main", 0, 1200, true, 1.0)];
    let dock = GlobalBarTopDock::from_absolute(
        &monitors[0].work_area,
        WindowPosition { x: 200, y: 10 },
        WindowSize {
            width: 300,
            height: 80,
        },
    );
    for prior_dock in [None, Some(&dock)] {
        let mut request = input(WindowPosition { x: 200, y: 10 }, &monitors, prior_dock);
        request.reliable_positioning = false;
        assert_eq!(
            top_dock_decision(request),
            TopDockDecision::BestEffortVisible
        );
    }
}

#[test]
fn extreme_rectangles_do_not_overflow_monitor_intersection_selection() {
    let monitors = [
        monitor("primary", i32::MIN, u32::MAX, true, 1.0),
        monitor("other", 0, u32::MAX, false, 1.0),
    ];
    assert!(matches!(
        top_dock_decision(TopDockInput {
            position: WindowPosition { x: i32::MIN, y: 10 },
            size: WindowSize {
                width: u32::MAX,
                height: u32::MAX
            },
            monitors: &monitors,
            prior_dock: None,
            reliable_positioning: true,
        }),
        TopDockDecision::Dock(_) | TopDockDecision::Freeform
    ));
}

#[test]
fn dock_deserialization_clamps_corrupted_relative_position() {
    let dock: GlobalBarTopDock =
        serde_json::from_str(r#"{"monitor":{"id":"main"},"relative_x":999999}"#).unwrap();
    assert_eq!(dock.relative_x, 10_000);
}
