/// Grid dimensions and tile size.
pub const TILE_SIZE: u16 = 8;
pub const DEFAULT_COLS: u16 = 20;
pub const DEFAULT_ROWS: u16 = 11;
pub const MAX_COLS: u16 = 64;
pub const MAX_ROWS: u16 = 64;

/// Animation frame timing (milliseconds converted to seconds for f32 dt).
pub const WALK_FRAME_MS: f32 = 150.0;
pub const TYPE_FRAME_MS: f32 = 300.0;
pub const WALK_SPEED: f32 = 3.0;
pub const SITTING_OFFSET_PX: f32 = 3.0;

/// Wander behavior ranges.
pub const WANDER_PAUSE_MIN: f32 = 2.0;
pub const WANDER_PAUSE_MAX: f32 = 20.0;
pub const WANDER_MOVES_MIN: u8 = 3;
pub const WANDER_MOVES_MAX: u8 = 6;
pub const REST_DURATION_MIN: f32 = 120.0;
pub const REST_DURATION_MAX: f32 = 240.0;

/// Timer durations.
pub const JSONL_POLL_MS: u64 = 500;
pub const SCAN_INTERVAL_MS: u64 = 3_000;
pub const STALE_THRESHOLD_SECS: u64 = 600;
pub const PERMISSION_TIMER_MS: u64 = 7_000;
pub const TEXT_IDLE_TIMER_MS: u64 = 5_000;
pub const TOOL_DONE_DELAY_MS: u64 = 300;
pub const JSONL_READ_CAP: u64 = 65_536;

/// Scanner thresholds.
pub const EXTERNAL_THRESHOLD_SECS: u64 = 120;
pub const DISMISSED_COOLDOWN_SECS: u64 = 180;
pub const CLEAR_IDLE_THRESHOLD_MS: u64 = 2_000;
pub const GLOBAL_MIN_FILE_SIZE: u64 = 3_072;

/// Visual constants.
pub const PALETTE_COUNT: u8 = 6;
pub const HUE_SHIFT_MIN_DEG: i16 = 45;
pub const HUE_SHIFT_RANGE_DEG: i16 = 270;
pub const MATRIX_DURATION_SECS: f32 = 0.3;
pub const WAITING_BUBBLE_SECS: f32 = 2.0;
pub const BUBBLE_FADE_SECS: f32 = 0.5;
pub const FURNITURE_ANIM_INTERVAL_SECS: f32 = 0.2;
pub const AUTO_ON_FACING_DEPTH: u16 = 3;

/// Frame timing.
pub const MAX_DELTA_TIME: f64 = 0.1;
pub const TICK_RATE_MS: u64 = 16;
