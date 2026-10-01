/// Grid dimensions and tile size.
pub const TILE_SIZE: u16 = 8;
pub const DEFAULT_COLS: u16 = 28;
pub const DEFAULT_ROWS: u16 = 16;
pub const MAX_COLS: u16 = 64;
pub const MAX_ROWS: u16 = 64;

/// Animation frame timing (milliseconds converted to seconds for f32 dt).
pub const WALK_FRAME_MS: f32 = 150.0;
pub const TYPE_FRAME_MS: f32 = 300.0;
pub const WALK_SPEED: f32 = 3.0;
pub const SITTING_OFFSET_PX: f32 = 1.0;

/// Wander behavior ranges.
pub const WANDER_PAUSE_MIN: f32 = 2.0;
pub const WANDER_PAUSE_MAX: f32 = 20.0;
pub const WANDER_MOVES_MIN: u8 = 3;
pub const WANDER_MOVES_MAX: u8 = 6;
pub const REST_DURATION_MIN: f32 = 120.0;
pub const REST_DURATION_MAX: f32 = 240.0;

/// Timer durations.
pub const JSONL_POLL_MS: u64 = 500;
pub const PERMISSION_TIMER_MS: u64 = 7_000;
pub const TEXT_IDLE_TIMER_MS: u64 = 5_000;
pub const JSONL_READ_CAP: u64 = 65_536;

/// Longest transcript line kept in memory; longer lines are skipped.
pub const JSONL_MAX_LINE_BYTES: usize = 8 * 1024 * 1024;

/// Bytes read from the end of a transcript to rebuild an adopted agent's state.
pub const REPLAY_TAIL_BYTES: u64 = 65_536;

/// Smallest terminal width the farm view is drawn in, in columns.
pub const MIN_TERMINAL_COLS: u16 = 40;

/// Smallest scene height the farm view is drawn in, in rows, excluding the panel.
pub const MIN_TERMINAL_ROWS: u16 = 8;

/// Widest agent identity in the agent panel, in terminal columns.
pub const NAME_MAX_WIDTH: usize = 12;

/// Widest floating name label above a character, in terminal columns.
pub const LABEL_MAX_WIDTH: usize = 8;

/// Longest text kept for one summary source (tool detail, assistant line, title, prompt).
pub const SUMMARY_SOURCE_MAX_CHARS: usize = 200;

/// Scanner thresholds.
pub const EXTERNAL_THRESHOLD_SECS: u64 = 120;
pub const DISMISSED_COOLDOWN_SECS: u64 = 180;
pub const CLEAR_IDLE_THRESHOLD_MS: u64 = 2_000;
pub const GLOBAL_MIN_FILE_SIZE: u64 = 3_072;

/// Dormancy: stale threshold triggers dormant state (agent enters building).
/// Removed threshold triggers actual removal (agent truly gone).
pub const DORMANT_THRESHOLD_SECS: u64 = 600; // 10 minutes → dormant
pub const REMOVED_THRESHOLD_SECS: u64 = 7_200; // 2 hours → removed

/// Agent limits.
pub const MAX_AGENTS: usize = 8;

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

/// Subdirectory of the config directory that holds log files.
pub const LOG_DIR_NAME: &str = "logs";

/// Log file name prefix; files are `harvest-claude.<date>.log`.
pub const LOG_FILE_PREFIX: &str = "harvest-claude";

/// Log file name suffix.
pub const LOG_FILE_SUFFIX: &str = "log";

/// Daily log files kept before the oldest is deleted.
pub const LOG_MAX_FILES: usize = 3;
