use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct HistoryBuffer {
    capacity: usize,
    entries: VecDeque<f32>,
}

impl HistoryBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(2),
            entries: VecDeque::with_capacity(capacity.max(2)),
        }
    }

    pub fn push(&mut self, value: f32) {
        if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(value);
    }

    pub fn iter(&self) -> impl Iterator<Item = &f32> {
        self.entries.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_buffer_drops_oldest() {
        let mut buf = HistoryBuffer::new(3);
        for v in [1.0, 2.0, 3.0, 4.0, 5.0] {
            buf.push(v);
        }
        let collected: Vec<f32> = buf.iter().copied().collect();
        assert_eq!(collected, vec![3.0, 4.0, 5.0]);
    }
}
