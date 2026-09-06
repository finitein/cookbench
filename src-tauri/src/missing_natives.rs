//! Post-discovery reconciliation for tracked local sessions whose native
//! files are gone.
//!
//! Native files remain authoritative. This helper never invents Cooked and
//! never treats an SSH transport loss as a missing local file.

use std::{collections::HashSet, path::Path};

use cookbench_core::{
    domain::{HostKind, StoveIdentity, StoveState},
    persistence::SessionRecord,
};

/// Returns non-pinned local tracked records whose `native_locator` is set
/// and no longer names a regular file.
///
/// SSH records are excluded so a disconnect stays Disconnected. Pinned
/// records stay until the user unpins (pin paths re-register old files).
/// Records without a locator, or whose last state is Cooked, are skipped:
/// missing activity is never Cooked, and Cooked persists until user clear.
pub fn filter_tracked_missing_natives<'a, I, F>(
    tracked: I,
    pinned_locators: &HashSet<StoveIdentity>,
    native_exists: F,
) -> Vec<SessionRecord>
where
    I: IntoIterator<Item = &'a SessionRecord>,
    F: Fn(&str) -> bool,
{
    tracked
        .into_iter()
        .filter(|record| {
            record.is_valid()
                && record.locator.host.kind == HostKind::Local
                && record.last_state != StoveState::Cooked
                && !pinned_locators.contains(&record.locator)
                && record
                    .native_locator
                    .as_ref()
                    .is_some_and(|path| !native_exists(path))
        })
        .cloned()
        .collect()
}

pub fn local_native_file_exists(path: &str) -> bool {
    Path::new(path).is_file()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use cookbench_core::{
        domain::{HarnessId, HostIdentity, StoveIdentity, StoveState},
        persistence::{RetainedStovePresentation, SessionRecord},
    };

    use super::filter_tracked_missing_natives;

    fn record(
        host: HostIdentity,
        session: &str,
        native_locator: Option<&str>,
        last_state: StoveState,
    ) -> SessionRecord {
        SessionRecord::new(
            StoveIdentity::new(host, HarnessId::Codex, session),
            native_locator.map(str::to_owned),
            1_700_000_000_000,
            RetainedStovePresentation::new("demo", "/tmp/demo"),
            last_state,
        )
        .expect("valid session record")
    }

    fn local(session: &str, path: &str) -> SessionRecord {
        record(
            HostIdentity::local("local"),
            session,
            Some(path),
            StoveState::NeedsHuman,
        )
    }

    #[test]
    fn keeps_existing_local_files_and_selects_missing_ones() {
        let present = local("present", "/safe/present.jsonl");
        let missing = local("missing", "/safe/missing.jsonl");
        let selected =
            filter_tracked_missing_natives([&present, &missing], &HashSet::new(), |path| {
                path == "/safe/present.jsonl"
            });
        assert_eq!(
            selected
                .iter()
                .map(|record| record.locator.native_session_id.as_str())
                .collect::<Vec<_>>(),
            vec!["missing"]
        );
    }

    #[test]
    fn skips_ssh_pinned_cooked_and_locatorless_records() {
        let ssh = record(
            HostIdentity::ssh("jump"),
            "ssh-session",
            Some("/safe/ssh.jsonl"),
            StoveState::Disconnected,
        );
        let pinned = local("pinned", "/safe/pinned.jsonl");
        let cooked = record(
            HostIdentity::local("local"),
            "cooked",
            Some("/safe/cooked.jsonl"),
            StoveState::Cooked,
        );
        let locatorless = record(
            HostIdentity::local("local"),
            "locatorless",
            None,
            StoveState::Failed,
        );
        let gone = local("gone", "/safe/gone.jsonl");
        let pinned_locators = HashSet::from([pinned.locator.clone()]);
        let selected = filter_tracked_missing_natives(
            [&ssh, &pinned, &cooked, &locatorless, &gone],
            &pinned_locators,
            |_| false,
        );
        assert_eq!(
            selected
                .iter()
                .map(|record| record.locator.native_session_id.as_str())
                .collect::<Vec<_>>(),
            vec!["gone"]
        );
    }

    #[test]
    fn skips_invalid_native_locators() {
        let mut invalid = local("invalid", "/safe/invalid.jsonl");
        invalid.native_locator = Some("bad\u{0001}".into());
        let selected = filter_tracked_missing_natives([&invalid], &HashSet::new(), |_| false);
        assert!(selected.is_empty());
    }
}
