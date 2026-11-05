//! iced theme wiring: map design tokens onto iced's `Theme`.

use iced::theme::Palette;
use iced::Theme;

use crate::tokens;

/// Build the Koko dark theme.
pub fn koko_dark() -> Theme {
    Theme::custom(
        "Koko Dark".into(),
        Palette {
            background: tokens::SURFACE_DARK,
            text: tokens::TEXT_PRIMARY,
            primary: tokens::ACCENT,
            success: tokens::STATUS_GREEN,
            danger: iced::Color::from_rgb(0.78, 0.68, 0.68),
        },
    )
}

/// Alias kept for API stability.
pub type KokoTheme = Theme;
