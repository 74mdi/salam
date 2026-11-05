//! Application state + update logic. The view lives in `views.rs`.

use iced::{widget::image::Handle as ImageHandle, widget::text_input, Subscription, Task};
use koko_protocol::{ClientFrame, ClientMsgId, MessageKind, ServerFrame};
use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

use crate::api::{self, AuthOk, ChatRow};
use crate::store::MirrorMsg;
use crate::ws::{self, WsEvent};

const COMPACT_LAYOUT_BREAKPOINT: f32 = 760.0;

fn device_name() -> &'static str {
    if cfg!(target_os = "android") {
        "koko-android"
    } else {
        "koko-desktop"
    }
}

fn keyboard_shortcut(
    event: iced::Event,
    status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    if let iced::Event::Window(iced::window::Event::Resized(size)) = &event {
        return Some(Message::WindowResized(size.width));
    }
    if status == iced::event::Status::Captured {
        return None;
    }
    let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, .. }) = event
    else {
        return None;
    };
    match key.as_ref() {
        iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) => {
            Some(Message::MediaViewerClosed)
        }
        iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab) => {
            if modifiers.shift() {
                Some(Message::FocusPrevious)
            } else {
                Some(Message::FocusNext)
            }
        }
        _ if !modifiers.command() => None,
        iced::keyboard::Key::Character("f" | "F") => Some(Message::FocusChatSearch),
        _ => None,
    }
}

/// Every UI event.
#[derive(Debug, Clone)]
pub enum Message {
    // ---- login form
    ServerChanged(String),
    HandleChanged(String),
    PasswordChanged(String),
    RememberAccountToggled(bool),
    SignInPressed,
    CreateAccountPressed,
    Auth(Result<AuthOk, String>),
    FocusNext,
    FocusPrevious,
    // ---- chats
    WindowResized(f32),
    Chats(Result<Vec<ChatRow>, String>),
    ChatsPressed,
    ToggleListPanel,
    MobileBackPressed,
    SidebarTick,
    ChatSearchChanged(String),
    ChatFilterSelected(ChatFilter),
    FocusChatSearch,
    ProfileMenuToggled,
    PeerChanged(String),
    OpenDmPressed,
    DmOpened(Result<i64, String>),
    ChatSelected(i64),
    Bootstrap(Result<Vec<crate::api::BootstrapMsg>, String>),
    // ---- conversation
    DraftChanged(String),
    SendPressed,
    // ---- websocket
    Ws(WsEvent),
    CatchUp(Result<Vec<koko_protocol::Envelope>, String>),
    SignOutPressed,
    SettingsPressed,
    SettingsSectionSelected(SettingsSection),
    BackToChatsPressed,
    OpenNotesPressed,
    NotesOpened(Result<i64, String>),
    SettingsDisplayNameChanged(String),
    SettingsBioChanged(String),
    SaveProfilePressed,
    ProfileUpdated(Result<crate::api::UserBrief, String>),
    AvatarPressed,
    AvatarFilePicked(Option<PickedFile>),
    AvatarUploaded(Result<crate::api::MediaUpload, String>),
    AvatarLoaded(Result<Vec<u8>, String>),
    CompactChatListToggled(bool),
    ShowTimestampsToggled(bool),
    HighContrastToggled(bool),
    ReduceTransparencyToggled(bool),
    LargeTypeToggled(bool),
    SurfaceToneSelected(String),
    AiProviderChanged(String),
    AiCustomUrlChanged(String),
    AiModelChanged(String),
    AiSystemPromptChanged(String),
    AiWebhookChanged(String),
    AiBotHandleChanged(String),
    AiEnabledToggled(bool),
    AiAnnounceActionsToggled(bool),
    AiCapabilityToggled(String),
    AttachPressed,
    FilePicked(Option<PickedFile>),
    MediaUploaded(Result<crate::api::MediaUpload, String>),
    ForwardPressed(MirrorMsg),
    MenuPressed(MirrorMsg),
    MenuDismissed,
    ReplyPressed(MirrorMsg),
    ReplyCancel,
    DeletePressed,
    DetailsPressed,
    DetailsClose,
    SaveMediaPressed(MirrorMsg),
    MediaSaved(Result<String, String>),
    MediaPreviewRequested(MirrorMsg),
    MediaPreviewLoaded {
        id: String,
        filename: String,
        result: Result<Vec<u8>, String>,
    },
    MediaViewerUpgrade(String),
    MediaViewerClosed,
    CopyMessageText(MirrorMsg),
    ForwardTargetChanged(String),
    ForwardCancel,
    ForwardSubmit,
    ForwardChatOpened(Result<i64, String>),
}

/// A file selected locally and ready to upload.
#[derive(Debug, Clone)]
pub struct PickedFile {
    pub name: String,
    pub mime: String,
    pub bytes: Vec<u8>,
}

/// Local filters supported by the current chat API and mirror.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatFilter {
    All,
    Unread,
    Groups,
}

/// Settings sections that are backed by the current client/server contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    Profile,
    Appearance,
    Account,
    Ai,
}

/// Which screen is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Login / register form.
    Login,
    /// Chat list + optional conversation.
    Chats,
    /// Profile and local desktop preferences.
    Settings,
}

/// The active conversation.
pub struct ActiveChat {
    /// Chat id.
    pub id: i64,
    /// Current input bar contents.
    pub draft: String,
    /// Rendered messages (read from the mirror).
    pub messages: Vec<MirrorMsg>,
}

/// The app.
#[allow(clippy::struct_excessive_bools)]
pub struct Koko {
    /// Screen.
    pub screen: Screen,
    /// Use the compact, single-panel layout on narrow windows and phones.
    pub compact_layout: bool,
    /// Login form fields.
    pub server: String,
    /// Login handle.
    pub handle: String,
    /// Login password.
    pub password: String,
    /// Whether to remember the server and handle locally.
    pub remember_account: bool,
    /// Handle of the peer to open a DM with.
    pub peer: String,
    /// Local chat/message search query.
    pub chat_search: String,
    /// Active local chat filter.
    pub chat_filter: ChatFilter,
    /// Whether the profile menu is visible in the rail.
    pub profile_menu_open: bool,
    /// Whether the chat list is the current target state.
    pub list_panel_open: bool,
    /// Animated chat-list width progress, from 0.0 to 1.0.
    pub list_panel_progress: f32,
    /// Current settings section.
    pub settings_section: SettingsSection,
    /// Stable id used by Ctrl+F to focus the chat search field.
    pub search_id: text_input::Id,
    /// Session token after login.
    pub token: Option<String>,
    /// Who we are.
    pub me: Option<AuthOk>,
    /// Chat list.
    pub chats: Vec<ChatRow>,
    /// Open conversation, if any.
    pub active: Option<ActiveChat>,
    /// Handle to the WS connector for outgoing frames.
    pub ws_out: Option<UnboundedSender<ClientFrame>>,
    /// WS connectivity.
    pub connected: bool,
    /// Local SQLite mirror.
    pub store: Option<crate::store::Store>,
    /// Mirror file path.
    pub mirror_path: String,
    /// Transient error banner.
    pub error: Option<String>,
    /// Editable profile field used by Settings.
    pub settings_display_name: String,
    /// Editable public profile biography.
    pub settings_bio: String,
    /// Profile image media reference.
    pub avatar_media_id: Option<String>,
    /// Decoded profile image for the native UI.
    pub avatar_image: Option<ImageHandle>,
    /// Downloaded chat image previews keyed by media id.
    pub media_previews: HashMap<String, ImageHandle>,
    /// Full-resolution image handles keyed by media id.
    pub media_full_previews: HashMap<String, ImageHandle>,
    /// Media ids currently being downloaded for the chat preview.
    pub media_loading: HashSet<String>,
    /// Viewer request waiting for its first preview bytes.
    pub media_viewer_pending: Option<(String, String)>,
    /// Full-screen image viewer state.
    pub media_viewer: Option<(String, String, ImageHandle)>,
    /// Use denser rows in the conversation list.
    pub compact_chat_list: bool,
    /// Show sent times below outgoing messages.
    pub show_timestamps: bool,
    /// Prefer stronger edges and text contrast.
    pub high_contrast: bool,
    /// Replace translucent surfaces with solid ones.
    pub reduce_transparency: bool,
    /// Use the larger type scale throughout the client.
    pub large_type: bool,
    /// Local surface tone label shown in settings.
    pub surface_tone: String,
    /// Current forwarded message, if the forward composer is open.
    pub forwarding: Option<MirrorMsg>,
    /// Message being replied to in the composer.
    pub replying_to: Option<MirrorMsg>,
    /// Message whose action menu is open.
    pub message_menu: Option<MirrorMsg>,
    /// Message whose metadata panel is open.
    pub message_details: Option<MirrorMsg>,
    /// Handle to open when forwarding a message.
    pub forward_target: String,
    /// Kind of the attachment currently uploading.
    pub pending_attachment_kind: Option<MessageKind>,
    /// Provider endpoint label selected for the AI connector.
    pub ai_provider: String,
    /// Model identifier selected for the AI connector.
    pub ai_model: String,
    /// Endpoint used when the Custom AI provider is selected.
    pub ai_custom_url: String,
    /// Editable system guidance for the bot.
    pub ai_system_prompt: String,
    /// Optional incoming webhook URL.
    pub ai_webhook_url: String,
    /// Bot handle used when the server-side bot adapter is enabled.
    pub ai_bot_handle: String,
    /// Whether AI actions are enabled for this client.
    pub ai_enabled: bool,
    /// Whether tool progress is surfaced in the chat timeline.
    pub ai_announce_actions: bool,
    /// Enabled capability labels.
    pub ai_capabilities: Vec<String>,
}

impl Default for Koko {
    #[allow(clippy::too_many_lines)]
    fn default() -> Self {
        let mirror_path = std::env::var("KOKO_MIRROR").unwrap_or_else(|_| "koko-mirror.db".into());
        let store = crate::store::Store::open(&mirror_path).ok();
        let server_fallback = if cfg!(target_os = "android") {
            ""
        } else {
            "http://127.0.0.1:8080"
        };
        let default_server = api::normalize_base_url(
            &std::env::var("KOKO_SERVER").unwrap_or_else(|_| server_fallback.into()),
        );
        let remembered = store
            .as_ref()
            .is_some_and(|s| s.preference("remember_account", false));
        let ai_provider = store.as_ref().map_or_else(
            || "Ollama".into(),
            |s| s.text_preference("ai_provider", "Ollama"),
        );
        let ai_model = store.as_ref().map_or_else(
            || "llama3.2".into(),
            |s| s.text_preference("ai_model", "llama3.2"),
        );
        let ai_custom_url = store
            .as_ref()
            .map_or_else(String::new, |s| s.text_preference("ai_custom_url", ""));
        let ai_system_prompt = store.as_ref().map_or_else(
            || "You are a helpful assistant inside Koko.".into(),
            |s| {
                s.text_preference(
                    "ai_system_prompt",
                    "You are a helpful assistant inside Koko.",
                )
            },
        );
        let ai_webhook_url = store
            .as_ref()
            .map_or_else(String::new, |s| s.text_preference("ai_webhook_url", ""));
        let ai_bot_handle = store.as_ref().map_or_else(
            || "koko-bot".into(),
            |s| s.text_preference("ai_bot_handle", "koko-bot"),
        );
        let ai_enabled = store
            .as_ref()
            .is_some_and(|s| s.preference("ai_enabled", false));
        let ai_announce_actions = store
            .as_ref()
            .is_none_or(|s| s.preference("ai_announce_actions", true));
        let ai_capabilities = store
            .as_ref()
            .and_then(|s| serde_json::from_str(&s.text_preference("ai_capabilities", "")).ok())
            .unwrap_or_else(|| vec!["Thinking".into(), "Reasoning".into(), "Coding".into()]);
        Koko {
            screen: Screen::Login,
            compact_layout: cfg!(target_os = "android"),
            server: if remembered {
                store.as_ref().map_or(default_server.clone(), |s| {
                    s.text_preference("login_server", &default_server)
                })
            } else {
                default_server
            },
            handle: if remembered {
                store
                    .as_ref()
                    .map_or_else(String::new, |s| s.text_preference("login_handle", ""))
            } else {
                String::new()
            },
            password: String::new(),
            remember_account: remembered,
            peer: String::new(),
            chat_search: String::new(),
            chat_filter: ChatFilter::All,
            profile_menu_open: false,
            list_panel_open: true,
            list_panel_progress: 1.0,
            settings_section: SettingsSection::Profile,
            search_id: text_input::Id::new("koko-chat-search"),
            token: None,
            me: None,
            chats: Vec::new(),
            active: None,
            ws_out: None,
            connected: false,
            store,
            mirror_path,
            error: None,
            settings_display_name: String::new(),
            settings_bio: String::new(),
            avatar_media_id: None,
            avatar_image: None,
            media_previews: HashMap::new(),
            media_full_previews: HashMap::new(),
            media_loading: HashSet::new(),
            media_viewer_pending: None,
            media_viewer: None,
            compact_chat_list: false,
            show_timestamps: true,
            high_contrast: false,
            reduce_transparency: false,
            large_type: false,
            surface_tone: "Balanced".into(),
            forwarding: None,
            replying_to: None,
            message_menu: None,
            message_details: None,
            forward_target: String::new(),
            pending_attachment_kind: None,
            ai_provider,
            ai_model,
            ai_custom_url,
            ai_system_prompt,
            ai_webhook_url,
            ai_bot_handle,
            ai_enabled,
            ai_announce_actions,
            ai_capabilities,
        }
    }
}

impl Koko {
    /// iced subscription: the WS connector lives while we hold a token.
    pub fn subscription(&self) -> Subscription<Message> {
        let shortcuts = iced::event::listen_with(keyboard_shortcut);
        let animation = if (self.list_panel_progress - if self.list_panel_open { 1.0 } else { 0.0 })
            .abs()
            > f32::EPSILON
        {
            iced::time::every(Duration::from_millis(16)).map(|_| Message::SidebarTick)
        } else {
            Subscription::none()
        };
        let Some(token) = self.token.clone() else {
            return Subscription::batch(vec![shortcuts, animation]);
        };
        let server = self.server.clone();
        Subscription::batch(vec![
            shortcuts,
            animation,
            Subscription::run_with_id(
                format!("koko-ws-{token}"),
                iced::stream::channel(64, move |output| async move {
                    ws::run(server, token, output).await;
                }),
            ),
        ])
    }

    /// Reload the active conversation from the mirror.
    fn refresh_active(&mut self) {
        if let Some(active) = &mut self.active {
            if let Some(store) = &self.store {
                if let Ok(msgs) = store.chat_messages(active.id) {
                    active.messages = msgs;
                }
            }
        }
    }

    /// Apply an envelope to the mirror (and refresh the open view).
    fn apply_envelope(&mut self, env: &koko_protocol::Envelope) {
        if let Some(store) = &self.store {
            if let Err(e) = store.apply_envelope(env) {
                log::error!("mirror: {e}");
            }
        }
        self.refresh_active();
    }

    /// Run a full REST catch-up from the local cursor.
    fn catch_up(&self) -> Task<Message> {
        let Some(token) = self.token.clone() else {
            return Task::none();
        };
        let since = self.store.as_ref().map_or(0, super::store::Store::max_seq);
        let base = self.server.clone();
        Task::perform(
            async move { api::inbox(&base, &token, since).await },
            Message::CatchUp,
        )
    }

    /// Reload the chat list.
    fn reload_chats(&self) -> Task<Message> {
        let Some(token) = self.token.clone() else {
            return Task::none();
        };
        let base = self.server.clone();
        Task::perform(
            async move { api::chats(&base, &token).await },
            Message::Chats,
        )
    }

    /// UI update.
    #[allow(clippy::too_many_lines)]
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WindowResized(width) => {
                if width.is_finite() && width > 0.0 {
                    self.compact_layout = width < COMPACT_LAYOUT_BREAKPOINT;
                }
                Task::none()
            }
            // ---------------- login form ----------------
            Message::ServerChanged(s) => {
                self.server = api::normalize_base_url(&s);
                Task::none()
            }
            Message::HandleChanged(h) => {
                self.handle = h;
                Task::none()
            }
            Message::PasswordChanged(p) => {
                self.password = p;
                Task::none()
            }
            Message::RememberAccountToggled(value) => {
                self.remember_account = value;
                if let Some(store) = &self.store {
                    if let Err(error) = store.set_preference("remember_account", value) {
                        self.error = Some(format!("local preferences: {error}"));
                    }
                }
                Task::none()
            }
            Message::SignInPressed => {
                let base = self.server.clone();
                let handle = self.handle.clone();
                let password = self.password.clone();
                let device = device_name();
                Task::perform(
                    async move { api::login(&base, &handle, &password, device).await },
                    Message::Auth,
                )
            }
            Message::FocusNext => iced::widget::focus_next(),
            Message::FocusPrevious => iced::widget::focus_previous(),
            Message::CreateAccountPressed => {
                let base = self.server.clone();
                let handle = self.handle.clone();
                let password = self.password.clone();
                let device = device_name();
                Task::perform(
                    async move { api::register_then_login(&base, &handle, &password, device).await },
                    Message::Auth,
                )
            }
            Message::Auth(Ok(auth)) => {
                log::info!("signed in as {}", auth.user.handle);
                self.token = Some(auth.token.clone());
                self.me = Some(auth);
                self.settings_display_name = self
                    .me
                    .as_ref()
                    .map_or_else(String::new, |me| me.user.display_name.clone());
                self.settings_bio = self
                    .me
                    .as_ref()
                    .map_or_else(String::new, |me| me.user.bio.clone());
                self.avatar_media_id = self
                    .me
                    .as_ref()
                    .and_then(|me| me.user.avatar_media_id.clone());
                self.screen = Screen::Chats;
                self.error = None;
                if self.store.is_none() {
                    self.store = crate::store::Store::open(&self.mirror_path).ok();
                }
                if let Some(store) = &self.store {
                    self.compact_chat_list = store.preference("compact_chat_list", false);
                    self.show_timestamps = store.preference("show_timestamps", true);
                    self.high_contrast = store.preference("high_contrast", false);
                    self.reduce_transparency = store.preference("reduce_transparency", false);
                    self.large_type = store.preference("large_type", false);
                    self.surface_tone = store.text_preference("surface_tone", "Balanced");
                    if self.remember_account {
                        if let Err(error) = store.set_text_preference("login_server", &self.server)
                        {
                            self.error = Some(format!("local account: {error}"));
                        }
                        if let Err(error) = store.set_text_preference("login_handle", &self.handle)
                        {
                            self.error = Some(format!("local account: {error}"));
                        }
                    }
                }
                let avatar = self.load_avatar();
                Task::batch(vec![self.reload_chats(), avatar])
            }
            // Every failure path lands in the same error banner.
            Message::Auth(Err(e))
            | Message::Chats(Err(e))
            | Message::DmOpened(Err(e))
            | Message::NotesOpened(Err(e))
            | Message::Bootstrap(Err(e))
            | Message::CatchUp(Err(e))
            | Message::ProfileUpdated(Err(e)) => {
                self.error = Some(e);
                Task::none()
            }
            Message::SettingsPressed => {
                self.settings_display_name = self
                    .me
                    .as_ref()
                    .map_or_else(String::new, |me| me.user.display_name.clone());
                self.screen = Screen::Settings;
                self.profile_menu_open = false;
                Task::none()
            }
            Message::SettingsSectionSelected(section) => {
                self.settings_section = section;
                Task::none()
            }
            Message::BackToChatsPressed | Message::ChatsPressed => {
                self.screen = Screen::Chats;
                self.profile_menu_open = false;
                Task::none()
            }
            Message::MobileBackPressed => {
                self.active = None;
                self.profile_menu_open = false;
                Task::none()
            }
            Message::ToggleListPanel => {
                self.list_panel_open = !self.list_panel_open;
                Task::none()
            }
            Message::SidebarTick => {
                let target = if self.list_panel_open { 1.0 } else { 0.0 };
                let delta = target - self.list_panel_progress;
                if delta.abs() < 0.02 {
                    self.list_panel_progress = target;
                } else {
                    self.list_panel_progress += delta * 0.12;
                }
                Task::none()
            }
            Message::ChatSearchChanged(query) => {
                self.chat_search = query;
                Task::none()
            }
            Message::ChatFilterSelected(filter) => {
                self.chat_filter = filter;
                Task::none()
            }
            Message::FocusChatSearch => {
                if self.screen == Screen::Chats {
                    text_input::focus(self.search_id.clone())
                } else {
                    Task::none()
                }
            }
            Message::ProfileMenuToggled => {
                self.profile_menu_open = !self.profile_menu_open;
                Task::none()
            }
            Message::OpenNotesPressed => {
                let Some(token) = self.token.clone() else {
                    return Task::none();
                };
                let base = self.server.clone();
                Task::perform(
                    async move { api::note_to_self(&base, &token).await },
                    Message::NotesOpened,
                )
            }
            Message::NotesOpened(Ok(chat_id)) => {
                self.screen = Screen::Chats;
                let open = self.open_chat(chat_id);
                Task::batch(vec![self.reload_chats(), open])
            }
            Message::SettingsDisplayNameChanged(name) => {
                self.settings_display_name = name;
                Task::none()
            }
            Message::SettingsBioChanged(bio) => {
                self.settings_bio = bio;
                Task::none()
            }
            Message::SaveProfilePressed => {
                let Some(token) = self.token.clone() else {
                    return Task::none();
                };
                let base = self.server.clone();
                let name = self.settings_display_name.trim().to_owned();
                let bio = self.settings_bio.trim().to_owned();
                let avatar = self.avatar_media_id.clone();
                Task::perform(
                    async move {
                        api::update_profile(&base, &token, &name, &bio, avatar.as_deref()).await
                    },
                    Message::ProfileUpdated,
                )
            }
            Message::ProfileUpdated(Ok(user)) => {
                if let Some(me) = &mut self.me {
                    me.user = user;
                }
                self.settings_bio = self
                    .me
                    .as_ref()
                    .map_or_else(String::new, |me| me.user.bio.clone());
                self.avatar_media_id = self
                    .me
                    .as_ref()
                    .and_then(|me| me.user.avatar_media_id.clone());
                self.error = None;
                self.load_avatar()
            }
            Message::AvatarPressed => Task::perform(pick_image_file(), Message::AvatarFilePicked),
            Message::AvatarFilePicked(None) | Message::FilePicked(None) => Task::none(),
            Message::AvatarFilePicked(Some(file)) => {
                let Some(token) = self.token.clone() else {
                    return Task::none();
                };
                let base = self.server.clone();
                Task::perform(
                    async move {
                        api::upload_media(&base, &token, &file.name, &file.mime, file.bytes).await
                    },
                    Message::AvatarUploaded,
                )
            }
            Message::AvatarUploaded(Ok(media)) => {
                self.avatar_media_id = Some(media.id);
                self.save_profile()
            }
            Message::AvatarUploaded(Err(error))
            | Message::ForwardChatOpened(Err(error))
            | Message::MediaSaved(Err(error)) => {
                self.error = Some(error);
                Task::none()
            }
            Message::MediaPreviewLoaded {
                id,
                result: Err(error),
                ..
            } => {
                self.media_loading.remove(&id);
                if self
                    .media_viewer_pending
                    .as_ref()
                    .is_some_and(|(pending_id, _)| pending_id == &id)
                {
                    self.media_viewer_pending = None;
                }
                self.error = Some(error);
                Task::none()
            }
            Message::AvatarLoaded(Ok(bytes)) => {
                let bytes = circle_avatar(bytes).unwrap_or_else(|bytes| bytes);
                self.avatar_image = Some(ImageHandle::from_bytes(bytes));
                Task::none()
            }
            Message::AvatarLoaded(Err(error)) => {
                log::debug!("avatar unavailable: {error}");
                Task::none()
            }
            Message::CompactChatListToggled(value) => {
                self.compact_chat_list = value;
                if let Some(store) = &self.store {
                    if let Err(error) = store.set_preference("compact_chat_list", value) {
                        self.error = Some(format!("local preferences: {error}"));
                    }
                }
                Task::none()
            }
            Message::ShowTimestampsToggled(value) => {
                self.show_timestamps = value;
                if let Some(store) = &self.store {
                    if let Err(error) = store.set_preference("show_timestamps", value) {
                        self.error = Some(format!("local preferences: {error}"));
                    }
                }
                Task::none()
            }
            Message::HighContrastToggled(value) => {
                self.high_contrast = value;
                if let Some(store) = &self.store {
                    if let Err(error) = store.set_preference("high_contrast", value) {
                        self.error = Some(format!("local preferences: {error}"));
                    }
                }
                Task::none()
            }
            Message::ReduceTransparencyToggled(value) => {
                self.reduce_transparency = value;
                if let Some(store) = &self.store {
                    if let Err(error) = store.set_preference("reduce_transparency", value) {
                        self.error = Some(format!("local preferences: {error}"));
                    }
                }
                Task::none()
            }
            Message::LargeTypeToggled(value) => {
                self.large_type = value;
                if let Some(store) = &self.store {
                    if let Err(error) = store.set_preference("large_type", value) {
                        self.error = Some(format!("local preferences: {error}"));
                    }
                }
                Task::none()
            }
            Message::SurfaceToneSelected(tone) => {
                self.surface_tone = tone;
                if let Some(store) = &self.store {
                    if let Err(error) =
                        store.set_text_preference("surface_tone", &self.surface_tone)
                    {
                        self.error = Some(format!("local preferences: {error}"));
                    }
                }
                Task::none()
            }
            Message::AiProviderChanged(value) => {
                self.ai_provider = value;
                self.persist_ai_text("ai_provider", &self.ai_provider.clone());
                Task::none()
            }
            Message::AiCustomUrlChanged(value) => {
                self.ai_custom_url = value;
                self.persist_ai_text("ai_custom_url", &self.ai_custom_url.clone());
                Task::none()
            }
            Message::AiModelChanged(value) => {
                self.ai_model = value;
                self.persist_ai_text("ai_model", &self.ai_model.clone());
                Task::none()
            }
            Message::AiSystemPromptChanged(value) => {
                self.ai_system_prompt = value;
                self.persist_ai_text("ai_system_prompt", &self.ai_system_prompt.clone());
                Task::none()
            }
            Message::AiWebhookChanged(value) => {
                self.ai_webhook_url = value;
                self.persist_ai_text("ai_webhook_url", &self.ai_webhook_url.clone());
                Task::none()
            }
            Message::AiBotHandleChanged(value) => {
                self.ai_bot_handle = value;
                self.persist_ai_text("ai_bot_handle", &self.ai_bot_handle.clone());
                Task::none()
            }
            Message::AiEnabledToggled(value) => {
                self.ai_enabled = value;
                self.persist_ai_bool("ai_enabled", value);
                Task::none()
            }
            Message::AiAnnounceActionsToggled(value) => {
                self.ai_announce_actions = value;
                self.persist_ai_bool("ai_announce_actions", value);
                Task::none()
            }
            Message::AiCapabilityToggled(capability) => {
                if self.ai_capabilities.iter().any(|item| item == &capability) {
                    self.ai_capabilities.retain(|item| item != &capability);
                } else {
                    self.ai_capabilities.push(capability);
                }
                if let Some(store) = &self.store {
                    if let Ok(value) = serde_json::to_string(&self.ai_capabilities) {
                        let _ = store.set_text_preference("ai_capabilities", &value);
                    }
                }
                Task::none()
            }
            Message::AttachPressed => Task::perform(pick_any_file(), Message::FilePicked),
            Message::FilePicked(Some(file)) => {
                let Some(token) = self.token.clone() else {
                    return Task::none();
                };
                let base = self.server.clone();
                self.pending_attachment_kind = Some(if file.mime.starts_with("image/") {
                    MessageKind::Image
                } else {
                    MessageKind::File
                });
                Task::perform(
                    async move {
                        api::upload_media(&base, &token, &file.name, &file.mime, file.bytes).await
                    },
                    Message::MediaUploaded,
                )
            }
            Message::MediaUploaded(Ok(media)) => {
                let Some(kind) = self.pending_attachment_kind.take() else {
                    return Task::none();
                };
                let body = serde_json::json!({
                    "id": media.id,
                    "filename": media.filename,
                    "mime": media.mime,
                    "size": media.size,
                })
                .to_string()
                .into_bytes();
                self.send_media(kind, body);
                Task::none()
            }
            Message::MediaUploaded(Err(error)) => {
                self.pending_attachment_kind = None;
                self.error = Some(error);
                Task::none()
            }
            Message::ForwardPressed(message) => {
                self.forwarding = Some(message);
                self.message_menu = None;
                self.forward_target.clear();
                Task::none()
            }
            Message::MenuPressed(message) => {
                self.message_menu = Some(message);
                self.message_details = None;
                Task::none()
            }
            Message::MenuDismissed => {
                self.message_menu = None;
                Task::none()
            }
            Message::ReplyPressed(message) => {
                self.replying_to = Some(message);
                self.message_menu = None;
                Task::none()
            }
            Message::ReplyCancel => {
                self.replying_to = None;
                Task::none()
            }
            Message::DeletePressed => {
                if let (Some(active), Some(message), Some(store)) =
                    (&self.active, self.message_menu.take(), &self.store)
                {
                    if let Err(error) = store.hide_message(active.id, &message) {
                        self.error = Some(format!("delete: {error}"));
                    }
                    self.refresh_active();
                }
                Task::none()
            }
            Message::DetailsPressed => {
                self.message_details = self.message_menu.take();
                Task::none()
            }
            Message::DetailsClose => {
                self.message_details = None;
                Task::none()
            }
            Message::CopyMessageText(message) => {
                self.message_menu = None;
                iced::clipboard::write(copyable_message_text(&message))
            }
            Message::SaveMediaPressed(message) => {
                self.message_menu = None;
                let Some(token) = self.token.clone() else {
                    return Task::none();
                };
                let base = self.server.clone();
                Task::perform(
                    async move { save_media_file(&base, &token, &message).await },
                    Message::MediaSaved,
                )
            }
            Message::MediaSaved(Ok(_path)) => {
                self.error = None;
                Task::none()
            }
            Message::MediaPreviewRequested(message) => {
                let Some((id, filename)) = media_reference(&message) else {
                    self.error = Some("image preview is unavailable".into());
                    return Task::none();
                };
                if let Some(handle) = self.media_previews.get(&id).cloned() {
                    self.media_viewer_pending = None;
                    self.media_viewer = Some((id.clone(), filename.clone(), handle));
                    if self.media_full_previews.contains_key(&id) {
                        return viewer_upgrade_task(id);
                    }
                    if self.media_loading.contains(&id) {
                        return Task::none();
                    }
                }
                let Some(token) = self.token.clone() else {
                    return Task::none();
                };
                self.media_viewer_pending = Some((id.clone(), filename.clone()));
                let base = self.server.clone();
                let request_id = id.clone();
                self.media_loading.insert(id.clone());
                Task::perform(
                    async move { api::download_media(&base, &token, &request_id).await },
                    move |result| Message::MediaPreviewLoaded {
                        id: id.clone(),
                        filename: filename.clone(),
                        result,
                    },
                )
            }
            Message::MediaPreviewLoaded {
                id,
                filename,
                result: Ok(bytes),
            } => {
                self.media_loading.remove(&id);
                let preview = thumbnail_bytes(&bytes).unwrap_or_else(|_| bytes.clone());
                let preview_handle = ImageHandle::from_bytes(preview);
                let full_handle = ImageHandle::from_bytes(bytes);
                let show_viewer = self
                    .media_viewer_pending
                    .as_ref()
                    .is_some_and(|(pending_id, _)| pending_id == &id);
                self.media_previews
                    .insert(id.clone(), preview_handle.clone());
                self.media_full_previews.insert(id.clone(), full_handle);
                if show_viewer {
                    self.media_viewer_pending = None;
                    self.media_viewer = Some((id.clone(), filename, preview_handle));
                    viewer_upgrade_task(id)
                } else {
                    Task::none()
                }
            }
            Message::MediaViewerUpgrade(id) => {
                if let Some((viewer_id, filename, _)) = &self.media_viewer {
                    if viewer_id == &id {
                        if let Some(full) = self.media_full_previews.get(&id) {
                            self.media_viewer = Some((id, filename.clone(), full.clone()));
                        }
                    }
                }
                Task::none()
            }
            Message::MediaViewerClosed => {
                self.media_viewer_pending = None;
                self.media_viewer = None;
                Task::none()
            }
            Message::ForwardTargetChanged(target) => {
                self.forward_target = target;
                Task::none()
            }
            Message::ForwardCancel => {
                self.forwarding = None;
                self.forward_target.clear();
                Task::none()
            }
            Message::ForwardSubmit => {
                let Some(token) = self.token.clone() else {
                    return Task::none();
                };
                let base = self.server.clone();
                let target = self.forward_target.trim().to_lowercase();
                if target.is_empty() || self.forwarding.is_none() {
                    return Task::none();
                }
                Task::perform(
                    async move { api::direct_chat(&base, &token, &target).await },
                    Message::ForwardChatOpened,
                )
            }
            Message::ForwardChatOpened(Ok(chat_id)) => {
                if let Some(message) = self.forwarding.take() {
                    self.send_media_to_chat(
                        chat_id,
                        MessageKind::from_code(message.kind),
                        message.body,
                    );
                }
                self.forward_target.clear();
                Task::batch(vec![self.reload_chats(), Task::none()])
            }
            // ---------------- chat list ----------------
            Message::Chats(Ok(chats)) => {
                self.chats = chats;
                Task::none()
            }
            Message::PeerChanged(p) => {
                self.peer = p;
                Task::none()
            }
            Message::OpenDmPressed => {
                let Some(token) = self.token.clone() else {
                    return Task::none();
                };
                let base = self.server.clone();
                let peer = self.peer.clone();
                let peer_trimmed = peer.trim().to_lowercase();
                self.peer.clear();
                Task::perform(
                    async move { api::direct_chat(&base, &token, &peer_trimmed).await },
                    Message::DmOpened,
                )
            }
            Message::DmOpened(Ok(chat_id)) => {
                self.peer.clear();
                let open = self.open_chat(chat_id);
                Task::batch(vec![self.reload_chats(), open])
            }
            Message::ChatSelected(chat_id) => self.open_chat(chat_id),
            Message::Bootstrap(Ok(msgs)) => {
                if let (Some(store), Some(active)) = (&self.store, &self.active) {
                    let chat_id = active.id;
                    for m in &msgs {
                        if let Err(e) = store.apply_bootstrap(chat_id, m) {
                            log::error!("mirror: {e}");
                        }
                    }
                }
                self.refresh_active();
                self.preload_media()
            }

            // ---------------- conversation ----------------
            Message::DraftChanged(d) => {
                if let Some(active) = &mut self.active {
                    active.draft = d;
                }
                Task::none()
            }
            Message::SendPressed => {
                self.send_draft();
                Task::none()
            }

            // ---------------- websocket ----------------
            Message::Ws(WsEvent::Ready(out)) => {
                self.ws_out = Some(out);
                Task::none()
            }
            Message::Ws(WsEvent::Connected { next_seq }) => {
                self.connected = true;
                log::info!("ws: connected at server cursor {next_seq}");
                self.catch_up()
            }
            Message::Ws(WsEvent::Disconnected) => {
                self.connected = false;
                Task::none()
            }
            Message::Ws(WsEvent::Server(frame)) => self.on_server_frame(frame),
            Message::CatchUp(Ok(envelopes)) => {
                for env in &envelopes {
                    self.apply_envelope(env);
                }
                if !envelopes.is_empty() {
                    log::info!("sync: applied {} envelopes", envelopes.len());
                    let cmd = self.reload_chats();
                    return cmd;
                }
                Task::none()
            }
            Message::SignOutPressed => {
                self.token = None;
                self.me = None;
                self.chats.clear();
                self.active = None;
                self.ws_out = None;
                self.connected = false;
                self.store = None;
                self.profile_menu_open = false;
                self.list_panel_open = true;
                self.list_panel_progress = 1.0;
                self.screen = Screen::Login;
                Task::none()
            }
        }
    }

    fn on_server_frame(&mut self, frame: ServerFrame) -> Task<Message> {
        match frame {
            ServerFrame::Envelope { env } => {
                self.apply_envelope(&env);
                Task::none()
            }
            ServerFrame::Sent { client_msg_id, seq } => {
                if let Some(store) = &self.store {
                    if let Err(e) = store.mark_delivered_any(client_msg_id.as_bytes(), seq) {
                        log::error!("mirror: {e}");
                    }
                }
                self.refresh_active();
                Task::none()
            }
            ServerFrame::Error { code, message } => {
                log::warn!("server error {:?}: {}", code.code(), message);
                Task::none()
            }
            ServerFrame::Hello { .. } | ServerFrame::Pong | ServerFrame::Typing { .. } => {
                Task::none()
            }
        }
    }

    fn open_chat(&mut self, chat_id: i64) -> Task<Message> {
        let messages = self
            .store
            .as_ref()
            .and_then(|s| s.chat_messages(chat_id).ok())
            .unwrap_or_default();
        self.active = Some(ActiveChat {
            id: chat_id,
            draft: String::new(),
            messages,
        });
        let previews = self.preload_media();
        // REST bootstrap fills any hole in the mirror.
        if let Some(token) = self.token.clone() {
            let base = self.server.clone();
            Task::batch(vec![
                Task::perform(
                    async move { api::chat_messages(&base, &token, chat_id, 0).await },
                    Message::Bootstrap,
                ),
                previews,
            ])
        } else {
            previews
        }
    }

    fn preload_media(&mut self) -> Task<Message> {
        let Some(token) = self.token.clone() else {
            return Task::none();
        };
        let Some(active) = &self.active else {
            return Task::none();
        };
        let base = self.server.clone();
        let mut tasks = Vec::new();
        for message in &active.messages {
            if message.kind != MessageKind::Image.code() {
                continue;
            }
            let Some((id, filename)) = media_reference(message) else {
                continue;
            };
            if self.media_previews.contains_key(&id) || !self.media_loading.insert(id.clone()) {
                continue;
            }
            let request_id = id.clone();
            let request_base = base.clone();
            let request_token = token.clone();
            tasks.push(Task::perform(
                async move { api::download_media(&request_base, &request_token, &request_id).await },
                move |result| Message::MediaPreviewLoaded {
                    id: id.clone(),
                    filename: filename.clone(),
                    result,
                },
            ));
        }
        Task::batch(tasks)
    }

    fn send_draft(&mut self) {
        let Some(active) = &mut self.active else {
            return;
        };
        let text = active.draft.trim().to_owned();
        if text.is_empty() {
            return;
        }
        active.draft.clear();
        let body = if let Some(reply) = self.replying_to.take() {
            let quoted = reply_text(&reply.body)
                .chars()
                .take(120)
                .collect::<String>();
            serde_json::json!({
                "text": text,
                "reply": {
                    "sender": reply.sender_handle,
                    "quote": quoted,
                }
            })
            .to_string()
        } else {
            text
        };
        self.send_media(MessageKind::Text, body.into_bytes());
    }

    fn send_media(&mut self, kind: MessageKind, body: Vec<u8>) {
        let Some(chat_id) = self.active.as_ref().map(|active| active.id) else {
            return;
        };
        self.send_media_to_chat(chat_id, kind, body);
        self.refresh_active();
    }

    fn send_media_to_chat(&mut self, chat_id: i64, kind: MessageKind, body: Vec<u8>) {
        let client_msg_id = koko_core::token::new_client_msg_id();
        let now = koko_core::time::unix_now();
        let me = self
            .me
            .as_ref()
            .map_or((0, "you".into()), |m| (m.user.id, m.user.handle.clone()));

        if let Some(store) = &self.store {
            let pending = MirrorMsg {
                sender_id: me.0,
                sender_handle: me.1.clone(),
                kind: kind.code(),
                body: body.clone(),
                created_at: now,
                client_msg_id: Some(client_msg_id),
                delivered: false,
            };
            if let Err(e) = store.add_pending(chat_id, &pending) {
                log::error!("mirror: {e}");
            }
        }

        let frame = ClientFrame::Send {
            client_msg_id: ClientMsgId::from_bytes(client_msg_id),
            chat: koko_protocol::ChatId(chat_id),
            kind,
            body,
            created_at: now,
        };
        if let Some(out) = &self.ws_out {
            if out.send(frame).is_err() {
                self.error = Some("not connected — message queued for retry".into());
            }
        } else {
            self.error = Some("connecting — message queued for retry".into());
        }
    }

    fn save_profile(&self) -> Task<Message> {
        let Some(token) = self.token.clone() else {
            return Task::none();
        };
        let base = self.server.clone();
        let name = self.settings_display_name.trim().to_owned();
        let bio = self.settings_bio.trim().to_owned();
        let avatar = self.avatar_media_id.clone();
        Task::perform(
            async move { api::update_profile(&base, &token, &name, &bio, avatar.as_deref()).await },
            Message::ProfileUpdated,
        )
    }

    fn load_avatar(&self) -> Task<Message> {
        let (Some(token), Some(media_id)) = (self.token.clone(), self.avatar_media_id.clone())
        else {
            return Task::none();
        };
        let base = self.server.clone();
        Task::perform(
            async move { api::download_media(&base, &token, &media_id).await },
            Message::AvatarLoaded,
        )
    }

    fn persist_ai_text(&self, key: &str, value: &str) {
        if let Some(store) = &self.store {
            if let Err(error) = store.set_text_preference(key, value) {
                log::warn!("ai preference {key}: {error}");
            }
        }
    }

    fn persist_ai_bool(&self, key: &str, value: bool) {
        if let Some(store) = &self.store {
            if let Err(error) = store.set_preference(key, value) {
                log::warn!("ai preference {key}: {error}");
            }
        }
    }
}

fn reply_text(body: &[u8]) -> String {
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(body) {
        if let Some(text) = value.get("text").and_then(serde_json::Value::as_str) {
            return text.to_owned();
        }
    }
    String::from_utf8_lossy(body).to_string()
}

async fn pick_image_file() -> Option<PickedFile> {
    let file = rfd::AsyncFileDialog::new()
        .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
        .pick_file()
        .await?;
    let name = file.file_name();
    Some(PickedFile {
        mime: mime_for_name(&name),
        bytes: file.read().await,
        name,
    })
}

async fn pick_any_file() -> Option<PickedFile> {
    let file = rfd::AsyncFileDialog::new()
        .add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif"])
        .add_filter("Files", &["*"])
        .pick_file()
        .await?;
    let name = file.file_name();
    Some(PickedFile {
        mime: mime_for_name(&name),
        bytes: file.read().await,
        name,
    })
}

async fn save_media_file(base: &str, token: &str, message: &MirrorMsg) -> Result<String, String> {
    let value = serde_json::from_slice::<serde_json::Value>(&message.body)
        .map_err(|_| "this message has no downloadable media".to_owned())?;
    let id = value
        .get("id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "media reference is missing".to_owned())?;
    let filename = value
        .get("filename")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("koko-download");
    let destination = rfd::AsyncFileDialog::new()
        .set_file_name(filename)
        .save_file()
        .await
        .ok_or_else(|| "save cancelled".to_owned())?;
    let bytes = api::download_media(base, token, id).await?;
    tokio::fs::write(destination.path(), bytes)
        .await
        .map_err(|error| format!("save file: {error}"))?;
    Ok(destination.path().display().to_string())
}

fn mime_for_name(name: &str) -> String {
    match name
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
    .into()
}

fn media_reference(message: &MirrorMsg) -> Option<(String, String)> {
    let value = serde_json::from_slice::<serde_json::Value>(&message.body).ok()?;
    let id = value.get("id")?.as_str()?.to_owned();
    let filename = value
        .get("filename")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("image")
        .to_owned();
    Some((id, filename))
}

#[allow(clippy::cast_precision_loss)]
fn circle_avatar(bytes: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
    let decoded = image::load_from_memory(&bytes).map_err(|_| bytes.clone())?;
    let side = decoded.width().min(decoded.height());
    let offset_x = (decoded.width() - side) / 2;
    let offset_y = (decoded.height() - side) / 2;
    let mut avatar = decoded.crop_imm(offset_x, offset_y, side, side).to_rgba8();
    let center = (side as f32 - 1.0) / 2.0;
    let radius = side as f32 / 2.0;
    for (x, y, pixel) in avatar.enumerate_pixels_mut() {
        let dx = x as f32 - center;
        let dy = y as f32 - center;
        if dx.mul_add(dx, dy * dy) > radius * radius {
            pixel[3] = 0;
        }
    }
    let mut output = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(avatar)
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(|_| bytes)?;
    Ok(output.into_inner())
}

fn viewer_upgrade_task(id: String) -> Task<Message> {
    Task::perform(
        async move {
            tokio::time::sleep(Duration::from_millis(140)).await;
            id
        },
        Message::MediaViewerUpgrade,
    )
}

fn thumbnail_bytes(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let decoded = image::load_from_memory(bytes).map_err(|error| error.to_string())?;
    let preview = decoded.thumbnail(960, 720).to_rgb8();
    let mut output = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(preview)
        .write_to(&mut output, image::ImageFormat::Jpeg)
        .map_err(|error| error.to_string())?;
    Ok(output.into_inner())
}

fn copyable_message_text(message: &MirrorMsg) -> String {
    if message.kind == MessageKind::Text.code() {
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&message.body) {
            if let Some(text) = value.get("text").and_then(serde_json::Value::as_str) {
                return text.to_owned();
            }
        }
    }
    String::from_utf8_lossy(&message.body).into_owned()
}

#[cfg(test)]
mod tests {
    use super::{Koko, Message};

    #[test]
    fn resize_switches_between_compact_and_desktop_layouts() {
        let mut app = Koko {
            compact_layout: false,
            ..Koko::default()
        };
        drop(app.update(Message::WindowResized(600.0)));
        assert!(app.compact_layout);

        drop(app.update(Message::WindowResized(f32::NAN)));
        assert!(app.compact_layout);

        drop(app.update(Message::WindowResized(900.0)));
        assert!(!app.compact_layout);
    }
}
