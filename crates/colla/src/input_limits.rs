#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputLimits {
    pub max_depth: usize,
    pub max_value_nodes: usize,
    pub max_change_nodes: usize,
    pub max_container_len: usize,
    pub max_string_bytes: usize,
    pub max_sequence_ops: usize,
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
