//! Duplo clique: o crossterm só entrega cliques soltos.

use std::time::{Duration, Instant};

use super::geometry::Cell;

pub struct ClickTracker {
    threshold: Duration,
    last: Option<((usize, Cell), Instant)>,
}

impl ClickTracker {
    pub fn new(threshold: Duration) -> Self {
        Self {
            threshold,
            last: None,
        }
    }

    /// `true` quando é o segundo clique no mesmo alvo dentro do limiar.
    pub fn register(&mut self, target: (usize, Cell), at: Instant) -> bool {
        if let Some((last, when)) = self.last
            && last == target
            && at.duration_since(when) <= self.threshold
        {
            self.last = None;
            return true;
        }
        self.last = Some((target, at));
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: (usize, Cell) = (0, Cell::Name);

    fn tracker() -> ClickTracker {
        ClickTracker::new(Duration::from_millis(400))
    }

    #[test]
    fn second_click_in_time_is_double() {
        let start = Instant::now();
        let mut c = tracker();
        assert!(!c.register(T, start));
        assert!(c.register(T, start + Duration::from_millis(100)));
    }

    #[test]
    fn slow_clicks_are_single() {
        let start = Instant::now();
        let mut c = tracker();
        assert!(!c.register(T, start));
        assert!(!c.register(T, start + Duration::from_millis(500)));
    }

    #[test]
    fn different_targets_are_single() {
        let start = Instant::now();
        let mut c = tracker();
        assert!(!c.register(T, start));
        assert!(!c.register((0, Cell::Value), start + Duration::from_millis(100)));
    }

    #[test]
    fn third_quick_click_starts_over() {
        let start = Instant::now();
        let mut c = tracker();
        assert!(!c.register(T, start));
        assert!(c.register(T, start + Duration::from_millis(100)));
        assert!(!c.register(T, start + Duration::from_millis(200)));
    }
}
