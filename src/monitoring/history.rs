use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct HistoryBuffer {
    entries: VecDeque<f32>,
}

impl HistoryBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(capacity.max(2)),
        }
    }

    pub fn push(&mut self, value: f32) {
        if self.entries.len() >= self.entries.capacity() {
            self.entries.pop_front();
        }
        self.entries.push_back(value);
    }

    pub fn iter(&self) -> impl Iterator<Item = &f32> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn value_at_x(&self, rect: egui::Rect, x: f32) -> Option<f32> {
        if self.entries.is_empty() || rect.width() <= 0.0 {
            return None;
        }
        let ratio = ((x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        let index = (ratio * (self.entries.len() - 1) as f32).round() as usize;
        self.entries.get(index).copied()
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
