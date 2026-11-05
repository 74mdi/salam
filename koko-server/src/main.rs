//! `koko-server` — one binary, one SQLite file, zero external services.

use actix_web::{web, App, HttpServer};
use clap::Parser;
use std::io::Write;
use std::sync::Arc;

use koko_server::config::{Cli, Config};
use koko_server::hub::Hub;
use koko_server::state::AppState;

fn print_banner(cfg: &Config) {
    let mut out = std::io::stdout().lock();
    let rule = "─".repeat(60);
    let _ = writeln!(out, "\n{rule}");
    let _ = writeln!(
        out,
        "  koko-server {}  ·  one binary, one file",
        koko_core::KOKO_VERSION
    );
    let _ = writeln!(out, "{rule}");
    let _ = writeln!(out, "{}", cfg.summary());
    let _ = writeln!(out, "{rule}");
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let cli = Cli::parse();
    let cfg = Arc::new(Config::from_cli(cli));
    print_banner(&cfg);

    // Database (creates + migrates on first boot).
    let pool = koko_db::connect(&cfg.db_path)
        .await
        .map_err(std::io::Error::other)?;
    let dropped = koko_db::sessions::delete_expired(&pool, koko_core::time::unix_now())
        .await
        .unwrap_or(0);
    if dropped > 0 {
        log::info!("housekeeping: dropped {dropped} expired sessions");
    }
    tokio::fs::create_dir_all(&cfg.media_dir).await?;

    let state = web::Data::new(AppState::new(pool, Hub::new(), cfg.clone()));

    let bind = cfg.bind.clone();
    log::info!("listening on http://{bind}");
    println!(" ready → http://{bind}   ·   ws://{bind}/ws\n");

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .app_data(web::PayloadConfig::new(cfg.max_file_size as usize + 1024))
            .configure(koko_server::configure_routes)
    })
    .bind(&bind)?
    .run()
    .await
}
