use crate::constants::{BUBBLE_FADE_SECS, WAITING_BUBBLE_SECS};
use crate::types::{BubbleKind, Pixel, SpriteData};

/// Bubble timer and display state.
#[derive(Debug, Clone)]
pub struct BubbleState {
    /// Bubble variant.
    pub kind: BubbleKind,
    /// Elapsed time since bubble appeared (seconds).
    pub elapsed: f32,
    /// Current alpha (0-255), reduced during fade-out.
    pub alpha: u8,
}

impl BubbleState {
    /// Create a new bubble state.
    pub fn new(kind: BubbleKind) -> Self {
        Self {
            kind,
            elapsed: 0.0,
            alpha: 255,
        }
    }
}

/// Generate an amber "..." permission bubble sprite (16x6).
pub fn permission_bubble() -> SpriteData {
    let bg: Pixel = (255, 255, 255, 230);
    let border: Pixel = (180, 180, 180, 230);
    let dot: Pixel = (200, 150, 50, 255);
    let tail: Pixel = (255, 255, 255, 230);
    let t: Pixel = (0, 0, 0, 0);

    vec![
        vec![
            t, border, border, border, border, border, border, border, border, border, border,
            border, border, border, border, t,
        ],
        vec![
            border, bg, bg, bg, bg, bg, bg, bg, bg, bg, bg, bg, bg, bg, bg, border,
        ],
        vec![
            border, bg, bg, bg, dot, bg, bg, dot, bg, bg, dot, bg, bg, bg, bg, border,
        ],
        vec![
            border, bg, bg, bg, bg, bg, bg, bg, bg, bg, bg, bg, bg, bg, bg, border,
        ],
        vec![
            t, border, border, border, border, border, border, border, border, border, border,
            border, border, border, border, t,
        ],
        vec![t, t, t, t, tail, tail, t, t, t, t, t, t, t, t, t, t],
    ]
}

/// Generate a green checkmark waiting bubble sprite (16x6).
pub fn waiting_bubble() -> SpriteData {
    let bg: Pixel = (255, 255, 255, 230);
    let border: Pixel = (180, 180, 180, 230);
    let check: Pixel = (50, 180, 80, 255);
    let tail: Pixel = (255, 255, 255, 230);
    let t: Pixel = (0, 0, 0, 0);

    vec![
        vec![
            t, border, border, border, border, border, border, border, border, border, border,
            border, border, border, border, t,
        ],
        vec![
            border, bg, bg, bg, bg, bg, bg, bg, bg, bg, bg, check, bg, bg, bg, border,
        ],
        vec![
            border, bg, bg, bg, bg, bg, bg, bg, bg, check, bg, bg, bg, bg, bg, border,
        ],
        vec![
            border, bg, bg, bg, bg, check, bg, check, bg, bg, bg, bg, bg, bg, bg, border,
        ],
        vec![
            t, border, border, border, border, border, check, border, border, border, border,
            border, border, border, border, t,
        ],
        vec![t, t, t, t, tail, tail, t, t, t, t, t, t, t, t, t, t],
    ]
}

/// Tick the bubble timer. Returns `true` when the bubble has expired and should be removed.
///
/// - Waiting bubble: displays for `WAITING_BUBBLE_SECS` then fades over `BUBBLE_FADE_SECS`.
/// - Permission bubble: persistent (never expires from timer alone).
pub fn update_bubble(bubble: &mut BubbleState, dt: f32) -> bool {
    bubble.elapsed += dt;

    match bubble.kind {
        BubbleKind::Waiting => {
            let total_duration = WAITING_BUBBLE_SECS + BUBBLE_FADE_SECS;
            if bubble.elapsed >= total_duration {
                bubble.alpha = 0;
                return true;
            }
            if bubble.elapsed > WAITING_BUBBLE_SECS {
                // Fade-out phase
                let fade_progress = (bubble.elapsed - WAITING_BUBBLE_SECS) / BUBBLE_FADE_SECS;
                bubble.alpha = ((1.0 - fade_progress) * 255.0).round() as u8;
            }
            false
        }
        BubbleKind::Permission => {
            // Persistent, never expires via timer
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_bubble_dimensions() {
        let sprite = permission_bubble();
        assert_eq!(sprite.len(), 6);
        assert!(sprite.iter().all(|row| row.len() == 16));
    }

    #[test]
    fn waiting_bubble_dimensions() {
        let sprite = waiting_bubble();
        assert_eq!(sprite.len(), 6);
        assert!(sprite.iter().all(|row| row.len() == 16));
    }

    #[test]
    fn waiting_bubble_expires() {
        let mut state = BubbleState::new(BubbleKind::Waiting);
        // Not expired yet at 1s
        assert!(!update_bubble(&mut state, 1.0));
        assert_eq!(state.alpha, 255);

        // Into fade territory (elapsed = 2.2, past WAITING_BUBBLE_SECS=2.0)
        assert!(!update_bubble(&mut state, 1.2));
        assert!(state.alpha < 255);

        // Fully expired (elapsed = 3.2, past total 2.5)
        assert!(update_bubble(&mut state, 1.0));
        assert_eq!(state.alpha, 0);
    }

    #[test]
    fn permission_bubble_never_expires() {
        let mut state = BubbleState::new(BubbleKind::Permission);
        assert!(!update_bubble(&mut state, 100.0));
        assert!(!update_bubble(&mut state, 1000.0));
        assert_eq!(state.alpha, 255);
    }

    #[test]
    fn bubble_state_new() {
        let state = BubbleState::new(BubbleKind::Waiting);
        assert_eq!(state.kind, BubbleKind::Waiting);
        assert_eq!(state.elapsed, 0.0);
        assert_eq!(state.alpha, 255);
    }

    #[test]
    fn waiting_bubble_expires_after_total_duration() {
        let mut state = BubbleState::new(BubbleKind::Waiting);
        let total = WAITING_BUBBLE_SECS + BUBBLE_FADE_SECS;
        // Advance past total duration in one step
        let expired = update_bubble(&mut state, total + 0.1);
        assert!(expired, "bubble should be expired past total duration");
        assert_eq!(state.alpha, 0);
    }

    #[test]
    fn permission_bubble_persistent_across_large_dt() {
        let mut state = BubbleState::new(BubbleKind::Permission);
        // Even after an extremely long time, permission bubble persists
        assert!(!update_bubble(&mut state, 10_000.0));
        assert!(!update_bubble(&mut state, 100_000.0));
        assert_eq!(state.alpha, 255);
    }
}
