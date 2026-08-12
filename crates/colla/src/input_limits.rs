/// Receiver-defined resource limits for untrusted Value and Change input.
///
/// Limits do not define the maximum valid in-memory value and are not applied
/// to algebra results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputLimits {
    /// Maximum recursive depth of Value and Change nodes.
    pub max_depth: usize,
    /// Maximum number of Value nodes in one input.
    pub max_value_nodes: usize,
    /// Maximum number of Change nodes in one input.
    pub max_change_nodes: usize,
    /// Maximum number of entries in one container.
    pub max_container_len: usize,
    /// Maximum UTF-8 byte length of one string.
    pub max_string_bytes: usize,
    /// Maximum number of operations in one sequence Change.
    pub max_sequence_ops: usize,
    /// Maximum logical input or output length of a sequence.
    pub max_sequence_len: usize,
}

impl Default for InputLimits {
    fn default() -> Self {
        Self {
            max_depth: 128,
            max_value_nodes: 1_000_000,
            max_change_nodes: 1_000_000,
            max_container_len: 1_000_000,
            max_string_bytes: 16 * 1024 * 1024,
            max_sequence_ops: 1_000_000,
            max_sequence_len: 1_000_000,
        }
    }
}
