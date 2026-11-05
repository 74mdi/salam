//! Design tokens — the single source of truth for the glass look.
//!
//! These values mirror the web client's CSS custom properties and the
//! screenshots in `docs/screenshots/`: change them here, change them there.

/// Monochrome primary action — a quiet ivory instead of a saturated brand hue.
pub const ACCENT: iced::Color = iced::Color::from_rgb(0.91, 0.91, 0.89);

/// Secondary emphasis. Kept separate so controls can stay legible without color.
pub const ACCENT_2: iced::Color = iced::Color::from_rgb(0.70, 0.70, 0.68);

/// Deep charcoal backdrop. The system window blur can sit behind this layer.
pub const SURFACE_DARK: iced::Color = iced::Color::from_rgb(0.035, 0.035, 0.038);

/// Structural surface. Native iced cannot sample the desktop backdrop, so this
/// is intentionally neutral and opaque enough to remain readable everywhere.
pub const GLASS_PANEL: iced::Color = iced::Color::from_rgba(0.075, 0.075, 0.08, 0.96);

/// Glass border (specular edge).
pub const GLASS_BORDER: iced::Color = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.12);

/// Primary text.
pub const TEXT_PRIMARY: iced::Color = iced::Color::from_rgb(0.91, 0.91, 0.90);

/// Secondary text.
pub const TEXT_SECONDARY: iced::Color = iced::Color::from_rgb(0.62, 0.62, 0.62);

/// Muted text.
pub const TEXT_MUTED: iced::Color = iced::Color::from_rgb(0.40, 0.40, 0.40);

/// Neutral connection/status indicator.
pub const STATUS_GREEN: iced::Color = iced::Color::from_rgb(0.74, 0.74, 0.72);

/// Text used on the ivory primary action.
pub const TEXT_INVERSE: iced::Color = iced::Color::from_rgb(0.06, 0.06, 0.06);

/// Panel corner radius (px).
pub const RADIUS_PANEL: f32 = 20.0;

/// Pill / bubble corner radius (px).
pub const RADIUS_PILL: f32 = 14.0;

/// Base spacing unit (px).
pub const SPACE: f32 = 8.0;
