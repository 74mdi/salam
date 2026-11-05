//! Small embedded subset of the official Iconoir regular SVG set.
//!
//! The source library is MIT licensed and uses a 24×24 grid. Keeping the
//! controls embedded makes the native client deterministic and offline-safe.

use iced::widget::svg::{self, Handle, Svg};
use iced::{Color, Length, Theme};

use crate::tokens;

/// Iconoir symbols used by the Koko shell.
#[derive(Debug, Clone, Copy)]
pub enum Icon {
    Home,
    Settings,
    More,
    Plus,
    Send,
    Image,
    Download,
    Reply,
    Forward,
    Brain,
    Code,
    Magic,
    Lock,
    Close,
}

/// Render one embedded Iconoir symbol with the current text color.
pub fn icon(symbol: Icon, size: u16) -> Svg<'static> {
    icon_with_color(symbol, size, tokens::TEXT_PRIMARY)
}

/// Render one symbol with an explicit tint for contrast-sensitive controls.
pub fn icon_with_color(symbol: Icon, size: u16, color: Color) -> Svg<'static> {
    iced::widget::svg(Handle::from_memory(source(symbol)))
        .width(Length::Fixed(f32::from(size)))
        .height(Length::Fixed(f32::from(size)))
        .style(move |_theme: &Theme, _status| svg::Style { color: Some(color) })
}

fn source(symbol: Icon) -> &'static [u8] {
    match symbol {
        Icon::Home => HOME,
        Icon::Settings => SETTINGS,
        Icon::More => MORE,
        Icon::Plus => PLUS,
        Icon::Send => SEND,
        Icon::Image => IMAGE,
        Icon::Download => DOWNLOAD,
        Icon::Reply => REPLY,
        Icon::Forward => FORWARD,
        Icon::Brain => BRAIN,
        Icon::Code => CODE,
        Icon::Magic => MAGIC,
        Icon::Lock => LOCK,
        Icon::Close => CLOSE,
    }
}

const HOME: &[u8] = br#"<svg width="24" height="24" viewBox="0 0 24 24" stroke-width="1.5" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M2 8L11.7317 3.13416C11.9006 3.04971 12.0994 3.0497 12.2683 3.13416L22 8" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/><path d="M20 11V19C20 20.1046 19.1046 21 18 21H6C4.89543 21 4 20.1046 4 19V11" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/></svg>"#;
const SETTINGS: &[u8] = br#"<svg width="24" height="24" viewBox="0 0 24 24" stroke-width="1.5" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M12 18C15.3137 18 18 15.3137 18 12C18 8.68629 15.3137 6 12 6C8.68629 6 6 8.68629 6 12C6 15.3137 8.68629 18 12 18Z" stroke="currentColor"/><path d="M2 12H6M18 12H22M12 2V6M12 18V22" stroke="currentColor" stroke-linecap="round"/></svg>"#;
const MORE: &[u8] = br#"<svg width="24" height="24" viewBox="0 0 24 24" stroke-width="1.5" fill="none" xmlns="http://www.w3.org/2000/svg"><circle cx="5" cy="12" r="1" fill="currentColor"/><circle cx="12" cy="12" r="1" fill="currentColor"/><circle cx="19" cy="12" r="1" fill="currentColor"/></svg>"#;
const PLUS: &[u8] = br#"<svg width="24" height="24" viewBox="0 0 24 24" stroke-width="1.5" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M6 12H18M12 6V18" stroke="currentColor" stroke-linecap="round"/></svg>"#;
const SEND: &[u8] = br#"<svg width="24" height="24" viewBox="0 0 24 24" stroke-width="1.5" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M3 11L21 3L13 21L10 13L3 11Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/><path d="M10 13L21 3" stroke="currentColor" stroke-linecap="round"/></svg>"#;
const IMAGE: &[u8] = br#"<svg width="24" height="24" viewBox="0 0 24 24" stroke-width="1.5" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M21 3.6V20.4C21 20.7314 20.7314 21 20.4 21H3.6C3.26863 21 3 20.7314 3 20.4V3.6C3 3.26863 3.26863 3 3.6 3H20.4C20.7314 3 21 3.26863 21 3.6Z" stroke="currentColor"/><path d="M3 16L10 13L21 18" stroke="currentColor"/><circle cx="16" cy="8" r="2" stroke="currentColor"/></svg>"#;
const DOWNLOAD: &[u8] = br#"<svg width="24" height="24" viewBox="0 0 24 24" stroke-width="1.5" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M12 3V17M12 17L17 12M12 17L7 12" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/><path d="M4 21H20" stroke="currentColor" stroke-linecap="round"/></svg>"#;
const REPLY: &[u8] = br#"<svg width="24" height="24" viewBox="0 0 24 24" stroke-width="1.5" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M9 8L4 12L9 16" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/><path d="M4 12H15C18.3137 12 21 14.6863 21 18V20" stroke="currentColor" stroke-linecap="round"/></svg>"#;
const FORWARD: &[u8] = br#"<svg width="24" height="24" viewBox="0 0 24 24" stroke-width="1.5" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M15 8L20 12L15 16" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/><path d="M20 12H9C5.68629 12 3 14.6863 3 18V20" stroke="currentColor" stroke-linecap="round"/></svg>"#;
const BRAIN: &[u8] = br#"<svg width="24" height="24" viewBox="0 0 24 24" stroke-width="1.5" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M12 4.5C12 3.11929 10.8807 2 9.5 2C8.11929 2 7 3.11929 7 4.5C7 4.88103 7.08524 5.24215 7.23769 5.56533C5.1868 4.78564 3 6.30649 3 8.5C3 8.66889 3.01213 8.83495 3.03555 8.99736C2.40343 9.73075 2 10.6844 2 11.7321C2 13.5262 3.17471 15.0459 4.79084 15.5538C4.37838 16.1069 4.14815 16.7936 4.14815 17.534C4.14815 19.3981 5.65999 20.91 7.5241 20.91C7.88032 20.91 8.22411 20.8548 8.54588 20.7528C8.97754 21.4937 9.7793 22 10.6993 22C11.4162 22 12.0627 21.6917 12.5 21.195" stroke="currentColor" stroke-linecap="round"/><path d="M12 4.5C12 3.11929 13.1193 2 14.5 2C15.8807 2 17 3.11929 17 4.5C17 4.88103 16.9148 5.24215 16.7623 5.56533C18.8132 4.78564 21 6.30649 21 8.5C21 8.66889 20.9879 8.83495 20.9644 8.99736C21.5966 9.73075 22 10.6844 22 11.7321C22 13.5262 20.8253 15.0459 19.2092 15.5538C19.6216 16.1069 19.8519 16.7936 19.8519 17.534C19.8519 19.3981 18.34 20.91 16.4759 20.91C16.1197 20.91 15.7759 20.8548 15.4541 20.7528C15.0225 21.4937 14.2207 22 13.3007 22C12.5838 22 11.9373 21.6917 11.5 21.195" stroke="currentColor" stroke-linecap="round"/><path d="M12 4.5V21" stroke="currentColor" stroke-linecap="round"/></svg>"#;
const CODE: &[u8] = br#"<svg width="24" height="24" viewBox="0 0 24 24" stroke-width="1.5" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M9 21H8C6.89543 21 6 20.1046 6 19V14C6 13 4.5 12 4.5 12C4.5 12 6 11 6 10V5C6 3.89543 6.89543 3 8 3H9M15 21H16C17.1046 21 18 20.1046 18 19V14C18 13 19.5 12 19.5 12C19.5 12 18 11 18 10V5C18 3.89543 17.1046 3 16 3H15" stroke="currentColor" stroke-linecap="round"/></svg>"#;
const MAGIC: &[u8] = br#"<svg width="24" height="24" viewBox="0 0 24 24" stroke-width="1.5" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M3 21L13 11M18 6L15.5 8.5M9.5 2L10.4453 4.55468L13 5.5L10.4453 6.44532L9.5 9L8.55468 6.44532L6 5.5L8.55468 4.55468L9.5 2Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/></svg>"#;
const LOCK: &[u8] = br#"<svg width="24" height="24" viewBox="0 0 24 24" stroke-width="1.5" fill="none" xmlns="http://www.w3.org/2000/svg"><rect x="4" y="10" width="16" height="11" rx="2" stroke="currentColor"/><path d="M8 10V7C8 4.79086 9.79086 3 12 3C14.2091 3 16 4.79086 16 7V10" stroke="currentColor" stroke-linecap="round"/><path d="M12 14V17" stroke="currentColor" stroke-linecap="round"/></svg>"#;
const CLOSE: &[u8] = br#"<svg width="24" height="24" viewBox="0 0 24 24" stroke-width="1.5" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M6 6L18 18M18 6L6 18" stroke="currentColor" stroke-linecap="round"/></svg>"#;
