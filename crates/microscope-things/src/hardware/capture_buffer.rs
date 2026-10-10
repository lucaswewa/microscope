//! Captures held in memory until they're saved, shared by every camera
//! driver (ADR-0018).
//!
//! As in OpenFlexure, each capture gets the next id, counting from 1. A
//! capture is added with a `buffer_max`, the most the buffer may then hold,
//! and the oldest go to make room. Taking a capture out removes it; taking
//! the latest, without an id, empties the buffer, so older captures can't
//! be saved out of order by mistake.

use std::collections::VecDeque;
use std::sync::Mutex;

use super::camera::Capture;

/// Captures in memory, by id.
#[derive(Debug, Default)]
pub struct CaptureBuffer {
    inner: Mutex<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    last_id: u64,
    /// Oldest first.
    captures: VecDeque<(u64, Capture)>,
}

impl CaptureBuffer {
    /// Adds `capture`, first dropping the oldest so that at most
    /// `buffer_max` (at least 1) are held with it. Returns its id.
    pub fn add(&self, capture: Capture, buffer_max: usize) -> u64 {
        let mut inner = self.inner.lock().expect("not poisoned");
        while inner.captures.len() >= buffer_max.max(1) {
            inner.captures.pop_front();
        }
        inner.last_id += 1;
        let id = inner.last_id;
        inner.captures.push_back((id, capture));
        id
    }

    /// Takes out the capture with `id`, or with none, the latest, emptying
    /// the buffer.
    pub fn take(&self, id: Option<u64>) -> Option<Capture> {
        let mut inner = self.inner.lock().expect("not poisoned");
        match id {
            Some(id) => {
                let index = inner.captures.iter().position(|(held, _)| *held == id)?;
                inner.captures.remove(index).map(|(_, capture)| capture)
            }
            None => {
                let latest = inner.captures.pop_back().map(|(_, capture)| capture);
                inner.captures.clear();
                latest
            }
        }
    }

    /// The ids held, oldest first.
    pub fn ids(&self) -> Vec<u64> {
        let inner = self.inner.lock().expect("not poisoned");
        inner.captures.iter().map(|(id, _)| *id).collect()
    }

    /// Empties the buffer.
    pub fn clear(&self) {
        self.inner.lock().expect("not poisoned").captures.clear();
    }
}

#[cfg(test)]
mod tests {
    use image::RgbImage;

    use super::*;
    use crate::hardware::CaptureMetadata;

    fn capture(width: u32) -> Capture {
        Capture {
            image: RgbImage::new(width, 1),
            metadata: CaptureMetadata::default(),
        }
    }

    #[test]
    fn holds_at_most_buffer_max_captures_and_finds_them_by_id() {
        let buffer = CaptureBuffer::default();
        assert_eq!(buffer.add(capture(1), 1), 1);
        assert_eq!(buffer.add(capture(2), 3), 2);
        assert_eq!(buffer.add(capture(3), 3), 3);
        assert_eq!(buffer.ids(), [1, 2, 3]);
        // Room for two: the oldest goes.
        assert_eq!(buffer.add(capture(4), 3), 4);
        assert_eq!(buffer.ids(), [2, 3, 4]);
        assert_eq!(buffer.add(capture(5), 2), 5);
        assert_eq!(buffer.ids(), [4, 5]);

        assert_eq!(buffer.take(Some(4)).expect("held").image.width(), 4);
        assert!(buffer.take(Some(4)).is_none(), "taken out");
        assert!(buffer.take(Some(1)).is_none(), "dropped");
        assert_eq!(buffer.ids(), [5]);
        // 0 is taken as 1.
        assert_eq!(buffer.add(capture(6), 0), 6);
        assert_eq!(buffer.ids(), [6]);
    }

    #[test]
    fn taking_the_latest_empties_the_buffer() {
        let buffer = CaptureBuffer::default();
        assert!(buffer.take(None).is_none());
        for width in 1..=3 {
            buffer.add(capture(width), 5);
        }
        assert_eq!(buffer.take(None).expect("held").image.width(), 3);
        assert!(buffer.ids().is_empty());
        buffer.add(capture(4), 5);
        buffer.clear();
        assert!(buffer.take(None).is_none());
    }
}
