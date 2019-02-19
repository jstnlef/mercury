#[derive(Clone)]
pub struct Config {
    /// Number of ordered streams available
    /// default: 1
    ordered_streams_size: usize,
    /// Number of sequenced streams available
    /// default: 1
    sequenced_streams_size: usize,
    /// The maximum number of fragments a particular payload will get split into.
    /// default: 16
    max_fragments: u8,
    /// This is the size of a fragment. If a payload is too large it needs to be split in fragments
    /// over the wire.
    /// default: 1450
    fragment_size_bytes: usize,
}

impl Config {
    #[inline]
    pub const fn ordered_streams_size(&self) -> usize {
        self.ordered_streams_size
    }

    #[inline]
    pub const fn sequenced_streams_size(&self) -> usize {
        self.sequenced_streams_size
    }

    /// Calculated value based on the maximum number of fragments and the fragment size.
    #[inline]
    pub const fn max_payload_size_bytes(&self) -> usize {
        self.max_fragments as usize + self.fragment_size_bytes
    }

    pub fn with_max_fragments(mut self, max_fragments: u8) -> Self {
        self.max_fragments = max_fragments;
        self
    }

    pub fn with_fragment_size_bytes(mut self, fragment_size_bytes: usize) -> Self {
        self.fragment_size_bytes = fragment_size_bytes;
        self
    }

    pub fn with_ordered_streams_size(mut self, ordered_streams_size: usize) -> Self {
        self.ordered_streams_size = ordered_streams_size;
        self
    }
    pub fn with_sequenced_streams_size(mut self, sequenced_streams_size: usize) -> Self {
        self.sequenced_streams_size = sequenced_streams_size;
        self
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ordered_streams_size: 1,
            sequenced_streams_size: 1,
            max_fragments: 16,
            fragment_size_bytes: 1450,
        }
    }
}
