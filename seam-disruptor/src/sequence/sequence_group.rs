use crate::sequence::Sequence;
use lockfree::set::{Removed, Set};
use std::sync::Arc;

pub struct SequenceGroup {
    set: Set<Arc<Sequence>>,
}

#[allow(dead_code)]
impl SequenceGroup {
    pub fn new() -> Self {
        Self { set: Set::new() }
    }

    pub fn add(&self, sequence: Arc<Sequence>) -> bool {
        self.set.insert(sequence).is_ok()
    }

    pub fn remove(&self, sequence: Arc<Sequence>) -> Option<Removed<Arc<Sequence>>> {
        self.set.remove(&sequence)
    }

    pub fn size(&self) -> usize {
        self.set.iter().count()
    }

    pub fn minimum_sequence(&self, minimum: u64) -> u64 {
        let mut minimum = minimum;

        for sequence in self.set.iter() {
            let value = sequence.get();
            minimum = std::cmp::min(minimum, value);
        }

        return minimum;
    }
}

#[cfg(test)]
mod tests {
    use crate::sequence::sequence_group::*;

    #[test]
    fn updates_atomically() {
        let sg = SequenceGroup::new();
        assert_eq!(0, sg.size());

        let s1 = Arc::new(Sequence::with_value(1));
        sg.add(s1);
        assert_eq!(1, sg.size());

        let s2 = Arc::new(Sequence::with_value(5));
        sg.add(s2);
        assert_eq!(2, sg.size());

        assert_eq!(1, sg.minimum_sequence(100));
    }
}
