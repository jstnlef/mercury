pub(crate) struct Stream {
    sequence_num: u16,
}

impl Stream {
    pub(crate) fn new() -> Self {
        Self { sequence_num: 0 }
    }
}
