//! `koko` — the native client.
//!
//! Run against a localhost server:
//!
//! ```bash
//! cargo run -p koko-client            # then point it at http://127.0.0.1:8080
//! KOKO_SERVER=http://127.0.0.1:8080 cargo run -p koko-client
//! ```

mod api;
mod app;
mod store;
mod views;
mod ws;

use app::Koko;

fn main() -> iced::Result {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    iced::application("koko", Koko::update, Koko::view)
        .theme(|_state| koko_ui::koko_dark())
        .subscription(Koko::subscription)
        .window(iced::window::Settings {
            size: iced::Size::new(1180.0, 780.0),
            ..iced::window::Settings::default()
        })
        .run()
}
