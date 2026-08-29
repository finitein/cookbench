/// Features an adapter can truthfully provide.
///
/// Each field is independent: for example, a file-only adapter can discover a
/// session and expose a locator without claiming it can stream watch events or
/// derive structured progress.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AdapterCapabilities {
    pub discovery: bool,
    pub watch_events: bool,
    pub structured_progress: bool,
    pub locator: bool,
    pub resume: bool,
}

impl AdapterCapabilities {
    pub const fn none() -> Self {
        Self {
            discovery: false,
            watch_events: false,
            structured_progress: false,
            locator: false,
            resume: false,
        }
    }
}
