// ─── Color palette ───────────────────────────────────────────────────────────
pub const BG_APP: u32 = 0x0f1117;
pub const BG_SIDEBAR: u32 = 0x161b22;
pub const BG_PANEL: u32 = 0x0d1117;
pub const BG_SIDEBAR_ACTIVE: u32 = 0x1c2333;
pub const BG_HEADER: u32 = 0x161b22;
pub const BG_CARD: u32 = 0x161b22;
pub const BORDER: u32 = 0x21262d;
pub const TEXT_MUTED: u32 = 0x484f58;
pub const TEXT_SECONDARY: u32 = 0x7d8590;
pub const TEXT_PRIMARY: u32 = 0xcdd9e5;
pub const ACCENT: u32 = 0x388bfd;
pub const ACCENT_BG: u32 = 0x1f3458;
pub const BG_ERROR: u32 = 0x2d1515;
pub const BORDER_ERROR: u32 = 0x6e1b1b;
pub const TEXT_ERROR: u32 = 0xff6b6b;

// ─── Layout constants ─────────────────────────────────────────────────────────
pub const CELL_W: f32 = 160.0;
pub const ROW_H: f32 = 36.0;
/// Rows fetched per lazy-load request.
pub const CHUNK_SIZE: usize = 1_000;
