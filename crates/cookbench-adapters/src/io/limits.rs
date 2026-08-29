/// Resource limits applied before any native session input reaches an adapter.
///
/// Harness-specific parsers can use the JSON limits when they inspect a record;
/// the tailer itself only needs the record and partial-buffer limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TailLimits {
    /// Maximum bytes in a complete JSONL record, excluding its newline.
    pub max_record_bytes: usize,
    /// Maximum bytes retained while awaiting a final newline.
    pub max_partial_bytes: usize,
    /// Maximum nesting an adapter parser may accept from one record.
    pub max_json_nesting: usize,
    /// Maximum bytes an adapter parser may retain for one JSON field.
    pub max_json_field_bytes: usize,
    /// Maximum bytes read from one file during a single `poll` call.
    pub max_read_bytes_per_poll: usize,
}

impl TailLimits {
    pub const MINIMUM: usize = 1;

    pub fn validate(self) -> bool {
        self.max_record_bytes >= Self::MINIMUM
            && self.max_partial_bytes >= Self::MINIMUM
            && self.max_json_nesting >= Self::MINIMUM
            && self.max_json_field_bytes >= Self::MINIMUM
            && self.max_read_bytes_per_poll >= Self::MINIMUM
    }
}

impl Default for TailLimits {
    fn default() -> Self {
        Self {
            max_record_bytes: 256 * 1024,
            max_partial_bytes: 256 * 1024,
            max_json_nesting: 64,
            max_json_field_bytes: 64 * 1024,
            max_read_bytes_per_poll: 1024 * 1024,
        }
    }
}
