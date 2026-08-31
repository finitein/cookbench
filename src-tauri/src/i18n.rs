//! Small native translations for UI that never passes through the webview.
//!
//! The frontend owns application copy. This module deliberately covers only
//! tray labels, native window titles, and operating-system notification text.

use std::sync::RwLock;

use cookbench_core::{notifications::NotificationEventKind, persistence::AppLocale};

/// Runtime locale used by native surfaces. The webview synchronizes its
/// resolved `navigator.language` value when the persisted preference is
/// `System`, because GUI apps cannot rely on POSIX locale environment
/// variables being present on macOS or Windows.
#[derive(Debug)]
pub struct NativeLocaleState(RwLock<AppLocale>);

impl Default for NativeLocaleState {
    fn default() -> Self {
        Self(RwLock::new(AppLocale::En))
    }
}

impl NativeLocaleState {
    pub fn current(&self) -> AppLocale {
        *self.0.read().expect("native locale lock poisoned")
    }

    pub fn set_preference(&self, preference: AppLocale) -> AppLocale {
        self.set_resolved(resolve_locale(preference))
            .expect("resolved locale cannot be System")
    }

    pub fn set_resolved(&self, locale: AppLocale) -> Result<AppLocale, String> {
        if locale == AppLocale::System {
            return Err("resolved native locale cannot be System".to_owned());
        }
        *self.0.write().expect("native locale lock poisoned") = locale;
        Ok(locale)
    }
}

pub fn resolve_locale(preference: AppLocale) -> AppLocale {
    if preference != AppLocale::System {
        return preference;
    }
    let system_locale = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .find_map(|name| std::env::var(name).ok().filter(|value| !value.is_empty()));
    resolve_system_locale(system_locale.as_deref())
}

pub fn resolve_system_locale(system_locale: Option<&str>) -> AppLocale {
    let normalized = system_locale.unwrap_or_default().to_ascii_lowercase();
    if normalized.starts_with("zh") {
        AppLocale::ZhCn
    } else if normalized.starts_with("ja") {
        AppLocale::Ja
    } else if normalized.starts_with("ko") {
        AppLocale::Ko
    } else {
        AppLocale::En
    }
}

pub fn settings_window_title(preference: AppLocale) -> &'static str {
    match resolve_locale(preference) {
        AppLocale::ZhCn => "Cookbench 设置",
        AppLocale::Ja => "Cookbench 設定",
        AppLocale::Ko => "Cookbench 설정",
        AppLocale::System | AppLocale::En => "Cookbench Settings",
    }
}

pub fn tray_menu_titles(preference: AppLocale) -> [&'static str; 4] {
    match resolve_locale(preference) {
        AppLocale::ZhCn => ["显示总 Bar", "隐藏总 Bar", "打开设置", "退出"],
        AppLocale::Ja => ["Bar を表示", "Bar を隠す", "設定を開く", "終了"],
        AppLocale::Ko => ["Bar 표시", "Bar 숨기기", "설정 열기", "종료"],
        AppLocale::System | AppLocale::En => ["Show Bar", "Hide Bar", "Open Settings", "Quit"],
    }
}

pub fn notification_event_label(
    preference: AppLocale,
    event: NotificationEventKind,
) -> &'static str {
    match resolve_locale(preference) {
        AppLocale::ZhCn => match event {
            NotificationEventKind::SessionAppeared => "发现新会话",
            NotificationEventKind::CookingStarted => "开始执行",
            NotificationEventKind::PhaseChanged => "阶段已变化",
            NotificationEventKind::NeedsHuman => "需要人工处理",
            NotificationEventKind::ProgressMilestone => "进度已更新",
            NotificationEventKind::Cooked => "任务已完成",
            NotificationEventKind::Failed => "任务失败",
            NotificationEventKind::Disconnected => "连接已断开",
            NotificationEventKind::ConnectionRestored => "连接已恢复",
            NotificationEventKind::StoveCleared => "Stove 已清除",
        },
        AppLocale::Ja => match event {
            NotificationEventKind::SessionAppeared => "セッションを検出",
            NotificationEventKind::CookingStarted => "実行を開始",
            NotificationEventKind::PhaseChanged => "フェーズが変更されました",
            NotificationEventKind::NeedsHuman => "確認が必要です",
            NotificationEventKind::ProgressMilestone => "進捗が更新されました",
            NotificationEventKind::Cooked => "タスクが完了しました",
            NotificationEventKind::Failed => "タスクが失敗しました",
            NotificationEventKind::Disconnected => "接続が切れました",
            NotificationEventKind::ConnectionRestored => "接続が復旧しました",
            NotificationEventKind::StoveCleared => "Stove を消去しました",
        },
        AppLocale::Ko => match event {
            NotificationEventKind::SessionAppeared => "세션 감지됨",
            NotificationEventKind::CookingStarted => "실행 시작됨",
            NotificationEventKind::PhaseChanged => "단계가 변경됨",
            NotificationEventKind::NeedsHuman => "확인이 필요함",
            NotificationEventKind::ProgressMilestone => "진행률 업데이트됨",
            NotificationEventKind::Cooked => "작업 완료됨",
            NotificationEventKind::Failed => "작업 실패함",
            NotificationEventKind::Disconnected => "연결 끊김",
            NotificationEventKind::ConnectionRestored => "연결 복구됨",
            NotificationEventKind::StoveCleared => "Stove 삭제됨",
        },
        AppLocale::System | AppLocale::En => match event {
            NotificationEventKind::SessionAppeared => "Session appeared",
            NotificationEventKind::CookingStarted => "Cooking started",
            NotificationEventKind::PhaseChanged => "Phase changed",
            NotificationEventKind::NeedsHuman => "Needs human",
            NotificationEventKind::ProgressMilestone => "Progress updated",
            NotificationEventKind::Cooked => "Cooked",
            NotificationEventKind::Failed => "Failed",
            NotificationEventKind::Disconnected => "Disconnected",
            NotificationEventKind::ConnectionRestored => "Connection restored",
            NotificationEventKind::StoveCleared => "Stove cleared",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_locale_never_depends_on_the_system() {
        assert_eq!(resolve_locale(AppLocale::ZhCn), AppLocale::ZhCn);
        assert_eq!(resolve_locale(AppLocale::Ja), AppLocale::Ja);
    }

    #[test]
    fn system_locale_maps_supported_languages_and_falls_back_to_english() {
        assert_eq!(resolve_system_locale(Some("zh_CN.UTF-8")), AppLocale::ZhCn);
        assert_eq!(resolve_system_locale(Some("ja-JP")), AppLocale::Ja);
        assert_eq!(resolve_system_locale(Some("ko_KR")), AppLocale::Ko);
        assert_eq!(resolve_system_locale(Some("fr_FR.UTF-8")), AppLocale::En);
        assert_eq!(resolve_system_locale(None), AppLocale::En);
    }

    #[test]
    fn native_copy_uses_the_selected_language() {
        assert_eq!(settings_window_title(AppLocale::ZhCn), "Cookbench 设置");
        assert_eq!(tray_menu_titles(AppLocale::Ja)[2], "設定を開く");
        assert_eq!(
            notification_event_label(AppLocale::Ko, NotificationEventKind::Cooked),
            "작업 완료됨"
        );
    }

    #[test]
    fn webview_can_resolve_system_locale_for_every_native_surface() {
        let state = NativeLocaleState::default();
        state.set_resolved(AppLocale::ZhCn).unwrap();

        let locale = state.current();
        assert_eq!(settings_window_title(locale), "Cookbench 设置");
        assert_eq!(tray_menu_titles(locale)[2], "打开设置");
        assert_eq!(
            notification_event_label(locale, NotificationEventKind::Cooked),
            "任务已完成"
        );
    }
}
