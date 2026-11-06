//! Glass surfaces for iced: frosted panels, cards, buttons, bubbles.
//!
//! **Honest rendering note.** The native client leaves desktop/window blur to
//! the host compositor. These surfaces provide the tint, rim and hierarchy on
//! every iced backend, including `tiny-skia` (no GPU required).

use iced::widget::button;
use iced::widget::container;
use iced::widget::text_input;
use iced::{Background, Border, Color, Shadow, Theme, Vector};

use crate::tokens;

fn surface(fill: Color, radius: f32, border: Color, shadow: Shadow) -> container::Style {
    container::Style {
        background: Some(Background::Color(fill)),
        border: Border {
            radius: radius.into(),
            width: 1.0,
            color: border,
        },
        shadow,
        text_color: Some(tokens::TEXT_PRIMARY),
    }
}

/// Large structural panel (sidebar, conversation shell, login card).
pub fn panel(_theme: &Theme) -> container::Style {
    surface(
        tokens::GLASS_PANEL,
        tokens::RADIUS_PANEL,
        tokens::GLASS_BORDER,
        Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
            offset: Vector::new(0.0, 12.0),
            blur_radius: 30.0,
        },
    )
}

/// Structural panel variant used by the accessibility controls.
pub fn panel_mode(
    solid: bool,
    high_contrast: bool,
    tone: &str,
    _theme: &Theme,
) -> container::Style {
    let fill = if solid {
        Color::from_rgb(0.09, 0.09, 0.10)
    } else {
        match tone {
            "Subtle" => Color::from_rgba(0.075, 0.075, 0.08, 0.84),
            "Strong" => Color::from_rgba(0.11, 0.11, 0.12, 0.98),
            _ => tokens::GLASS_PANEL,
        }
    };
    let border = if high_contrast {
        Color::from_rgba(1.0, 1.0, 1.0, 0.24)
    } else {
        tokens::GLASS_BORDER
    };
    surface(
        fill,
        tokens::RADIUS_PANEL,
        border,
        Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
            offset: Vector::new(0.0, 12.0),
            blur_radius: 30.0,
        },
    )
}

/// Small card (feature tiles, empty states, chips, avatars).
pub fn card(_theme: &Theme) -> container::Style {
    surface(
        Color::from_rgba(1.0, 1.0, 1.0, 0.065),
        tokens::RADIUS_PILL,
        tokens::GLASS_BORDER,
        Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.25),
            offset: Vector::new(0.0, 6.0),
            blur_radius: 18.0,
        },
    )
}

/// A quiet content section. Unlike `card`, it does not pretend to be glass.
pub fn content_card(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.035))),
        border: Border {
            radius: 16.0.into(),
            width: 1.0,
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.07),
        },
        shadow: Shadow::default(),
        text_color: Some(tokens::TEXT_PRIMARY),
    }
}

/// Section surface variant for tone, high-contrast and reduced-transparency modes.
pub fn content_card_mode(
    solid: bool,
    high_contrast: bool,
    tone: &str,
    _theme: &Theme,
) -> container::Style {
    let fill = if solid {
        Color::from_rgb(0.105, 0.105, 0.11)
    } else {
        match tone {
            "Subtle" => Color::from_rgba(1.0, 1.0, 1.0, 0.02),
            "Strong" => Color::from_rgba(1.0, 1.0, 1.0, 0.07),
            _ => Color::from_rgba(1.0, 1.0, 1.0, 0.035),
        }
    };
    let border = if high_contrast {
        Color::from_rgba(1.0, 1.0, 1.0, 0.24)
    } else {
        Color::from_rgba(1.0, 1.0, 1.0, 0.09)
    };
    surface(fill, 12.0, border, Shadow::default())
}

/// The persistent navigation column: structured, but not a floating card.
pub fn sidebar(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.03, 0.03, 0.032, 0.90))),
        border: Border {
            radius: 0.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        text_color: Some(tokens::TEXT_PRIMARY),
    }
}

/// Chat message bubble. Message content stays flat; glass is reserved for controls.
pub fn bubble(mine: bool, _theme: &Theme) -> container::Style {
    let fill = if mine {
        Color::from_rgb(0.22, 0.22, 0.23)
    } else {
        Color::from_rgb(0.12, 0.12, 0.13)
    };
    container::Style {
        background: Some(Background::Color(fill)),
        border: Border {
            radius: 18.0.into(),
            width: 1.0,
            color: fill,
        },
        shadow: Shadow::default(),
        text_color: Some(tokens::TEXT_PRIMARY),
    }
}

/// Primary action button (ivory fill, specular edge).
pub fn accent(_theme: &Theme, status: button::Status) -> button::Style {
    let (bg, border_color) = match status {
        button::Status::Hovered => (
            Background::Color(Color::from_rgb(1.0, 1.0, 0.98)),
            Color::from_rgba(1.0, 1.0, 1.0, 0.30),
        ),
        button::Status::Pressed => (
            Background::Color(Color::from_rgb(0.72, 0.72, 0.70)),
            Color::from_rgba(1.0, 1.0, 1.0, 0.20),
        ),
        _ => (
            Background::Color(tokens::ACCENT),
            Color::from_rgba(1.0, 1.0, 1.0, 0.25),
        ),
    };
    button::Style {
        background: Some(bg),
        text_color: tokens::TEXT_INVERSE,
        border: Border {
            radius: tokens::RADIUS_PILL.into(),
            width: 1.0,
            color: border_color,
        },
        shadow: Shadow {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 14.0,
        },
    }
}

/// Dark send control so the light Iconoir glyph remains visible in the
/// monochrome palette.
pub fn send_button(_theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered => Color::from_rgb(0.28, 0.28, 0.29),
        button::Status::Pressed => Color::from_rgb(0.18, 0.18, 0.19),
        _ => Color::from_rgb(0.22, 0.22, 0.23),
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: tokens::TEXT_PRIMARY,
        border: Border {
            radius: tokens::RADIUS_PILL.into(),
            width: 1.0,
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.24),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.35),
            offset: Vector::new(0.0, 3.0),
            blur_radius: 10.0,
        },
    }
}

/// Subtle glass button (secondary actions, icon buttons, chat rows).
pub fn ghost(_theme: &Theme, status: button::Status) -> button::Style {
    let alpha = match status {
        button::Status::Hovered => 0.16,
        button::Status::Pressed => 0.10,
        _ => 0.07,
    };
    button::Style {
        background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, alpha))),
        text_color: tokens::TEXT_PRIMARY,
        border: Border {
            radius: tokens::RADIUS_PILL.into(),
            width: 1.0,
            color: tokens::GLASS_BORDER,
        },
        shadow: Shadow::default(),
    }
}

/// Icon-sized control used in the rail and avatar menu.
pub fn icon_button(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = ghost(theme, status);
    style.border.radius = 12.0.into();
    style
}

/// A quiet icon action that only gains a surface on hover or press.
pub fn quiet_icon_button(_theme: &Theme, status: button::Status) -> button::Style {
    let (alpha, border) = match status {
        button::Status::Hovered => (0.09, tokens::GLASS_BORDER),
        button::Status::Pressed => (0.14, tokens::GLASS_BORDER),
        _ => (0.0, Color::TRANSPARENT),
    };
    button::Style {
        background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, alpha))),
        text_color: tokens::TEXT_PRIMARY,
        border: Border {
            radius: 10.0.into(),
            width: 1.0,
            color: border,
        },
        shadow: Shadow::default(),
    }
}

/// Low-noise row button used for settings switches and menus.
pub fn row_button(_theme: &Theme, status: button::Status) -> button::Style {
    let alpha = match status {
        button::Status::Hovered => 0.09,
        button::Status::Pressed => 0.13,
        _ => 0.0,
    };
    button::Style {
        background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, alpha))),
        text_color: tokens::TEXT_PRIMARY,
        border: Border {
            radius: 12.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
    }
}

/// Small selected/unselected navigation item.
pub fn rail_item(selected: bool, theme: &Theme, status: button::Status) -> button::Style {
    let mut style = icon_button(theme, status);
    if selected {
        style.background = Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.15)));
        style.border.color = Color::from_rgba(1.0, 1.0, 1.0, 0.18);
    } else {
        style.background = None;
        style.border.color = Color::TRANSPARENT;
    }
    style
}

/// Selected/unselected settings navigation item.
pub fn nav_item(selected: bool, theme: &Theme, status: button::Status) -> button::Style {
    let mut style = row_button(theme, status);
    if selected {
        style.background = Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.14)));
        style.border.width = 1.0;
        style.border.color = tokens::GLASS_BORDER;
    }
    style
}

/// Compact filter chip.
pub fn chip(selected: bool, theme: &Theme, status: button::Status) -> button::Style {
    let mut style = ghost(theme, status);
    if selected {
        style.background = Some(Background::Color(tokens::ACCENT));
        style.text_color = tokens::TEXT_INVERSE;
    }
    style
}

/// Floating profile/menu surface.
pub fn menu(_theme: &Theme) -> container::Style {
    surface(
        Color::from_rgba(0.09, 0.09, 0.10, 0.98),
        14.0,
        tokens::GLASS_BORDER,
        Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.34),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 18.0,
        },
    )
}

/// Menu rows keep the menu functional without adding a second surface.
pub fn menu_item(theme: &Theme, status: button::Status) -> button::Style {
    row_button(theme, status)
}

/// Image attachment control: quiet at rest, brighter when it can be opened.
pub fn media_button(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = ghost(theme, status);
    style.border.radius = 14.0.into();
    style
}

/// Quoted-message surface used by reply cards in bubbles and the composer.
pub fn reply_quote(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.065))),
        border: Border {
            radius: 10.0.into(),
            width: 1.0,
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.14),
        },
        shadow: Shadow::default(),
        text_color: Some(tokens::TEXT_PRIMARY),
    }
}

/// Full-screen media viewer scrim.
pub fn scrim(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.72))),
        border: Border::default(),
        shadow: Shadow::default(),
        text_color: Some(tokens::TEXT_PRIMARY),
    }
}

/// Centered media viewer panel.
pub fn modal(_theme: &Theme) -> container::Style {
    surface(
        Color::from_rgba(0.08, 0.08, 0.09, 0.98),
        22.0,
        Color::from_rgba(1.0, 1.0, 1.0, 0.18),
        Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.55),
            offset: Vector::new(0.0, 18.0),
            blur_radius: 36.0,
        },
    )
}

/// Neutral avatar tile.
pub fn avatar(_theme: &Theme) -> container::Style {
    surface(
        Color::from_rgba(1.0, 1.0, 1.0, 0.08),
        12.0,
        tokens::GLASS_BORDER,
        Shadow::default(),
    )
}

/// Circular avatar surface used for profile photos and presence identity.
pub fn avatar_circle(_theme: &Theme) -> container::Style {
    surface(
        Color::from_rgba(1.0, 1.0, 1.0, 0.08),
        999.0,
        tokens::GLASS_BORDER,
        Shadow::default(),
    )
}

/// Opaque switch track: the knob is the only movable part.
pub fn switch_track(on: bool, _theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(if on {
            tokens::ACCENT
        } else {
            Color::from_rgb(0.23, 0.23, 0.24)
        })),
        border: Border {
            radius: 13.0.into(),
            width: 1.0,
            color: if on {
                Color::from_rgba(1.0, 1.0, 1.0, 0.36)
            } else {
                tokens::GLASS_BORDER
            },
        },
        shadow: Shadow::default(),
        text_color: Some(tokens::TEXT_PRIMARY),
    }
}

/// Switch knob with a light lens-like treatment while the track stays opaque.
pub fn switch_knob(on: bool, _theme: &Theme) -> container::Style {
    surface(
        if on {
            tokens::TEXT_INVERSE
        } else {
            tokens::TEXT_PRIMARY
        },
        9.0,
        Color::from_rgba(1.0, 1.0, 1.0, 0.28),
        Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.25),
            offset: Vector::new(0.0, 1.0),
            blur_radius: 3.0,
        },
    )
}

/// Conversation rows: flat by default, with a restrained selected state.
pub fn list_row(_theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.075),
        button::Status::Pressed => Color::from_rgba(1.0, 1.0, 1.0, 0.11),
        _ => Color::TRANSPARENT,
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: tokens::TEXT_PRIMARY,
        border: Border {
            radius: 10.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
    }
}

/// Quiet, high-contrast text fields that match the dark shell.
pub fn input(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let focused = matches!(status, text_input::Status::Focused);
    text_input::Style {
        background: Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.22)),
        border: Border {
            radius: 10.0.into(),
            width: if focused { 1.5 } else { 1.0 },
            color: if focused {
                Color::from_rgba(1.0, 1.0, 1.0, 0.72)
            } else {
                Color::from_rgba(1.0, 1.0, 1.0, 0.16)
            },
        },
        icon: tokens::TEXT_SECONDARY,
        placeholder: tokens::TEXT_MUTED,
        value: tokens::TEXT_PRIMARY,
        selection: Color::from_rgba(1.0, 1.0, 1.0, 0.22),
    }
}

/// The wallpaper layer: deep charcoal behind everything translucent.
pub fn wallpaper(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(tokens::SURFACE_DARK)),
        border: Border::default(),
        shadow: Shadow::default(),
        text_color: Some(tokens::TEXT_PRIMARY),
    }
}
