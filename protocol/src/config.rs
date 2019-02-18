pub struct Config {
    /// The maximum number of fragments a particular payload will get split into.
    max_fragments: u8,
    /// This is the size of a fragment. If a payload is too large it needs to be split in fragments
    /// over the wire.
    /// Recommended value: +- 1450 (1500 is the default MTU)
    fragment_size_bytes: usize,
}

impl Config {
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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_fragments: 16,
            fragment_size_bytes: 1450,
        }
    }
}
