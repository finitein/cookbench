//! macOS status-item Stove presentation.
//!
//! This module deliberately receives the already ordered desktop snapshot. It
//! never reads Harness files or derives another priority order.

use std::sync::Mutex;

use crate::app_state::{StoveSnapshot, StoveStateWire, StoveWire};

pub const STATUS_HEIGHT: u32 = 22;
const SLOT_WIDTH: u32 = 22;
const SLOT_GAP: u32 = 2;
const HORIZONTAL_PADDING: u32 = 2;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusSlot {
    pub stove_id: String,
    /// Inclusive image-space left edge.
    pub start_x: u32,
    /// Exclusive image-space right edge.
    pub end_x: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusImage {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusPresentation {
    pub image: StatusImage,
    pub slots: Vec<StatusSlot>,
}

/// Immutable slots are replaced only after their paired native image update
/// succeeds. This keeps status-item hit testing tied to exactly one rendering.
#[derive(Default)]
pub struct StatusStovesState {
    slots: Mutex<Vec<StatusSlot>>,
}

impl StatusStovesState {
    pub fn replace_slots(&self, slots: Vec<StatusSlot>) {
        *self.slots.lock().expect("status Stove slots lock poisoned") = slots;
    }

    pub fn clear(&self) {
        self.replace_slots(Vec::new());
    }

    pub fn stove_at(&self, image_x: f64) -> Option<String> {
        hit_test(
            &self.slots.lock().expect("status Stove slots lock poisoned"),
            image_x,
        )
    }
}

pub fn presentation(snapshot: &StoveSnapshot, requested_count: u8) -> Option<StatusPresentation> {
    let count = usize::from(requested_count.min(8));
    if count == 0 {
        return None;
    }
    let selected = snapshot
        .attention_order
        .iter()
        .filter_map(|id| snapshot.stoves.iter().find(|stove| stove.id == *id))
        .take(count)
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return None;
    }
    Some(render(&selected))
}

pub fn hit_test(slots: &[StatusSlot], image_x: f64) -> Option<String> {
    if !image_x.is_finite() || image_x < 0.0 {
        return None;
    }
    slots
        .iter()
        .find(|slot| image_x >= f64::from(slot.start_x) && image_x < f64::from(slot.end_x))
        .map(|slot| slot.stove_id.clone())
}

fn render(stoves: &[&StoveWire]) -> StatusPresentation {
    let count = u32::try_from(stoves.len()).expect("status Stove count is bounded");
    let width = HORIZONTAL_PADDING * 2 + count * SLOT_WIDTH + (count - 1) * SLOT_GAP;
    let mut image = StatusImage {
        rgba: vec![0; usize::try_from(width * STATUS_HEIGHT * 4).expect("bounded image size")],
        width,
        height: STATUS_HEIGHT,
    };
    let mut slots = Vec::with_capacity(stoves.len());
    for (index, stove) in stoves.iter().enumerate() {
        let index = u32::try_from(index).expect("bounded status Stove index");
        let start_x = HORIZONTAL_PADDING + index * (SLOT_WIDTH + SLOT_GAP);
        draw_stove(&mut image, start_x, stove.state);
        slots.push(StatusSlot {
            stove_id: stove.id.clone(),
            start_x,
            end_x: start_x + SLOT_WIDTH,
        });
    }
    StatusPresentation { image, slots }
}

fn draw_stove(image: &mut StatusImage, offset_x: u32, state: StoveStateWire) {
    let color = state_color(state);
    let center_x = offset_x + SLOT_WIDTH / 2;
    let center_y = STATUS_HEIGHT / 2;
    let outer_radius = 8_i32;
    let inner_radius = 5_i32;
    for y in 0..STATUS_HEIGHT {
        for x in offset_x..offset_x + SLOT_WIDTH {
            let dx =
                i32::try_from(x).expect("bounded x") - i32::try_from(center_x).expect("bounded x");
            let dy =
                i32::try_from(y).expect("bounded y") - i32::try_from(center_y).expect("bounded y");
            let distance_squared = dx * dx + dy * dy;
            if distance_squared <= outer_radius * outer_radius
                && distance_squared >= inner_radius * inner_radius
            {
                put_pixel(image, x, y, color);
            }
        }
    }
    // A compact centered mark makes the ring identifiable at menu-bar scale.
    for y in center_y.saturating_sub(2)..=center_y + 2 {
        put_pixel(image, center_x, y, color);
    }
    for x in center_x.saturating_sub(2)..=center_x + 2 {
        put_pixel(image, x, center_y, color);
    }
}

fn put_pixel(image: &mut StatusImage, x: u32, y: u32, color: [u8; 4]) {
    let index = usize::try_from((y * image.width + x) * 4).expect("bounded image index");
    image.rgba[index..index + 4].copy_from_slice(&color);
}

fn state_color(state: StoveStateWire) -> [u8; 4] {
    match state {
        StoveStateWire::NeedsHuman => [194, 72, 69, 255],
        StoveStateWire::Failed => [166, 54, 62, 255],
        StoveStateWire::Disconnected => [96, 112, 128, 255],
        StoveStateWire::Cooked => [19, 113, 69, 255],
        StoveStateWire::Starting | StoveStateWire::Planning | StoveStateWire::Cooking => {
            [46, 116, 174, 255]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::{HarnessWire, HostKindWire, HostWire};

    fn stove(id: &str, state: StoveStateWire) -> StoveWire {
        StoveWire {
            id: id.into(),
            harness: HarnessWire {
                id: "codex".into(),
                label: "Codex".into(),
            },
            host: HostWire {
                kind: HostKindWire::Local,
                id: "local".into(),
            },
            project_root: "Project".into(),
            project_label: "Project".into(),
            project_root_display: "Project".into(),
            task_title: None,
            current_action: None,
            next_action: None,
            elapsed_ms: None,
            state,
            progress: None,
            locator_capability: crate::app_state::LocatorCapability::Available,
            retained_completion: false,
            pinned: false,
        }
    }

    fn snapshot(count: usize) -> StoveSnapshot {
        let stoves = (0..count)
            .map(|index| stove(&format!("stove-{index}"), StoveStateWire::Cooking))
            .collect::<Vec<_>>();
        StoveSnapshot {
            revision: 1,
            attention_order: stoves.iter().map(|stove| stove.id.clone()).collect(),
            stoves,
        }
    }

    #[test]
    fn requested_zero_or_an_empty_snapshot_uses_the_static_icon_fallback() {
        assert!(presentation(&snapshot(1), 0).is_none());
        assert!(presentation(&snapshot(0), 3).is_none());
    }

    #[test]
    fn rasterizer_is_nonblank_and_bounded_for_one_three_and_eight_slots() {
        for count in [1, 3, 8] {
            let rendered = presentation(&snapshot(count), 8).expect("stoves render");
            assert_eq!(rendered.slots.len(), count);
            assert_eq!(rendered.image.height, STATUS_HEIGHT);
            assert_eq!(
                rendered.image.rgba.len(),
                (rendered.image.width * STATUS_HEIGHT * 4) as usize
            );
            assert!(rendered
                .image
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel[3] != 0));
        }
    }

    #[test]
    fn slots_follow_canonical_attention_order_without_reranking() {
        let mut input = snapshot(3);
        input.attention_order = vec!["stove-2".into(), "stove-0".into(), "stove-1".into()];
        let rendered = presentation(&input, 3).expect("stoves render");
        assert_eq!(
            rendered
                .slots
                .iter()
                .map(|slot| slot.stove_id.as_str())
                .collect::<Vec<_>>(),
            ["stove-2", "stove-0", "stove-1"]
        );
    }

    #[test]
    fn hit_testing_refuses_gaps_and_uses_exact_slot_boundaries() {
        let rendered = presentation(&snapshot(3), 3).expect("stoves render");
        let first = &rendered.slots[0];
        let second = &rendered.slots[1];
        assert_eq!(
            hit_test(&rendered.slots, f64::from(first.start_x)),
            Some("stove-0".into())
        );
        assert_eq!(
            hit_test(&rendered.slots, f64::from(first.end_x) - 0.01),
            Some("stove-0".into())
        );
        assert_eq!(hit_test(&rendered.slots, f64::from(first.end_x)), None);
        assert_eq!(
            hit_test(&rendered.slots, f64::from(second.start_x)),
            Some("stove-1".into())
        );
        assert_eq!(hit_test(&rendered.slots, -1.0), None);
    }

    #[test]
    fn clearing_the_atomic_slot_snapshot_refuses_stale_clicks() {
        let state = StatusStovesState::default();
        let rendered = presentation(&snapshot(1), 1).expect("stove renders");
        state.replace_slots(rendered.slots);
        assert_eq!(state.stove_at(3.0), Some("stove-0".into()));
        state.clear();
        assert_eq!(state.stove_at(3.0), None);
    }
}
