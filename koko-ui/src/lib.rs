//! # koko-ui
//!
//! The liquid-glass design system for native Koko clients, built on iced.
//!
//! * [`tokens`] — colors, radii, spacing (mirrors the web client's CSS
//!   custom properties; change them once, everywhere).
//! * [`theme`] — maps tokens onto iced's `Theme`.
//! * [`glass`] — frosted panels, cards, buttons, bubbles (style functions).
//!
//! Renderer policy: everything renders on `tiny-skia` (no GPU needed — the
//! Linux fallback everywhere), and the optional `wgpu` feature upgrades to
//! the GPU renderer where available. Real backdrop blur (iced_glass-style
//! shader pass) plugs in behind [`glass`] without touching call sites.

pub mod glass;
pub mod iconoir;
pub mod theme;
pub mod tokens;

pub use glass::{accent, bubble, card, ghost, panel, wallpaper};
pub use theme::koko_dark;
pub use tokens::{
    ACCENT, ACCENT_2, GLASS_BORDER, GLASS_PANEL, RADIUS_PANEL, RADIUS_PILL, SPACE, STATUS_GREEN,
    SURFACE_DARK, TEXT_INVERSE, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
