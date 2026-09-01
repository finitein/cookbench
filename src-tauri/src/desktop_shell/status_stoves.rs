//! Bounded macOS status-item Stove presentation.
//!
//! The desktop snapshot already contains the canonical attention order. This
//! module only retains visual positions and maps safe native-menu targets.

use std::{collections::HashMap, sync::Mutex};

use crate::app_state::{StoveSnapshot, StoveStateWire, StoveWire};
use cookbench_core::persistence::AppLocale;

pub const STATUS_HEIGHT: u32 = 22;
const SLOT_WIDTH: u32 = 22;
const SLOT_GAP: u32 = 2;
const HORIZONTAL_PADDING: u32 = 2;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusSlot {
    pub stove_id: String,
    pub start_x: u32,
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusMenuStove {
    pub menu_id: String,
    pub stove_id: String,
    pub label: String,
}

#[derive(Default)]
struct RenderState {
    revision: u64,
    slots: Vec<StatusSlot>,
    image_width: u32,
    menu_targets: HashMap<String, String>,
}

/// Updates are committed only after the native icon/menu operation succeeds.
/// Clicks therefore use one immutable image/slot or menu/target pairing.
#[derive(Default)]
pub struct StatusStovesState {
    inner: Mutex<RenderState>,
}

impl StatusStovesState {
    pub fn accepts_revision(&self, revision: u64) -> bool {
        revision
            >= self
                .inner
                .lock()
                .expect("status Stove state lock poisoned")
                .revision
    }

    pub fn presentation(&self, snapshot: &StoveSnapshot, count: u8) -> Option<StatusPresentation> {
        let inner = self.inner.lock().expect("status Stove state lock poisoned");
        presentation(snapshot, count, &inner.slots)
    }
    pub fn commit_presentation(
        &self,
        revision: u64,
        presentation: Option<&StatusPresentation>,
    ) -> bool {
        let mut inner = self.inner.lock().expect("status Stove state lock poisoned");
        if revision < inner.revision {
            return false;
        }
        inner.revision = inner.revision.max(revision);
        if let Some(presentation) = presentation {
            inner.slots = presentation.slots.clone();
            inner.image_width = presentation.image.width;
        } else {
            inner.slots.clear();
            inner.image_width = 0;
        }
        true
    }
    pub fn commit_menu(&self, revision: u64, menu: &[StatusMenuStove]) -> bool {
        let mut inner = self.inner.lock().expect("status Stove state lock poisoned");
        if revision < inner.revision {
            return false;
        }
        inner.revision = inner.revision.max(revision);
        inner.menu_targets = menu
            .iter()
            .map(|item| (item.menu_id.clone(), item.stove_id.clone()))
            .collect();
        true
    }
    pub fn stove_at_status_x(&self, local_x: f64, item_width: f64) -> Option<String> {
        let inner = self.inner.lock().expect("status Stove state lock poisoned");
        hit_test(
            &inner.slots,
            normalize_status_x(local_x, item_width, inner.image_width)?,
        )
    }
    pub fn menu_target(&self, menu_id: &str) -> Option<String> {
        self.inner
            .lock()
            .expect("status Stove state lock poisoned")
            .menu_targets
            .get(menu_id)
            .cloned()
    }
}

pub fn presentation(
    snapshot: &StoveSnapshot,
    count: u8,
    previous: &[StatusSlot],
) -> Option<StatusPresentation> {
    let ids = selected_stove_ids(snapshot, count, previous);
    if ids.is_empty() {
        return None;
    }
    let stoves = ids
        .iter()
        .filter_map(|id| snapshot.stoves.iter().find(|stove| stove.id == *id))
        .collect::<Vec<_>>();
    Some(render(&stoves))
}

/// The selected set is always the canonical top N. Retained IDs keep their
/// old slot index; newly qualifying IDs replace vacant lower-priority slots.
pub fn selected_stove_ids(
    snapshot: &StoveSnapshot,
    count: u8,
    previous: &[StatusSlot],
) -> Vec<String> {
    let count = usize::from(count.min(8));
    let canonical = snapshot
        .attention_order
        .iter()
        .filter(|id| snapshot.stoves.iter().any(|stove| stove.id == ***id))
        .take(count)
        .cloned()
        .collect::<Vec<_>>();
    let mut positions = vec![None; canonical.len()];
    for (index, slot) in previous.iter().take(positions.len()).enumerate() {
        if canonical.contains(&slot.stove_id) {
            positions[index] = Some(slot.stove_id.clone());
        }
    }
    let assigned = positions.iter().flatten().cloned().collect::<Vec<_>>();
    let mut new_ids = canonical.into_iter().filter(|id| !assigned.contains(id));
    for position in &mut positions {
        if position.is_none() {
            *position = new_ids.next();
        }
    }
    positions.into_iter().flatten().collect()
}

pub fn all_stove_menu(snapshot: &StoveSnapshot) -> Vec<StatusMenuStove> {
    all_stove_menu_for_locale(snapshot, AppLocale::En)
}

pub fn all_stove_menu_for_locale(
    snapshot: &StoveSnapshot,
    locale: AppLocale,
) -> Vec<StatusMenuStove> {
    snapshot
        .attention_order
        .iter()
        .filter_map(|id| snapshot.stoves.iter().find(|stove| stove.id == *id))
        .map(|stove| StatusMenuStove {
            menu_id: stable_menu_id(&stove.id),
            stove_id: stove.id.clone(),
            label: safe_status_label(stove, locale),
        })
        .collect()
}

pub fn accessibility_label(snapshot: &StoveSnapshot, count: u8) -> String {
    accessibility_label_for_locale(snapshot, count, AppLocale::En)
}

pub fn accessibility_label_for_locale(
    snapshot: &StoveSnapshot,
    count: u8,
    locale: AppLocale,
) -> String {
    let states = selected_stove_ids(snapshot, count, &[])
        .iter()
        .filter_map(|id| snapshot.stoves.iter().find(|stove| stove.id == *id))
        .map(|stove| state_label(stove.state, locale))
        .collect::<Vec<_>>();
    if states.is_empty() {
        "Cookbench".into()
    } else {
        let count = states.len();
        let states = states.join(", ");
        match locale {
            AppLocale::ZhCn => format!("Cookbench：{count} 个炉灶，{states}"),
            AppLocale::Ja => format!("Cookbench: {count} 台のストーブ、{states}"),
            AppLocale::Ko => format!("Cookbench: 스토브 {count}개, {states}"),
            AppLocale::System | AppLocale::En => {
                let noun = if count == 1 { "Stove" } else { "Stoves" };
                format!("Cookbench: {count} {noun}, {states}")
            }
        }
    }
}

/// Maps an actual status-item physical width to source-image pixels. This is
/// what keeps hit testing correct when AppKit rescales an RGBA image on Retina.
pub fn normalize_status_x(local_x: f64, item_width: f64, image_width: u32) -> Option<f64> {
    if !local_x.is_finite() || !item_width.is_finite() || local_x < 0.0 || item_width <= 0.0 {
        return None;
    }
    let image_x = local_x * f64::from(image_width) / item_width;
    (image_x < f64::from(image_width)).then_some(image_x)
}
pub fn hit_test(slots: &[StatusSlot], image_x: f64) -> Option<String> {
    slots
        .iter()
        .find(|slot| image_x >= f64::from(slot.start_x) && image_x < f64::from(slot.end_x))
        .map(|slot| slot.stove_id.clone())
}

fn stable_menu_id(stove_id: &str) -> String {
    let hash = stove_id
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        });
    format!("status-stove-{hash:016x}")
}
fn safe_status_label(stove: &StoveWire, locale: AppLocale) -> String {
    let mut label = String::new();
    let mut needs_space = false;
    for character in stove.project_label.chars() {
        if character.is_control() || is_unsafe_presentation_character(character) {
            continue;
        }
        if character.is_whitespace() {
            needs_space = !label.is_empty();
        } else {
            if needs_space {
                label.push(' ');
                needs_space = false;
            }
            label.push(character);
        }
        if label.chars().count() >= 64 {
            break;
        }
    }
    let label = if label.is_empty() {
        project_fallback(locale).into()
    } else {
        label
    };
    format!("{}: {label}", state_label(stove.state, locale))
}
fn is_unsafe_presentation_character(character: char) -> bool {
    matches!(character, '\u{00ad}' | '\u{061c}' | '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{206f}' | '\u{feff}')
}
fn project_fallback(locale: AppLocale) -> &'static str {
    match locale {
        AppLocale::ZhCn => "项目",
        AppLocale::Ja => "プロジェクト",
        AppLocale::Ko => "프로젝트",
        _ => "Project",
    }
}
fn state_label(state: StoveStateWire, locale: AppLocale) -> &'static str {
    match (locale, state) {
        (AppLocale::ZhCn, StoveStateWire::Starting) => "启动中",
        (AppLocale::ZhCn, StoveStateWire::Planning) => "规划中",
        (AppLocale::ZhCn, StoveStateWire::Cooking) => "运行中",
        (AppLocale::ZhCn, StoveStateWire::NeedsHuman) => "需要协助",
        (AppLocale::ZhCn, StoveStateWire::Cooked) => "已完成",
        (AppLocale::ZhCn, StoveStateWire::Failed) => "失败",
        (AppLocale::ZhCn, StoveStateWire::Disconnected) => "已断开",
        (AppLocale::Ja, StoveStateWire::Starting) => "開始中",
        (AppLocale::Ja, StoveStateWire::Planning) => "計画中",
        (AppLocale::Ja, StoveStateWire::Cooking) => "実行中",
        (AppLocale::Ja, StoveStateWire::NeedsHuman) => "対応待ち",
        (AppLocale::Ja, StoveStateWire::Cooked) => "完了",
        (AppLocale::Ja, StoveStateWire::Failed) => "失敗",
        (AppLocale::Ja, StoveStateWire::Disconnected) => "切断",
        (AppLocale::Ko, StoveStateWire::Starting) => "시작 중",
        (AppLocale::Ko, StoveStateWire::Planning) => "계획 중",
        (AppLocale::Ko, StoveStateWire::Cooking) => "진행 중",
        (AppLocale::Ko, StoveStateWire::NeedsHuman) => "지원 필요",
        (AppLocale::Ko, StoveStateWire::Cooked) => "완료",
        (AppLocale::Ko, StoveStateWire::Failed) => "실패",
        (AppLocale::Ko, StoveStateWire::Disconnected) => "연결 끊김",
        (_, state) => match state {
            StoveStateWire::Starting => "Starting",
            StoveStateWire::Planning => "Planning",
            StoveStateWire::Cooking => "Cooking",
            StoveStateWire::NeedsHuman => "Needs Human",
            StoveStateWire::Cooked => "Cooked",
            StoveStateWire::Failed => "Failed",
            StoveStateWire::Disconnected => "Disconnected",
        },
    }
}

fn render(stoves: &[&StoveWire]) -> StatusPresentation {
    let count = stoves.len() as u32;
    let width = HORIZONTAL_PADDING * 2 + count * SLOT_WIDTH + (count - 1) * SLOT_GAP;
    let mut image = StatusImage {
        rgba: vec![0; (width * STATUS_HEIGHT * 4) as usize],
        width,
        height: STATUS_HEIGHT,
    };
    let slots = stoves
        .iter()
        .enumerate()
        .map(|(index, stove)| {
            let start_x = HORIZONTAL_PADDING + index as u32 * (SLOT_WIDTH + SLOT_GAP);
            draw_stove(&mut image, start_x, stove.state, &stove.harness.id);
            StatusSlot {
                stove_id: stove.id.clone(),
                start_x,
                end_x: start_x + SLOT_WIDTH,
            }
        })
        .collect();
    StatusPresentation { image, slots }
}
fn draw_stove(image: &mut StatusImage, offset_x: u32, state: StoveStateWire, harness_id: &str) {
    let color = match state {
        StoveStateWire::NeedsHuman => [194, 72, 69, 255],
        StoveStateWire::Failed => [166, 54, 62, 255],
        StoveStateWire::Disconnected => [96, 112, 128, 255],
        StoveStateWire::Cooked => [19, 113, 69, 255],
        StoveStateWire::Starting | StoveStateWire::Planning | StoveStateWire::Cooking => {
            [46, 116, 174, 255]
        }
    };
    let cx = offset_x + SLOT_WIDTH / 2;
    let cy = STATUS_HEIGHT / 2;
    for y in 0..STATUS_HEIGHT {
        for x in offset_x..offset_x + SLOT_WIDTH {
            let dx = x as i32 - cx as i32;
            let dy = y as i32 - cy as i32;
            let d = dx * dx + dy * dy;
            if (25..=64).contains(&d) || harness_mark(harness_id, x, y, cx, cy) {
                let index = ((y * image.width + x) * 4) as usize;
                image.rgba[index..index + 4].copy_from_slice(&color);
            }
        }
    }
}
fn harness_mark(harness_id: &str, x: u32, y: u32, cx: u32, cy: u32) -> bool {
    match harness_id {
        "codex" => (x == cx && y.abs_diff(cy) <= 2) || (y == cy && x.abs_diff(cx) <= 2),
        "claudeCode" => x.abs_diff(cx) == y.abs_diff(cy) && x.abs_diff(cx) <= 2,
        "pi" => (x == cx.saturating_sub(2) || x == cx.saturating_add(2)) && y.abs_diff(cy) <= 2,
        _ => y == cy && x.abs_diff(cx) <= 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::{HarnessWire, HostKindWire, HostWire, LocatorCapability};
    fn stove(id: &str) -> StoveWire {
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
            project_label: id.into(),
            project_root_display: "Project".into(),
            task_title: None,
            current_action: None,
            next_action: None,
            elapsed_ms: None,
            state: StoveStateWire::Cooking,
            progress: None,
            locator_capability: LocatorCapability::Available,
            retained_completion: false,
            pinned: false,
        }
    }
    fn snapshot(ids: &[&str]) -> StoveSnapshot {
        StoveSnapshot {
            revision: 1,
            attention_order: ids.iter().map(|id| (*id).into()).collect(),
            stoves: ids.iter().map(|id| stove(id)).collect(),
        }
    }
    #[test]
    fn empty_or_zero_slots_restore_static_mark() {
        assert!(presentation(&snapshot(&["a"]), 0, &[]).is_none());
        assert!(presentation(&snapshot(&[]), 3, &[]).is_none());
    }
    #[test]
    fn render_is_nonblank_for_one_three_and_eight() {
        for count in [1, 3, 8] {
            let ids = (0..count).map(|i| format!("s{i}")).collect::<Vec<_>>();
            let refs = ids.iter().map(String::as_str).collect::<Vec<_>>();
            let p = presentation(&snapshot(&refs), 8, &[]).unwrap();
            assert_eq!(p.slots.len(), count);
            assert!(p
                .image
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel[3] != 0));
        }
    }
    #[test]
    fn stable_slots_replace_only_lowest_removed_selection() {
        let old = presentation(&snapshot(&["a", "b", "c"]), 3, &[]).unwrap();
        assert_eq!(
            selected_stove_ids(&snapshot(&["d", "a", "b", "c"]), 3, &old.slots),
            ["a", "b", "d"]
        );
        assert_eq!(
            selected_stove_ids(&snapshot(&["a", "c", "d"]), 3, &old.slots),
            ["a", "d", "c"]
        );
    }
    #[test]
    fn all_stove_menu_is_canonical_and_stable() {
        let input = snapshot(&["c", "a", "b"]);
        let menu = all_stove_menu(&input);
        assert_eq!(
            menu.iter()
                .map(|item| item.stove_id.as_str())
                .collect::<Vec<_>>(),
            ["c", "a", "b"]
        );
        assert_eq!(menu[0].menu_id, all_stove_menu(&input)[0].menu_id);
        assert_ne!(menu[0].menu_id, menu[1].menu_id);
    }
    #[test]
    fn scaled_hit_testing_and_gaps_are_exact() {
        let p = presentation(&snapshot(&["a", "b"]), 2, &[]).unwrap();
        let first = &p.slots[0];
        assert_eq!(
            hit_test(
                &p.slots,
                normalize_status_x(
                    f64::from(first.start_x),
                    f64::from(p.image.width),
                    p.image.width
                )
                .unwrap()
            ),
            Some("a".into())
        );
        assert_eq!(
            hit_test(
                &p.slots,
                normalize_status_x(
                    f64::from(first.end_x) * 2.0,
                    f64::from(p.image.width) * 2.0,
                    p.image.width
                )
                .unwrap()
            ),
            None
        );
        assert_eq!(normalize_status_x(50.0, 100.0, 50), Some(25.0));
    }
    #[test]
    fn stale_slots_and_menu_targets_do_nothing() {
        let state = StatusStovesState::default();
        let p = presentation(&snapshot(&["a"]), 1, &[]).unwrap();
        state.commit_presentation(1, Some(&p));
        state.commit_menu(1, &all_stove_menu(&snapshot(&["a"])));
        assert_eq!(
            state.stove_at_status_x(3.0, f64::from(p.image.width)),
            Some("a".into())
        );
        state.commit_presentation(2, None);
        state.commit_menu(2, &[]);
        assert!(state.stove_at_status_x(3.0, 22.0).is_none());
        assert!(state.menu_target("missing").is_none());
    }
    #[test]
    fn accessibility_label_uses_only_safe_state_metadata() {
        assert_eq!(accessibility_label(&snapshot(&[]), 3), "Cookbench");
        assert_eq!(
            accessibility_label(&snapshot(&["a", "b"]), 1),
            "Cookbench: 1 Stove, Cooking"
        );
    }

    #[test]
    fn accessibility_template_uses_natural_locale_count_forms() {
        let empty = snapshot(&[]);
        let one = snapshot(&["a"]);
        let two = snapshot(&["a", "b"]);
        assert_eq!(
            accessibility_label_for_locale(&empty, 3, AppLocale::En),
            "Cookbench"
        );
        assert_eq!(
            accessibility_label_for_locale(&one, 1, AppLocale::En),
            "Cookbench: 1 Stove, Cooking"
        );
        assert_eq!(
            accessibility_label_for_locale(&two, 2, AppLocale::En),
            "Cookbench: 2 Stoves, Cooking, Cooking"
        );
        assert_eq!(
            accessibility_label_for_locale(&one, 1, AppLocale::ZhCn),
            "Cookbench：1 个炉灶，运行中"
        );
        assert_eq!(
            accessibility_label_for_locale(&one, 1, AppLocale::Ja),
            "Cookbench: 1 台のストーブ、実行中"
        );
        assert_eq!(
            accessibility_label_for_locale(&one, 1, AppLocale::Ko),
            "Cookbench: 스토브 1개, 진행 중"
        );
    }

    #[test]
    fn stale_commits_cannot_replace_a_newer_snapshot() {
        let state = StatusStovesState::default();
        let current = presentation(&snapshot(&["a"]), 1, &[]).unwrap();
        assert!(state.commit_presentation(2, Some(&current)));
        assert!(!state.commit_presentation(1, None));
        assert_eq!(
            state.stove_at_status_x(3.0, f64::from(current.image.width)),
            Some("a".into())
        );
    }

    #[test]
    fn labels_remove_formatting_and_normalize_whitespace() {
        let mut input = snapshot(&["a"]);
        input.stoves[0].project_label = "  alpha\u{202e}\n beta\u{200b}  ".into();
        assert_eq!(
            all_stove_menu_for_locale(&input, AppLocale::En)[0].label,
            "Cooking: alpha beta"
        );
        input.stoves[0].project_label = "\u{202e}".into();
        assert_eq!(
            all_stove_menu_for_locale(&input, AppLocale::Ja)[0].label,
            "実行中: プロジェクト"
        );
    }

    #[test]
    fn harness_marks_are_distinct() {
        assert!(harness_mark("codex", 11, 9, 11, 11));
        assert!(!harness_mark("claudeCode", 11, 9, 11, 11));
        assert!(harness_mark("claudeCode", 9, 9, 11, 11));
        assert!(harness_mark("pi", 9, 11, 11, 11));
    }
}
