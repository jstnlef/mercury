#[derive(Clone)]
pub struct OrderedStream {
    sequence_num: u16,
}

impl OrderedStream {
    pub fn new() -> Self {
        Self { sequence_num: 0 }
    }
}

#[derive(Clone)]
pub struct SequencedStream {
    sequence_num: u16,
}

impl SequencedStream {
    pub fn new() -> Self {
        Self { sequence_num: 0 }
    }
}
