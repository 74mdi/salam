//! Views: login, chat list, conversation. All glass, all koko-ui tokens.

use iced::widget::{
    button, column, container, image, mouse_area, row, scrollable, stack, text, text_input,
};
use iced::{Element, Length, Padding};

use koko_ui::glass;
use koko_ui::iconoir::{icon as iconoir_icon, icon_with_color as iconoir_icon_color, Icon};
use koko_ui::{ACCENT_2, STATUS_GREEN, TEXT_MUTED, TEXT_SECONDARY};

use crate::app::{ActiveChat, ChatFilter, Koko, Message, Screen, SettingsSection};
use crate::store::MirrorMsg;

impl Koko {
    /// The root view (referenced by `main.rs`).
    pub fn view(&self) -> Element<'_, Message> {
        view(self)
    }
}

/// Root view.
fn view(koko: &Koko) -> Element<'_, Message> {
    let body: Element<'_, Message> = match koko.screen {
        crate::app::Screen::Login => login(koko),
        crate::app::Screen::Chats => chats(koko),
        crate::app::Screen::Settings => settings(koko),
    };
    let body = if koko.media_viewer.is_some() {
        stack![body, media_viewer(koko)]
    } else {
        stack![body]
    };
    container(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(glass::wallpaper)
        .into()
}

// ---------------------------------------------------------------- login

#[allow(clippy::too_many_lines)]
fn login(koko: &Koko) -> Element<'_, Message> {
    let sign_in = button(
        container(text("Sign in").size(14))
            .center_x(Length::Fill)
            .padding(if koko.compact_layout { 15 } else { 13 }),
    )
    .on_press(Message::SignInPressed)
    .width(Length::Fill)
    .style(glass::accent);

    let create = button(
        container(text("Create account on your server").size(12))
            .center_x(Length::Fill)
            .padding(if koko.compact_layout { 12 } else { 6 }),
    )
    .on_press(Message::CreateAccountPressed)
    .width(Length::Fill)
    .style(glass::ghost);

    let mut form = column![
        text("Welcome back").size(22),
        text("Connect to your Koko server.")
            .size(13)
            .style(secondary_style),
        iced::widget::Space::with_height(12),
        text("Server address").size(11).style(muted_style),
        text_input(
            if koko.compact_layout {
                "https://your-koko-server"
            } else {
                "http://127.0.0.1:8080"
            },
            &koko.server,
        )
        .on_input(Message::ServerChanged)
        .size(14)
        .padding(if koko.compact_layout { 14 } else { 12 })
        .style(glass::input),
        text("Username").size(11).style(muted_style),
        text_input("your handle", &koko.handle)
            .on_input(Message::HandleChanged)
            .size(14)
            .padding(if koko.compact_layout { 14 } else { 12 })
            .style(glass::input),
        text("Password").size(11).style(muted_style),
        text_input("your password", &koko.password)
            .on_input(Message::PasswordChanged)
            .size(14)
            .padding(if koko.compact_layout { 14 } else { 12 })
            .style(glass::input)
            .secure(true),
        remember_account_row(koko.remember_account),
        iced::widget::Space::with_height(8),
        sign_in,
        create,
    ]
    .spacing(8)
    .width(if koko.compact_layout {
        Length::Fill
    } else {
        Length::Fixed(360.0)
    });

    if let Some(err) = &koko.error {
        form = form.push(
            container(text(err).size(12).style(danger_style))
                .padding(10)
                .width(Length::Fill)
                .style(glass::card),
        );
    }

    let intro = container(
        column![
            logo_row(),
            iced::widget::Space::with_height(32),
            text("Private conversations,\non your server.").size(30),
            text("Koko keeps the interface quiet and the data close to where you run it.")
                .size(13)
                .style(secondary_style),
            iced::widget::Space::with_height(18),
            row![
                login_badge("LOCAL"),
                login_badge("RUST"),
                login_badge("SYNCED"),
            ]
            .spacing(6),
        ]
        .spacing(10),
    )
    .width(330)
    .padding(28);
    let form_panel = container(form)
        .width(if koko.compact_layout {
            Length::Fill
        } else {
            Length::Fixed(420.0)
        })
        .padding(if koko.compact_layout { 18 } else { 28 })
        .style(move |theme| {
            glass::content_card_mode(
                koko.reduce_transparency,
                koko.high_contrast,
                &koko.surface_tone,
                theme,
            )
        });
    if koko.compact_layout {
        return container(scrollable(column![logo_row(), form_panel].spacing(18)))
            .padding(12)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }
    container(row![intro, form_panel].spacing(10))
        .padding(10)
        .width(790)
        .style(move |theme| {
            glass::panel_mode(
                koko.reduce_transparency,
                koko.high_contrast,
                &koko.surface_tone,
                theme,
            )
        })
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

fn login_badge(label: &'static str) -> Element<'static, Message> {
    container(text(label).size(10).style(muted_style))
        .padding(Padding {
            top: 5.0,
            right: 8.0,
            bottom: 5.0,
            left: 8.0,
        })
        .style(glass::card)
        .into()
}

fn logo_row() -> Element<'static, Message> {
    let mark = container(text("K").size(20))
        .width(38)
        .height(38)
        .center_x(38)
        .center_y(38)
        .style(glass::card);
    row![mark, text("koko").size(24)]
        .spacing(10)
        .align_y(iced::Alignment::Center)
        .into()
}

fn avatar_view(koko: &Koko, size: u16) -> Element<'_, Message> {
    if let Some(handle) = &koko.avatar_image {
        container(
            image(handle.clone())
                .width(Length::Fill)
                .height(Length::Fill)
                .content_fit(iced::ContentFit::Cover),
        )
        .width(size)
        .height(size)
        .clip(true)
        .style(glass::avatar_circle)
        .into()
    } else {
        let initial = koko
            .me
            .as_ref()
            .and_then(|me| me.user.display_name.chars().next())
            .or_else(|| {
                koko.me
                    .as_ref()
                    .and_then(|me| me.user.handle.chars().next())
            })
            .unwrap_or('K')
            .to_ascii_uppercase();
        container(text(initial.to_string()).size((size / 3).max(14)))
            .width(size)
            .height(size)
            .center_x(size)
            .center_y(size)
            .style(glass::avatar_circle)
            .into()
    }
}

// ---------------------------------------------------------------- chats

fn chats(koko: &Koko) -> Element<'_, Message> {
    let thread: Element<'_, Message> = match &koko.active {
        Some(active) => conversation(koko, active),
        None => empty_state(),
    };
    if koko.compact_layout {
        let content: Element<'_, Message> = if koko.active.is_some() {
            thread_panel(koko, thread)
        } else {
            column![mobile_header(koko), chat_list(koko)]
                .spacing(8)
                .into()
        };
        return container(content)
            .padding(8)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }
    row![
        rail(koko),
        profile_menu(koko),
        chat_list(koko),
        thread_panel(koko, thread),
    ]
    .spacing(8)
    .padding(8)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn mobile_header(koko: &Koko) -> Element<'_, Message> {
    container(
        row![
            column![
                logo_row(),
                row![
                    status_dot(koko.connected),
                    text(if koko.connected {
                        "Connected"
                    } else {
                        "Reconnecting"
                    })
                    .size(11)
                    .style(secondary_style),
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center),
            ]
            .spacing(4),
            iced::widget::Space::with_width(Length::Fill),
            button(iconoir_icon(Icon::Settings, 20))
                .on_press(Message::SettingsPressed)
                .width(44)
                .height(44)
                .style(glass::icon_button),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding(12)
    .width(Length::Fill)
    .style(move |theme| {
        glass::panel_mode(
            koko.reduce_transparency,
            koko.high_contrast,
            &koko.surface_tone,
            theme,
        )
    })
    .into()
}

fn rail(koko: &Koko) -> Element<'_, Message> {
    let content = column![button(avatar_view(koko, 40))
        .on_press(Message::ProfileMenuToggled)
        .width(40)
        .height(40)
        .style(glass::icon_button),]
    .spacing(8);
    let content = content
        .push(iced::widget::Space::with_height(12))
        .push(rail_item(
            Icon::Home,
            "Chats",
            koko.screen == Screen::Chats,
            Message::ChatsPressed,
        ))
        .push(iced::widget::Space::with_height(Length::Fill));
    let content = if koko.screen == Screen::Chats {
        content
            .push(
                button(
                    container(text(if koko.list_panel_open { "‹" } else { "›" }).size(18))
                        .center_x(40)
                        .center_y(40),
                )
                .on_press(Message::ToggleListPanel)
                .width(40)
                .height(40)
                .style(|theme, status| glass::rail_item(false, theme, status)),
            )
            .push(iced::widget::Space::with_height(8))
    } else {
        content
    };
    let content = content.push(rail_item(
        Icon::Settings,
        "Settings",
        koko.screen == Screen::Settings,
        Message::SettingsPressed,
    ));
    container(content)
        .width(56)
        .height(Length::Fill)
        .padding(8)
        .style(move |theme| {
            glass::panel_mode(
                koko.reduce_transparency,
                koko.high_contrast,
                &koko.surface_tone,
                theme,
            )
        })
        .into()
}

fn profile_menu(koko: &Koko) -> Element<'_, Message> {
    if !koko.profile_menu_open {
        return iced::widget::Space::with_width(0).into();
    }
    container(
        column![
            text(koko.me.as_ref().map_or("Account", |me| &me.user.handle))
                .size(12)
                .style(muted_style),
            text("Profile menu").size(14),
            button(text("Settings").size(12))
                .on_press(Message::SettingsPressed)
                .style(glass::menu_item),
            button(text("Sign out").size(12))
                .on_press(Message::SignOutPressed)
                .style(glass::menu_item),
        ]
        .spacing(6),
    )
    .width(180)
    .padding(12)
    .style(glass::menu)
    .into()
}

fn rail_item(
    symbol: Icon,
    _accessible_name: &'static str,
    selected: bool,
    message: Message,
) -> Element<'static, Message> {
    button(
        container(iconoir_icon(symbol, 18))
            .center_x(40)
            .center_y(40),
    )
    .on_press(message)
    .width(40)
    .height(40)
    .style(move |theme, status| glass::rail_item(selected, theme, status))
    .into()
}

#[allow(clippy::too_many_lines)]
fn chat_list(koko: &Koko) -> Element<'_, Message> {
    let mut rows = column![].spacing(if koko.compact_chat_list { 2 } else { 6 });
    let my_handle = koko.me.as_ref().map(|me| me.user.handle.clone());
    let query = koko.chat_search.trim().to_lowercase();
    let mut visible = 0;
    for chat in &koko.chats {
        let title = chat_title(chat, my_handle.as_deref());
        let name_match = query.is_empty() || title.to_lowercase().contains(&query);
        let message_match = !query.is_empty()
            && koko
                .store
                .as_ref()
                .is_some_and(|store| store.chat_contains_text(chat.id, &query));
        let filter_match = match koko.chat_filter {
            ChatFilter::All => true,
            ChatFilter::Unread => chat.unread > 0,
            ChatFilter::Groups => chat.member_count > 2 || chat.title.is_some(),
        };
        if !(filter_match && (name_match || message_match)) {
            continue;
        }
        visible += 1;
        let selected = koko.active.as_ref().map(|a| a.id) == Some(chat.id);
        let preview = koko
            .store
            .as_ref()
            .and_then(|store| store.latest_preview(chat.id))
            .unwrap_or_else(|| "No messages yet".into());
        rows = rows.push(chat_item(
            chat,
            selected,
            my_handle.clone(),
            koko.compact_chat_list,
            preview,
        ));
    }
    if visible == 0 {
        rows = rows.push(
            container(
                text(if koko.chats.is_empty() {
                    "No chats yet. Start one below."
                } else {
                    "No chats match this view."
                })
                .size(12)
                .style(secondary_style),
            )
            .padding(14)
            .style(glass::content_card),
        );
    }
    let filters = row![
        filter_chip(
            "All",
            ChatFilter::All,
            koko.chat_filter,
            koko.compact_layout,
        ),
        filter_chip(
            "Unread",
            ChatFilter::Unread,
            koko.chat_filter,
            koko.compact_layout,
        ),
        filter_chip(
            "Groups",
            ChatFilter::Groups,
            koko.chat_filter,
            koko.compact_layout,
        ),
    ]
    .spacing(5);
    let panel_toggle: Element<'_, Message> = if koko.compact_layout {
        iced::widget::Space::with_width(0).into()
    } else {
        button(text(if koko.list_panel_open { "‹" } else { "›" }).size(18))
            .on_press(Message::ToggleListPanel)
            .padding(7)
            .style(glass::icon_button)
            .into()
    };
    let content = column![
        row![
            column![
                text("Chats").size(type_size(koko, 24)),
                text(format!("{} conversations", koko.chats.len()))
                    .size(11)
                    .style(muted_style),
            ]
            .spacing(2),
            iced::widget::Space::with_width(Length::Fill),
            panel_toggle,
        ]
        .align_y(iced::Alignment::Center),
        text_input("Search chats or messages", &koko.chat_search)
            .id(koko.search_id.clone())
            .on_input(Message::ChatSearchChanged)
            .size(13)
            .padding(if koko.compact_layout { 14 } else { 10 })
            .style(glass::input),
        filters,
        scrollable(rows).height(Length::Fill),
        iced::widget::Space::with_height(6),
        text_input("Start a conversation…", &koko.peer)
            .on_input(Message::PeerChanged)
            .on_submit(Message::OpenDmPressed)
            .size(13)
            .padding(if koko.compact_layout { 14 } else { 10 })
            .style(glass::input),
    ]
    .spacing(10)
    .padding(16);
    let width = if koko.compact_layout {
        Length::Fill
    } else {
        Length::Fixed((320.0 * koko.list_panel_progress).max(1.0))
    };
    container(content)
        .width(width)
        .height(Length::Fill)
        .style(move |theme| {
            glass::panel_mode(
                koko.reduce_transparency,
                koko.high_contrast,
                &koko.surface_tone,
                theme,
            )
        })
        .clip(true)
        .into()
}

fn thread_panel<'a>(koko: &'a Koko, content: Element<'a, Message>) -> Element<'a, Message> {
    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |theme| {
            glass::panel_mode(
                koko.reduce_transparency,
                koko.high_contrast,
                &koko.surface_tone,
                theme,
            )
        })
        .into()
}

fn filter_chip(
    label: &'static str,
    value: ChatFilter,
    active: ChatFilter,
    compact_layout: bool,
) -> Element<'static, Message> {
    button(text(label).size(11))
        .on_press(Message::ChatFilterSelected(value))
        .padding(Padding {
            top: if compact_layout { 11.0 } else { 6.0 },
            right: 10.0,
            bottom: if compact_layout { 11.0 } else { 6.0 },
            left: 10.0,
        })
        .style(move |theme, status| glass::chip(value == active, theme, status))
        .into()
}

/// One row of the chat list. `my_handle` is passed by value so the returned
/// element borrows nothing from the caller's locals.
#[allow(clippy::needless_pass_by_value)]
fn chat_item(
    chat: &crate::api::ChatRow,
    selected: bool,
    my_handle: Option<String>,
    compact: bool,
    preview: String,
) -> Element<'_, Message> {
    let title = chat_title(chat, my_handle.as_deref());
    let when = format_time(chat.last_activity);
    let unread_badge = if chat.unread > 0 {
        container(text(format!("{}", chat.unread)).size(11))
            .padding(Padding {
                top: 2.0,
                bottom: 2.0,
                left: 8.0,
                right: 8.0,
            })
            .style(glass::card)
    } else {
        container(text("").size(11))
    };
    let avatar = title.chars().next().unwrap_or('K').to_ascii_uppercase();
    button(
        row![
            container(text(avatar.to_string()).size(if compact { 12 } else { 14 }))
                .width(if compact { 28 } else { 34 })
                .height(if compact { 28 } else { 34 })
                .center_x(if compact { 28 } else { 34 })
                .center_y(if compact { 28 } else { 34 })
                .style(glass::avatar),
            column![
                row![
                    text(title).size(if compact { 12 } else { 13 }),
                    iced::widget::Space::with_width(Length::Fill),
                    text(when).size(10).style(muted_style),
                ],
                row![
                    text(preview).size(11).style(secondary_style),
                    iced::widget::Space::with_width(Length::Fill),
                    unread_badge,
                ],
            ]
            .spacing(if compact { 1 } else { 3 }),
        ]
        .spacing(9)
        .align_y(iced::Alignment::Center),
    )
    .on_press(Message::ChatSelected(chat.id))
    .width(Length::Fill)
    .style(move |theme, status| {
        let mut style = glass::list_row(theme, status);
        if selected {
            style.background = Some(iced::Background::Color(iced::Color::from_rgba(
                1.0, 1.0, 1.0, 0.14,
            )));
            style.border.width = 1.0;
            style.border.color = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.25);
        }
        style
    })
    .into()
}

fn chat_title(chat: &crate::api::ChatRow, my_handle: Option<&str>) -> String {
    chat.title.clone().unwrap_or_else(|| {
        chat.members
            .iter()
            .filter(|m| my_handle != Some(m.as_str()))
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    })
}

// --------------------------------------------------------------- settings

fn settings(koko: &Koko) -> Element<'_, Message> {
    let content = match koko.settings_section {
        SettingsSection::Profile => settings_profile(koko),
        SettingsSection::Appearance => settings_appearance(koko),
        SettingsSection::Account => settings_account(koko),
        SettingsSection::Ai => settings_ai(koko),
    };
    if koko.compact_layout {
        return column![
            container(
                row![
                    button(text("←").size(18))
                        .on_press(Message::BackToChatsPressed)
                        .width(44)
                        .height(44)
                        .style(glass::ghost),
                    text("Settings").size(20),
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center),
            )
            .padding(8),
            row![
                settings_mobile_tab("Profile", SettingsSection::Profile, koko.settings_section),
                settings_mobile_tab("Look", SettingsSection::Appearance, koko.settings_section),
                settings_mobile_tab("Account", SettingsSection::Account, koko.settings_section),
                settings_mobile_tab("AI", SettingsSection::Ai, koko.settings_section),
            ]
            .spacing(4),
            content,
        ]
        .spacing(8)
        .padding(8)
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    }
    let nav = container(
        column![
            text("SETTINGS").size(11).style(muted_style),
            settings_nav_item("Profile", SettingsSection::Profile, koko.settings_section),
            settings_nav_item(
                "Appearance",
                SettingsSection::Appearance,
                koko.settings_section,
            ),
            settings_nav_item(
                "Account & connection",
                SettingsSection::Account,
                koko.settings_section,
            ),
            settings_nav_item("AI & bots", SettingsSection::Ai, koko.settings_section),
        ]
        .spacing(6)
        .padding(16),
    )
    .width(200)
    .height(Length::Shrink)
    .style(move |theme| {
        glass::panel_mode(
            koko.reduce_transparency,
            koko.high_contrast,
            &koko.surface_tone,
            theme,
        )
    });

    row![rail(koko), profile_menu(koko), nav, content]
        .spacing(8)
        .padding(8)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_y(iced::Alignment::Start)
        .into()
}

fn settings_mobile_tab(
    label: &'static str,
    section: SettingsSection,
    active: SettingsSection,
) -> Element<'static, Message> {
    button(text(label).size(12))
        .on_press(Message::SettingsSectionSelected(section))
        .padding(Padding {
            top: 10.0,
            right: 12.0,
            bottom: 10.0,
            left: 12.0,
        })
        .style(move |theme, status| glass::nav_item(section == active, theme, status))
        .into()
}

fn settings_nav_item(
    label: &'static str,
    section: SettingsSection,
    active: SettingsSection,
) -> Element<'static, Message> {
    button(text(label).size(12))
        .on_press(Message::SettingsSectionSelected(section))
        .width(Length::Fill)
        .padding(Padding {
            top: 10.0,
            right: 12.0,
            bottom: 10.0,
            left: 12.0,
        })
        .style(move |theme, status| glass::nav_item(section == active, theme, status))
        .into()
}

fn settings_header<'a>(koko: &'a Koko, title: &'a str, subtitle: &'a str) -> Element<'a, Message> {
    let heading: Element<'a, Message> = if koko.compact_layout {
        column![
            text(title).size(type_size(koko, 24)),
            text(subtitle).size(12).style(secondary_style),
        ]
        .spacing(2)
        .into()
    } else {
        row![
            button(text("←").size(18))
                .on_press(Message::BackToChatsPressed)
                .style(glass::ghost),
            column![
                text(title).size(type_size(koko, 24)),
                text(subtitle).size(12).style(secondary_style),
            ]
            .spacing(2),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center)
        .into()
    };
    let conversation = column![heading, iced::widget::Space::with_height(10),].into();
    conversation
}

fn settings_profile(koko: &Koko) -> Element<'_, Message> {
    let display_name = text_input("Display name", &koko.settings_display_name)
        .on_input(Message::SettingsDisplayNameChanged)
        .on_submit(Message::SaveProfilePressed)
        .padding(11)
        .style(glass::input);
    let body = column![
        settings_header(koko, "Profile", "Your identity on this server."),
        settings_section(
            "Public profile",
            koko,
            column![
                row![
                    button(avatar_view(koko, 64))
                        .on_press(Message::AvatarPressed)
                        .style(glass::icon_button),
                    column![
                        text("Profile picture").size(13),
                        text("Choose a PNG, JPG, or WebP image.")
                            .size(12)
                            .style(secondary_style),
                    ]
                    .spacing(3),
                ]
                .spacing(12)
                .align_y(iced::Alignment::Center),
                text("Display name").size(13),
                text("Press Enter after editing to sync your name.")
                    .size(12)
                    .style(secondary_style),
                display_name,
                text("Bio").size(13),
                text_input("A short line about you", &koko.settings_bio)
                    .on_input(Message::SettingsBioChanged)
                    .on_submit(Message::SaveProfilePressed)
                    .padding(11)
                    .style(glass::input),
                text(format!(
                    "Handle: @{}",
                    koko.me.as_ref().map_or("", |me| &me.user.handle)
                ))
                .size(12)
                .style(muted_style),
                button(text("Save profile").size(13))
                    .on_press(Message::SaveProfilePressed)
                    .style(glass::accent),
            ]
            .spacing(8),
        ),
    ]
    .spacing(14)
    .max_width(800);
    settings_content(koko, body)
}

fn settings_appearance(koko: &Koko) -> Element<'_, Message> {
    let body = column![
        settings_header(koko, "Appearance", "Calm, readable, and yours."),
        settings_section(
            "Interface",
            koko,
            column![
                switch_row(
                    "Compact chat list",
                    "Use shorter rows when you want more conversations in view.",
                    koko.compact_chat_list,
                    Message::CompactChatListToggled,
                ),
                switch_row(
                    "Show sent times",
                    "Keep delivery times visible below your messages.",
                    koko.show_timestamps,
                    Message::ShowTimestampsToggled,
                ),
                switch_row(
                    "Large type",
                    "Increase the main reading scale.",
                    koko.large_type,
                    Message::LargeTypeToggled,
                ),
                switch_row(
                    "High contrast edges",
                    "Strengthen borders on glass surfaces.",
                    koko.high_contrast,
                    Message::HighContrastToggled,
                ),
                switch_row(
                    "Reduce transparency",
                    "Use solid surfaces instead of translucent ones.",
                    koko.reduce_transparency,
                    Message::ReduceTransparencyToggled,
                ),
            ]
            .spacing(4),
        ),
        settings_section(
            "Surface tone",
            koko,
            row![
                tone_chip("Quiet", "Subtle", &koko.surface_tone),
                tone_chip("Balanced", "Balanced", &koko.surface_tone),
                tone_chip("Strong", "Strong", &koko.surface_tone),
            ]
            .spacing(6),
        ),
    ]
    .spacing(14)
    .max_width(800);
    settings_content(koko, body)
}

fn settings_account(koko: &Koko) -> Element<'_, Message> {
    let body = column![
        settings_header(koko, "Account & connection", "The current server session."),
        settings_section(
            "Connection",
            koko,
            column![
                text(format!(
                    "Signed in as @{}",
                    koko.me.as_ref().map_or("", |me| &me.user.handle)
                ))
                .size(13),
                text(format!("Server: {}", koko.server))
                    .size(12)
                    .style(secondary_style),
                text(if koko.connected {
                    "Live connection"
                } else {
                    "Reconnecting"
                })
                .size(12)
                .style(muted_style),
                button(text("Open notes to self").size(13))
                    .on_press(Message::OpenNotesPressed)
                    .style(glass::ghost),
                button(text("Sign out").size(13))
                    .on_press(Message::SignOutPressed)
                    .style(glass::ghost),
            ]
            .spacing(8),
        ),
    ]
    .spacing(14)
    .max_width(800);
    settings_content(koko, body)
}

fn settings_ai(koko: &Koko) -> Element<'_, Message> {
    let custom_endpoint: Element<'_, Message> = if koko.ai_provider == "Custom" {
        column![
            text("Custom endpoint").size(13),
            text("Use an OpenAI-compatible URL for your own gateway or provider.")
                .size(11)
                .style(secondary_style),
            text_input("https://your-provider.example/v1", &koko.ai_custom_url)
                .on_input(Message::AiCustomUrlChanged)
                .padding(11)
                .style(glass::input),
        ]
        .spacing(6)
        .into()
    } else {
        iced::widget::Space::with_height(0).into()
    };
    let provider = column![
        text("Provider").size(13),
        text("Choose the connector your bot adapter will use.")
            .size(11)
            .style(secondary_style),
        row![
            ai_provider_chip("Ollama", &koko.ai_provider),
            ai_provider_chip("OpenAI", &koko.ai_provider),
            ai_provider_chip("Anthropic", &koko.ai_provider),
            ai_provider_chip("Custom", &koko.ai_provider),
        ]
        .spacing(5)
        .wrap(),
    ]
    .spacing(7);
    let body = column![
        settings_header(koko, "AI & bots", "Configure an assistant that can participate in chats."),
        settings_section(
            "Bot profile",
            koko,
            column![
                text_input("Bot handle", &koko.ai_bot_handle)
                    .on_input(Message::AiBotHandleChanged)
                    .padding(11)
                    .style(glass::input),
                text_input("Model identifier", &koko.ai_model)
                    .on_input(Message::AiModelChanged)
                    .padding(11)
                    .style(glass::input),
                text_input("System guidance", &koko.ai_system_prompt)
                    .on_input(Message::AiSystemPromptChanged)
                    .padding(11)
                    .style(glass::input),
                provider,
                custom_endpoint,
                switch_row(
                    "Enable AI actions",
                    "Allow the configured adapter to answer when explicitly invoked.",
                    koko.ai_enabled,
                    Message::AiEnabledToggled,
                ),
                switch_row(
                    "Show action progress",
                    "Show tool and workflow progress in the chat timeline.",
                    koko.ai_announce_actions,
                    Message::AiAnnounceActionsToggled,
                ),
            ]
            .spacing(9),
        ),
        settings_section(
            "Webhook bridge",
            koko,
            column![
                text("Use a server-side adapter or automation runner for incoming events.")
                    .size(12)
                    .style(secondary_style),
                text_input("https://example.com/koko/webhook", &koko.ai_webhook_url)
                    .on_input(Message::AiWebhookChanged)
                    .padding(11)
                    .style(glass::input),
                text("The client stores this endpoint locally. A server adapter must be installed before live requests are sent.")
                    .size(11)
                    .style(muted_style),
            ]
            .spacing(8),
        ),
        settings_section(
            "Capabilities",
            koko,
            column![
                ai_capability_group("Text & reasoning", TEXT_CAPABILITIES, koko),
                ai_capability_group("Vision & image", VISION_CAPABILITIES, koko),
                ai_capability_group("Audio & speech", AUDIO_CAPABILITIES, koko),
                ai_capability_group("Video & motion", VIDEO_CAPABILITIES, koko),
                ai_capability_group("Actions & agents", ACTION_CAPABILITIES, koko),
            ]
            .spacing(12),
        ),
    ]
    .spacing(14)
    .max_width(800);
    settings_content(koko, body)
}

fn ai_capability_group<'a>(
    title: &'static str,
    capabilities: &'static [&'static str],
    koko: &'a Koko,
) -> Element<'a, Message> {
    let mut chips = row![].spacing(5);
    for capability in capabilities {
        chips = chips.push(
            button(text(*capability).size(11))
                .on_press(Message::AiCapabilityToggled((*capability).into()))
                .padding(Padding {
                    top: 6.0,
                    right: 9.0,
                    bottom: 6.0,
                    left: 9.0,
                })
                .style(move |theme, status| {
                    glass::chip(
                        koko.ai_capabilities.iter().any(|item| item == capability),
                        theme,
                        status,
                    )
                }),
        );
    }
    column![text(title).size(12), chips.width(Length::Fill).wrap()]
        .spacing(5)
        .into()
}

fn ai_provider_chip(label: &'static str, active: &str) -> Element<'static, Message> {
    let selected = label == active;
    button(text(label).size(12))
        .on_press(Message::AiProviderChanged(label.into()))
        .padding(Padding {
            top: 8.0,
            right: 12.0,
            bottom: 8.0,
            left: 12.0,
        })
        .style(move |theme, status| glass::chip(selected, theme, status))
        .into()
}

const TEXT_CAPABILITIES: &[&str] = &[
    "Thinking",
    "Reasoning",
    "Coding",
    "Debugging",
    "Writing",
    "Editing",
    "Summarizing",
    "Translating",
    "Sentiment analysis",
    "Data extraction",
    "Classification",
];
const VISION_CAPABILITIES: &[&str] = &[
    "Image generation",
    "Object detection",
    "Facial recognition",
    "OCR",
    "Image editing",
    "Image captioning",
    "Visual diagnostics",
];
const AUDIO_CAPABILITIES: &[&str] = &[
    "Transcription",
    "Voice synthesis",
    "Voice cloning",
    "Audio translation",
    "Sound classification",
];
const VIDEO_CAPABILITIES: &[&str] = &[
    "Video generation",
    "Video editing",
    "Video tracking",
    "Motion capture",
    "Video summarization",
];
const ACTION_CAPABILITIES: &[&str] = &[
    "Web browsing",
    "API execution",
    "Code execution",
    "Database querying",
    "Workflow automation",
];

fn settings_content<'a>(
    koko: &'a Koko,
    body: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut content = column![body.into()].spacing(14);
    if let Some(error) = &koko.error {
        content = content.push(
            container(text(error).size(12).style(danger_style))
                .padding(10)
                .style(glass::card),
        );
    }
    let content = container(content).width(Length::Fill);
    let content = if koko.compact_layout {
        content
    } else {
        content.max_width(840.0)
    };
    container(scrollable(content).width(Length::Fill))
        .padding(Padding {
            top: if koko.compact_layout {
                12.0
            } else {
                22.0 + (1.0 - koko.settings_transition_progress) * 8.0
            },
            right: if koko.compact_layout { 8.0 } else { 28.0 },
            bottom: 12.0,
            left: if koko.compact_layout { 8.0 } else { 28.0 },
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .into()
}

fn switch_row(
    label: &'static str,
    description: &'static str,
    value: bool,
    toggle: fn(bool) -> Message,
) -> Element<'static, Message> {
    button(
        row![
            column![
                text(label).size(13),
                text(description).size(11).style(secondary_style),
            ]
            .spacing(2),
            iced::widget::Space::with_width(Length::Fill),
            switch_visual(value),
        ]
        .align_y(iced::Alignment::Center)
        .spacing(10),
    )
    .on_press(toggle(!value))
    .width(Length::Fill)
    .padding(Padding {
        top: 8.0,
        right: 8.0,
        bottom: 8.0,
        left: 8.0,
    })
    .style(glass::row_button)
    .into()
}

fn remember_account_row(value: bool) -> Element<'static, Message> {
    button(
        row![
            container(iconoir_icon(Icon::Lock, 18))
                .width(34)
                .height(34)
                .center_x(34)
                .center_y(34)
                .style(glass::card),
            column![
                text("Keep me signed in").size(13),
                text("Saves this account on the device — never your password.")
                    .size(11)
                    .style(secondary_style),
            ]
            .spacing(2),
            iced::widget::Space::with_width(Length::Fill),
            switch_visual(value),
        ]
        .width(Length::Fill)
        .align_y(iced::Alignment::Center)
        .spacing(10),
    )
    .on_press(Message::RememberAccountToggled(!value))
    .width(Length::Fill)
    .padding(10)
    .style(glass::ghost)
    .into()
}

fn switch_visual(value: bool) -> Element<'static, Message> {
    let knob = container(text(""))
        .width(18)
        .height(18)
        .style(move |theme| glass::switch_knob(value, theme));
    let offset = if value { 22 } else { 4 };
    container(row![iced::widget::Space::with_width(offset), knob])
        .width(48)
        .height(26)
        .padding(4)
        .style(move |theme| glass::switch_track(value, theme))
        .into()
}

fn tone_chip(label: &'static str, value: &'static str, active: &str) -> Element<'static, Message> {
    let selected = value == active;
    button(text(label).size(12))
        .on_press(Message::SurfaceToneSelected(value.into()))
        .padding(Padding {
            top: 8.0,
            right: 14.0,
            bottom: 8.0,
            left: 14.0,
        })
        .style(move |theme, status| glass::chip(selected, theme, status))
        .into()
}

fn settings_section<'a>(
    title: &'a str,
    koko: &'a Koko,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(column![text(title).size(14), content.into()].spacing(10))
        .padding(18)
        .width(Length::Fill)
        .style(move |theme| {
            glass::content_card_mode(
                koko.reduce_transparency,
                koko.high_contrast,
                &koko.surface_tone,
                theme,
            )
        })
        .into()
}

fn status_dot(connected: bool) -> Element<'static, Message> {
    let color = if connected {
        STATUS_GREEN
    } else {
        iced::Color::from_rgb(0.42, 0.42, 0.42)
    };
    container(text(""))
        .width(8)
        .height(8)
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(color)),
            border: iced::Border {
                radius: 4.0.into(),
                width: 0.0,
                color,
            },
            ..container::Style::default()
        })
        .into()
}

fn empty_state() -> Element<'static, Message> {
    container(
        column![
            container(text("K").size(34))
                .width(72)
                .height(72)
                .center_x(72)
                .center_y(72)
                .style(glass::card),
            text("No conversation selected").size(20),
            text("Choose one from the list, or start a new conversation.")
                .size(13)
                .style(secondary_style),
        ]
        .spacing(12)
        .align_x(iced::alignment::Alignment::Center),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

fn media_viewer(koko: &Koko) -> Element<'_, Message> {
    let Some((_id, filename, handle)) = &koko.media_viewer else {
        return iced::widget::Space::with_width(0).into();
    };
    let backdrop = mouse_area(
        container(iced::widget::Space::with_width(1))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(glass::scrim),
    )
    .on_press(Message::MediaViewerClosed);
    let panel = container(
        column![
            row![
                column![
                    text("Image preview").size(15),
                    text(filename).size(11).style(muted_style),
                ]
                .spacing(2),
                iced::widget::Space::with_width(Length::Fill),
                button(iconoir_icon(Icon::Close, 18))
                    .on_press(Message::MediaViewerClosed)
                    .style(glass::icon_button),
            ]
            .align_y(iced::Alignment::Center),
            container(
                image(handle.clone())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .content_fit(iced::ContentFit::Contain),
            )
            .width(Length::Fixed(720.0))
            .height(Length::Fixed(560.0))
            .clip(true),
        ]
        .spacing(12),
    )
    .padding(18)
    .width(Length::Fixed(820.0))
    .style(glass::modal);
    container(stack![
        backdrop,
        container(panel)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
    ])
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

// --------------------------------------------------------- conversation

#[allow(clippy::too_many_lines)]
fn conversation<'a>(koko: &'a Koko, active: &'a ActiveChat) -> Element<'a, Message> {
    let header = koko.chats.iter().find(|c| c.id == active.id).map_or_else(
        || "chat".into(),
        |c| c.title.clone().unwrap_or_else(|| c.members.join(", ")),
    );
    let my_id = koko.me.as_ref().map_or(-1, |m| m.user.id);

    let mut list = column![].spacing(10).padding(Padding {
        top: 18.0,
        right: 4.0,
        bottom: 18.0,
        left: 4.0,
    });
    if active.messages.is_empty() {
        list = list.push(
            container(
                text("No messages yet — say hi.")
                    .size(12)
                    .style(muted_style),
            )
            .center_x(Length::Fill)
            .padding(12),
        );
    }
    for m in &active.messages {
        let mine = m.sender_id == my_id;
        let body = String::from_utf8_lossy(&m.body).to_string();
        let label = message_summary(m.kind, &m.body, &body);
        let more = button(iconoir_icon(Icon::More, 16))
            .on_press(Message::MenuPressed(m.clone()))
            .padding(6)
            .style(glass::quiet_icon_button);
        let preview = message_preview(koko, m, label);
        let mut bubble_col =
            column![row![preview, more].align_y(iced::Alignment::Center),].spacing(3);
        if !m.delivered {
            bubble_col = bubble_col.push(text("sending…").size(10).style(muted_style));
        } else if mine && koko.show_timestamps {
            bubble_col = bubble_col.push(text(format_time(m.created_at)).size(10).style(|_| {
                iced::widget::text::Style {
                    color: Some(ACCENT_2),
                }
            }));
        }
        let bubble_body = container(bubble_col)
            .padding(Padding {
                top: 8.0,
                bottom: 8.0,
                left: 14.0,
                right: 14.0,
            })
            .max_width(420)
            .style(move |t| glass::bubble(mine, t));
        let bubble: Element<'_, Message> = bubble_body.into();
        let line: Element<'_, Message> = if mine {
            row![iced::widget::Space::with_width(Length::Fill), bubble].into()
        } else {
            row![bubble, iced::widget::Space::with_width(Length::Fill)].into()
        };
        list = list.push(line);
    }

    let input = text_input("Message", &active.draft)
        .on_input(Message::DraftChanged)
        .on_submit(Message::SendPressed)
        .size(14)
        .padding(if koko.compact_layout { 14 } else { 13 })
        .style(glass::input);

    let attach = button(iconoir_icon(Icon::Plus, 18))
        .on_press(Message::AttachPressed)
        .width(if koko.compact_layout { 44 } else { 40 })
        .height(if koko.compact_layout { 44 } else { 40 })
        .style(glass::icon_button);
    let send = button(iconoir_icon_color(Icon::Send, 18, koko_ui::TEXT_PRIMARY))
        .on_press(Message::SendPressed)
        .padding(Padding {
            top: 11.0,
            right: 16.0,
            bottom: 11.0,
            left: 16.0,
        })
        .height(if koko.compact_layout { 44 } else { 40 })
        .style(glass::send_button);

    let mut composer_content = column![].spacing(8);
    if let Some(reply) = &koko.replying_to {
        composer_content = composer_content.push(reply_composer_card(koko, reply));
    }
    if koko.forwarding.is_some() {
        composer_content = composer_content.push(
            row![
                text("Forward message to").size(12),
                text_input("username", &koko.forward_target)
                    .on_input(Message::ForwardTargetChanged)
                    .on_submit(Message::ForwardSubmit)
                    .padding(8)
                    .style(glass::input),
                button(text("Forward").size(12))
                    .on_press(Message::ForwardSubmit)
                    .style(glass::accent),
                button(text("×").size(15))
                    .on_press(Message::ForwardCancel)
                    .style(glass::ghost),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        );
    }
    composer_content = composer_content.push(row![attach, input, send].spacing(8));
    let composer = container(composer_content).padding(8).style(move |theme| {
        glass::content_card_mode(
            koko.reduce_transparency,
            koko.high_contrast,
            &koko.surface_tone,
            theme,
        )
    });

    let details = if let Some(message) = &koko.message_details {
        message_details_panel(message)
    } else {
        iced::widget::Space::with_height(0).into()
    };

    let heading: Element<'a, Message> = if koko.compact_layout {
        row![
            button(text("←").size(18))
                .on_press(Message::MobileBackPressed)
                .width(44)
                .height(44)
                .style(glass::ghost),
            avatar_view(koko, 32),
            column![
                text(header).size(type_size(koko, 22)),
                row![
                    status_dot(koko.connected),
                    text(if koko.connected {
                        "online"
                    } else {
                        "reconnecting"
                    })
                    .size(11)
                    .style(secondary_style),
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center),
            ]
            .spacing(2),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
        .into()
    } else {
        row![
            avatar_view(koko, 32),
            column![
                text(header).size(type_size(koko, 22)),
                row![
                    status_dot(koko.connected),
                    text(if koko.connected {
                        "online"
                    } else {
                        "reconnecting"
                    })
                    .size(11)
                    .style(secondary_style),
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center),
            ]
            .spacing(2),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center)
        .into()
    };
    let conversation = column![
        heading,
        details,
        container(
            scrollable(container(list).width(Length::Fill).max_width(800.0)).width(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .clip(true),
        iced::widget::Space::with_height(4),
        composer,
    ]
    .spacing(8)
    .height(Length::Fill);
    if koko.message_menu.is_some() {
        stack![conversation, message_actions_popup(koko)]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        conversation.into()
    }
}

fn message_actions_popup(koko: &Koko) -> Element<'_, Message> {
    let Some(message) = &koko.message_menu else {
        return iced::widget::Space::with_width(0).into();
    };
    let mut actions = column![
        text("Message actions").size(11).style(muted_style),
        button(row![iconoir_icon(Icon::Reply, 14), text("Reply").size(11)].spacing(7))
            .on_press(Message::ReplyPressed(message.clone()))
            .width(Length::Fill)
            .style(glass::menu_item),
        button(row![iconoir_icon(Icon::Forward, 14), text("Forward").size(11)].spacing(7))
            .on_press(Message::ForwardPressed(message.clone()))
            .width(Length::Fill)
            .style(glass::menu_item),
        button(text("Details").size(11))
            .on_press(Message::DetailsPressed)
            .width(Length::Fill)
            .style(glass::menu_item),
        button(text("Copy text").size(11))
            .on_press(Message::CopyMessageText(message.clone()))
            .width(Length::Fill)
            .style(glass::menu_item),
        button(text("Delete for me").size(11))
            .on_press(Message::DeletePressed)
            .width(Length::Fill)
            .style(glass::menu_item),
    ]
    .spacing(3)
    .width(Length::Fixed(190.0));
    if is_media_message(message.kind) {
        actions = actions.push(
            button(
                row![
                    iconoir_icon(Icon::Download, 14),
                    text("Save to file").size(11)
                ]
                .spacing(7),
            )
            .on_press(Message::SaveMediaPressed(message.clone()))
            .width(Length::Fill)
            .style(glass::menu_item),
        );
    }
    actions = actions.push(
        button(text("Close").size(11))
            .on_press(Message::MenuDismissed)
            .width(Length::Fill)
            .style(glass::menu_item),
    );
    container(container(actions).padding(8).style(glass::menu))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_right(Length::Fill)
        .align_bottom(Length::Fill)
        .padding(Padding {
            top: 0.0,
            right: 18.0,
            bottom: 72.0,
            left: 0.0,
        })
        .into()
}

fn message_preview<'a>(
    koko: &'a Koko,
    message: &'a MirrorMsg,
    label: String,
) -> Element<'a, Message> {
    if message.kind == koko_protocol::MessageKind::Image.code() {
        if let Some((id, _filename)) = media_reference_view(message) {
            if let Some(handle) = koko.media_previews.get(&id) {
                return button(
                    container(
                        image(handle.clone())
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .content_fit(iced::ContentFit::Contain),
                    )
                    .width(Length::Fixed(300.0))
                    .height(Length::Fixed(180.0))
                    .clip(true),
                )
                .on_press(Message::MediaPreviewRequested(message.clone()))
                .width(Length::Fixed(300.0))
                .height(Length::Fixed(180.0))
                .padding(0)
                .style(glass::media_button)
                .into();
            }
            return button(
                row![
                    iconoir_icon(Icon::Image, 15),
                    text(label).size(type_size(koko, 14))
                ]
                .spacing(7),
            )
            .on_press(Message::MediaPreviewRequested(message.clone()))
            .style(glass::media_button)
            .into();
        }
    }
    if message.kind == koko_protocol::MessageKind::Text.code() {
        if let Some(parts) = reply_parts(&message.body) {
            return reply_message_content(koko, parts);
        }
    }
    text(label).size(type_size(koko, 14)).into()
}

#[derive(Debug, Clone)]
struct ReplyParts {
    sender: String,
    quote: String,
    text: String,
}

fn reply_parts(body: &[u8]) -> Option<ReplyParts> {
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(body) {
        let reply = value.get("reply")?;
        return Some(ReplyParts {
            sender: reply.get("sender")?.as_str()?.to_owned(),
            quote: reply.get("quote")?.as_str()?.to_owned(),
            text: value.get("text")?.as_str()?.to_owned(),
        });
    }
    let value = String::from_utf8_lossy(body);
    let rest = value.strip_prefix("Reply to @")?;
    let (sender, rest) = rest.split_once(":\n> ")?;
    let (quote, text) = rest.split_once('\n')?;
    Some(ReplyParts {
        sender: sender.to_owned(),
        quote: quote.to_owned(),
        text: text.to_owned(),
    })
}

fn reply_message_content(koko: &Koko, parts: ReplyParts) -> Element<'static, Message> {
    container(
        column![
            row![
                iconoir_icon(Icon::Reply, 13),
                text(format!("Reply to @{}", parts.sender)).size(11),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
            container(text(parts.quote).size(11).style(muted_style))
                .padding(Padding {
                    top: 6.0,
                    right: 8.0,
                    bottom: 6.0,
                    left: 8.0,
                })
                .style(glass::reply_quote),
            text(parts.text).size(type_size(koko, 14)),
        ]
        .spacing(5),
    )
    .padding(8)
    .style(glass::reply_quote)
    .into()
}

fn reply_composer_card<'a>(_koko: &'a Koko, reply: &'a MirrorMsg) -> Element<'a, Message> {
    let quote =
        reply_parts(&reply.body).map_or_else(|| reply_text_view(&reply.body), |parts| parts.text);
    container(
        row![
            container(iconoir_icon(Icon::Reply, 16))
                .width(30)
                .height(30)
                .center_x(30)
                .center_y(30)
                .style(glass::card),
            column![
                text(format!("Replying to @{}", reply.sender_handle)).size(11),
                text(quote).size(11).style(secondary_style),
            ]
            .spacing(2),
            iced::widget::Space::with_width(Length::Fill),
            button(iconoir_icon(Icon::Close, 15))
                .on_press(Message::ReplyCancel)
                .style(glass::icon_button),
        ]
        .align_y(iced::Alignment::Center)
        .spacing(8),
    )
    .padding(8)
    .style(glass::reply_quote)
    .into()
}

fn reply_text_view(body: &[u8]) -> String {
    String::from_utf8_lossy(body)
        .lines()
        .next()
        .unwrap_or("message")
        .chars()
        .take(120)
        .collect()
}

fn media_reference_view(message: &MirrorMsg) -> Option<(String, String)> {
    let value = serde_json::from_slice::<serde_json::Value>(&message.body).ok()?;
    let id = value.get("id")?.as_str()?.to_owned();
    let filename = value
        .get("filename")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("image")
        .to_owned();
    Some((id, filename))
}

fn is_media_message(kind: i64) -> bool {
    kind == koko_protocol::MessageKind::Image.code()
        || kind == koko_protocol::MessageKind::File.code()
}

fn message_details_panel(message: &MirrorMsg) -> Element<'_, Message> {
    let body = String::from_utf8_lossy(&message.body);
    let summary = message_summary(message.kind, &message.body, &body);
    container(
        row![
            column![
                text("Message details").size(12),
                text(format!(
                    "From @{} · {}",
                    message.sender_handle,
                    format_time(message.created_at)
                ))
                .size(11)
                .style(secondary_style),
                text(format!(
                    "{} · {}",
                    summary,
                    if message.delivered {
                        "delivered"
                    } else {
                        "sending"
                    }
                ))
                .size(11)
                .style(muted_style),
            ]
            .spacing(3),
            iced::widget::Space::with_width(Length::Fill),
            button(text("Close").size(11))
                .on_press(Message::DetailsClose)
                .style(glass::ghost),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding(10)
    .style(glass::content_card)
    .into()
}

fn message_summary(kind: i64, body: &[u8], fallback: &str) -> String {
    if kind == koko_protocol::MessageKind::Image.code()
        || kind == koko_protocol::MessageKind::File.code()
    {
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(body) {
            let filename = value
                .get("filename")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("attachment");
            let size = value
                .get("size")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0);
            let kind_label = if kind == koko_protocol::MessageKind::Image.code() {
                "Image"
            } else {
                "File"
            };
            return format!("{kind_label} · {filename} · {}", format_bytes(size));
        }
    }
    if kind == koko_protocol::MessageKind::Text.code() {
        if let Some(parts) = reply_parts(body) {
            return format!("Reply to @{}: {}", parts.sender, parts.text);
        }
    }
    fallback.to_owned()
}

fn format_bytes(bytes: i64) -> String {
    let bytes = bytes.max(0);
    if bytes >= 1_048_576 {
        let whole = bytes / 1_048_576;
        let tenth = (bytes % 1_048_576) * 10 / 1_048_576;
        format!("{whole}.{tenth} MB")
    } else if bytes >= 1024 {
        let whole = bytes / 1024;
        let tenth = (bytes % 1024) * 10 / 1024;
        format!("{whole}.{tenth} KB")
    } else {
        format!("{bytes} B")
    }
}

// ---------------------------------------------------------------- helpers

fn type_size(koko: &Koko, base: u16) -> u16 {
    if koko.large_type {
        base.saturating_add(2)
    } else {
        base
    }
}

/// `HH:MM · 4 Mar` from unix seconds (UTC; chrono-free civil-from-days math).
fn format_time(unix: i64) -> String {
    let days = unix.div_euclid(86_400);
    let secs_of_day = unix.rem_euclid(86_400);
    let z = days + 719_468;
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let d = doy - (153 * mp + 2) / 5 + 1;
    let hh = secs_of_day / 3600;
    let mm = (secs_of_day % 3600) / 60;
    format!("{hh:02}:{mm:02} · {d} {}", month(m))
}

fn month(m: i64) -> &'static str {
    [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][(m.clamp(1, 12) - 1) as usize]
}

fn secondary_style(_theme: &iced::Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(TEXT_SECONDARY),
    }
}

fn muted_style(_theme: &iced::Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(TEXT_MUTED),
    }
}

fn danger_style(_theme: &iced::Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(iced::Color::from_rgb(0.95, 0.45, 0.45)),
    }
}

#[cfg(test)]
mod tests {
    use crate::app::{Koko, Screen};

    #[test]
    fn chat_list_can_be_built() {
        let koko = Koko {
            screen: Screen::Chats,
            ..Koko::default()
        };
        drop(koko.view());
    }

    #[test]
    fn compact_chat_list_can_be_built() {
        let koko = Koko {
            screen: Screen::Chats,
            compact_layout: true,
            ..Koko::default()
        };
        drop(koko.view());
    }

    #[test]
    fn compact_settings_can_be_built() {
        let koko = Koko {
            screen: Screen::Settings,
            compact_layout: true,
            ..Koko::default()
        };
        drop(koko.view());
    }
}
