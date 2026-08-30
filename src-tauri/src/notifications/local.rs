use std::{
    collections::BTreeMap,
    process::{Child, Command},
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

use cookbench_core::{
    notifications::NotificationEventKind, persistence::LocalNotificationPreferences,
};
use serde::{Deserialize, Serialize};
use tauri::{plugin::PermissionState, Emitter, Manager, Runtime, UserAttentionType};
use tauri_plugin_notification::NotificationExt;

pub const LOCAL_ALERT_EVENT: &str = "cookbench://local-alert";
const DUPLICATE_WINDOW_MS: u64 = 1_000;
const MAX_RECENT_ALERTS: usize = 256;
const SOUND_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LocalAlertChannel {
    Sound,
    SystemBanner,
    BarFlash,
    SystemAttention,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LocalAlertResult {
    Delivered,
    Queued,
    PermissionDenied,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAlertPayload {
    pub stove_id: String,
    pub project: String,
    pub event: NotificationEventKind,
}

impl LocalAlertPayload {
    pub fn new(
        stove_id: impl AsRef<str>,
        project: impl AsRef<str>,
        event: NotificationEventKind,
    ) -> Self {
        Self {
            stove_id: bounded(stove_id.as_ref(), 128),
            project: bounded(project.as_ref(), 128),
            event,
        }
    }

    pub fn state_label(&self) -> &'static str {
        match self.event {
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
        }
    }

    fn banner_body(&self) -> String {
        format!("{} · {}", self.project, self.state_label())
    }
}

pub trait LocalAlertEffects {
    fn play_sound(&self) -> LocalAlertResult;
    fn show_banner(&self, payload: &LocalAlertPayload) -> LocalAlertResult;
    fn flash_stove(&self, payload: &LocalAlertPayload) -> LocalAlertResult;
    fn request_attention(&self) -> LocalAlertResult;
}

#[derive(Default)]
pub struct LocalAlertDispatcher {
    recent: Mutex<BTreeMap<(String, NotificationEventKind), u64>>,
}

impl LocalAlertDispatcher {
    pub fn dispatch(
        &self,
        preferences: &LocalNotificationPreferences,
        payload: &LocalAlertPayload,
        now_ms: u64,
        effects: &impl LocalAlertEffects,
    ) -> Vec<(LocalAlertChannel, LocalAlertResult)> {
        if !preferences.events.contains(&payload.event) || self.is_duplicate(payload, now_ms) {
            return Vec::new();
        }

        let mut outcomes = Vec::with_capacity(4);
        if preferences.sound {
            outcomes.push((LocalAlertChannel::Sound, effects.play_sound()));
        }
        if preferences.system_banner {
            outcomes.push((
                LocalAlertChannel::SystemBanner,
                effects.show_banner(payload),
            ));
        }
        if preferences.bar_flash {
            outcomes.push((LocalAlertChannel::BarFlash, effects.flash_stove(payload)));
        }
        if preferences.system_attention {
            outcomes.push((
                LocalAlertChannel::SystemAttention,
                effects.request_attention(),
            ));
        }
        outcomes
    }

    pub fn test_channel(
        &self,
        channel: LocalAlertChannel,
        payload: &LocalAlertPayload,
        effects: &impl LocalAlertEffects,
    ) -> LocalAlertResult {
        match channel {
            LocalAlertChannel::Sound => effects.play_sound(),
            LocalAlertChannel::SystemBanner => effects.show_banner(payload),
            LocalAlertChannel::BarFlash => effects.flash_stove(payload),
            LocalAlertChannel::SystemAttention => effects.request_attention(),
        }
    }

    fn is_duplicate(&self, payload: &LocalAlertPayload, now_ms: u64) -> bool {
        let mut recent = self.recent.lock().expect("local alert lock poisoned");
        let key = (payload.stove_id.clone(), payload.event);
        if recent
            .get(&key)
            .is_some_and(|previous| now_ms.saturating_sub(*previous) < DUPLICATE_WINDOW_MS)
        {
            return true;
        }
        recent.insert(key, now_ms);
        recent.retain(|_, timestamp| now_ms.saturating_sub(*timestamp) <= DUPLICATE_WINDOW_MS);
        while recent.len() > MAX_RECENT_ALERTS {
            let Some(oldest) = recent
                .iter()
                .min_by_key(|(_, timestamp)| **timestamp)
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            recent.remove(&oldest);
        }
        false
    }
}

pub struct TauriLocalAlertEffects<'a, R: Runtime> {
    app: &'a tauri::AppHandle<R>,
}

impl<'a, R: Runtime> TauriLocalAlertEffects<'a, R> {
    pub const fn new(app: &'a tauri::AppHandle<R>) -> Self {
        Self { app }
    }

    pub fn request_banner_permission(&self) -> LocalAlertResult {
        match self.app.notification().request_permission() {
            Ok(PermissionState::Granted) => LocalAlertResult::Delivered,
            Ok(PermissionState::Denied) => LocalAlertResult::PermissionDenied,
            Ok(PermissionState::Prompt | PermissionState::PromptWithRationale) | Err(_) => {
                LocalAlertResult::Unavailable
            }
        }
    }
}

impl<R: Runtime> LocalAlertEffects for TauriLocalAlertEffects<'_, R> {
    fn play_sound(&self) -> LocalAlertResult {
        play_system_sound()
    }

    fn show_banner(&self, payload: &LocalAlertPayload) -> LocalAlertResult {
        match self.app.notification().permission_state() {
            Ok(PermissionState::Granted) => self
                .app
                .notification()
                .builder()
                .title("Cookbench")
                .body(payload.banner_body())
                .show()
                .map_or(LocalAlertResult::Unavailable, |_| {
                    LocalAlertResult::Delivered
                }),
            Ok(PermissionState::Denied) => LocalAlertResult::PermissionDenied,
            Ok(PermissionState::Prompt | PermissionState::PromptWithRationale) | Err(_) => {
                LocalAlertResult::Unavailable
            }
        }
    }

    fn flash_stove(&self, payload: &LocalAlertPayload) -> LocalAlertResult {
        self.app
            .emit(LOCAL_ALERT_EVENT, payload)
            .map_or(LocalAlertResult::Unavailable, |_| {
                LocalAlertResult::Delivered
            })
    }

    fn request_attention(&self) -> LocalAlertResult {
        let Some(window) = self.app.get_webview_window("main") else {
            return LocalAlertResult::Unavailable;
        };
        window
            .request_user_attention(Some(UserAttentionType::Informational))
            .map_or(LocalAlertResult::Unavailable, |_| {
                LocalAlertResult::Delivered
            })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SystemSoundCommand {
    pub program: &'static str,
    pub args: &'static [&'static str],
}

impl SystemSoundCommand {
    pub fn for_platform(platform: &str) -> Option<Self> {
        sound_commands_for_platform(platform).first().copied()
    }
}

fn sound_commands_for_platform(platform: &str) -> &'static [SystemSoundCommand] {
    const MACOS: &[SystemSoundCommand] = &[SystemSoundCommand {
        program: "/usr/bin/osascript",
        args: &["-e", "beep"],
    }];
    const WINDOWS: &[SystemSoundCommand] = &[SystemSoundCommand {
        program: "powershell.exe",
        args: &[
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[System.Media.SystemSounds]::Asterisk.Play()",
        ],
    }];
    const LINUX: &[SystemSoundCommand] = &[
        SystemSoundCommand {
            program: "canberra-gtk-play",
            args: &["--id", "message"],
        },
        SystemSoundCommand {
            program: "paplay",
            args: &["/usr/share/sounds/freedesktop/stereo/message.oga"],
        },
    ];
    match platform {
        "macos" => MACOS,
        "windows" => WINDOWS,
        "linux" => LINUX,
        _ => &[],
    }
}

fn play_system_sound() -> LocalAlertResult {
    let commands = sound_commands_for_platform(std::env::consts::OS);
    if commands.is_empty() {
        return LocalAlertResult::Unavailable;
    }

    thread::Builder::new()
        .name("cookbench-local-sound".to_owned())
        .spawn(move || {
            first_successful_sound(commands, run_system_sound_command);
        })
        .map_or(LocalAlertResult::Unavailable, |_| LocalAlertResult::Queued)
}

fn first_successful_sound(
    commands: &[SystemSoundCommand],
    mut run: impl FnMut(SystemSoundCommand) -> bool,
) -> bool {
    commands.iter().copied().any(&mut run)
}

fn run_system_sound_command(sound: SystemSoundCommand) -> bool {
    let mut command = Command::new(sound.program);
    command.args(sound.args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    command
        .spawn()
        .is_ok_and(|child| wait_bounded(child, SOUND_TIMEOUT))
}

fn wait_bounded(mut child: Child, timeout: Duration) -> bool {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if started.elapsed() < timeout => thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }
}

fn bounded(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::{first_successful_sound, SystemSoundCommand};

    #[test]
    fn sound_candidates_continue_after_a_nonzero_exit() {
        let commands = [
            SystemSoundCommand {
                program: "first",
                args: &[],
            },
            SystemSoundCommand {
                program: "second",
                args: &[],
            },
        ];
        let mut attempted = Vec::new();

        let delivered = first_successful_sound(&commands, |command| {
            attempted.push(command.program);
            command.program == "second"
        });

        assert!(delivered);
        assert_eq!(attempted, ["first", "second"]);
    }
}
