/// TODO: add a description
pub struct SequenceBuffer<T>
where
    T: Clone + Default,
{
    sequence_num: u16,
    sequence_nums: Box<[u32]>,
    entries: Box<[T]>,
}

impl<T> SequenceBuffer<T>
where
    T: Clone + Default,
{
    pub fn new(size: u16) -> Self {
        Self {
            sequence_num: 0,
            sequence_nums: vec![u32::max_value(); size as usize].into_boxed_slice(),
            entries: vec![T::default(); size as usize].into_boxed_slice(),
        }
    }

    /// Inserts an entry into the sequence buffer by sequence_num.
    pub fn insert(&mut self, sequence_num: u16, entry: T) -> Option<&mut T> {
        if sequence_num_less_than(
            sequence_num,
            self.sequence_num.wrapping_sub(self.entries.len() as u16),
        ) {
            return None;
        }

        let sequence_num_plus_1 = sequence_num.wrapping_add(1);
        if sequence_num_greater_than(sequence_num_plus_1, self.sequence_num) {
            self.remove_range(self.sequence_num, sequence_num);
            self.sequence_num = sequence_num_plus_1;
        }

        let index = self.index(sequence_num);
        self.sequence_nums[index] = sequence_num as u32;
        self.entries[index] = entry;
        Some(&mut self.entries[index])
    }

    /// Removes a particular entry from the sequence buffer by sequence_num.
    pub fn remove(&mut self, sequence_num: u16) {
        let index = self.index(sequence_num);
        self.sequence_nums[index] = u32::max_value();
        self.entries[index] = T::default();
    }

    /// A particular entry slot is available if the value of the sequence number at the index
    /// is equal to u32 max
    pub fn available(&self, sequence_num: u16) -> bool {
        self.sequence_nums[self.index(sequence_num)] == u32::max_value()
    }

    /// Check to see if the given sequence_num has been stored in the buffer.
    pub fn exists(&self, sequence_num: u16) -> bool {
        self.sequence_nums[self.index(sequence_num)] == sequence_num as u32
    }

    /// Reset the sequence buffer to its initial state
    pub fn reset(&mut self) {
        self.sequence_num = 0;
        for sequence_num in self.sequence_nums.iter_mut() {
            *sequence_num = u32::max_value();
        }
        for entry in self.entries.iter_mut() {
            *entry = T::default();
        }
    }

    // Removes a range of entries from the sequence buffer
    fn remove_range(&mut self, start_sequence: u16, end_sequence: u16) {
        let start_sequence = start_sequence as u32;
        let mut end_sequence = end_sequence as u32;

        if end_sequence < start_sequence {
            end_sequence += u16::max_value() as u32 + 1;
        }

        if end_sequence - start_sequence < self.entries.len() as u32 {
            for sequence_num in start_sequence..end_sequence {
                self.remove((sequence_num % u16::max_value() as u32) as u16);
            }
        } else {
            for sequence_num in 0..self.entries.len() as u16 {
                self.remove(sequence_num);
            }
        }
    }

    fn index(&self, sequence_num: u16) -> usize {
        sequence_num as usize % self.entries.len()
    }
}

const HALF_U16_MAX: u16 = u16::max_value() / 2 + 1;

#[inline]
fn sequence_num_greater_than(s1: u16, s2: u16) -> bool {
    ((s1 > s2) && (s1 - s2 <= HALF_U16_MAX)) || ((s1 < s2) && (s2 - s1 > HALF_U16_MAX))
}

#[inline]
fn sequence_num_less_than(s1: u16, s2: u16) -> bool {
    sequence_num_greater_than(s2, s1)
}

#[cfg(test)]
mod tests {
    use super::{sequence_num_greater_than, SequenceBuffer, HALF_U16_MAX};

    #[derive(Clone, Default)]
    struct DataStub;

    // TODO: Add more tests. Especially around edge cases.

    // This also tests to ensure that the wrapping case is handled successfully.
    // e.g. 0 > u16::max_value()
    #[test]
    fn test_sequence_num_greater_than() {
        let range_max: u32 = 66000;
        for i in 0..range_max {
            let first = (i % u16::max_value() as u32) as u16;
            let next = ((i + 1) % u16::max_value() as u32) as u16;
            assert!(sequence_num_greater_than(next, first));
            assert!(!sequence_num_greater_than(first, next));
        }
    }

    // Around the halfway point, we're going to start assuming that the smaller numbers are more
    // recent.
    #[test]
    fn test_sequence_num_greater_than_with_large_delta() {
        assert!(!sequence_num_greater_than(0, HALF_U16_MAX));
        assert!(sequence_num_greater_than(0, HALF_U16_MAX + 1));
    }

    #[test]
    fn test_insert_into_fragment_buffer() {
        let mut fragment_buffer = SequenceBuffer::new(4);
        fragment_buffer.insert(1, DataStub);
        assert!(fragment_buffer.exists(1));
        assert!(!fragment_buffer.available(1));
        assert!(fragment_buffer.available(2));
    }

    #[test]
    fn test_remove_from_fragment_buffer() {
        let mut fragment_buffer = SequenceBuffer::new(4);
        fragment_buffer.insert(1, DataStub);
        fragment_buffer.remove(1);
        assert!(!fragment_buffer.exists(1));
        assert!(fragment_buffer.available(1));
    }
}
