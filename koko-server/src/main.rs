//! `koko-server` — one binary, one SQLite file, zero external services.

use actix_cors::Cors;
use actix_web::dev::Service;
use actix_web::http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, ORIGIN};
use actix_web::{web, App, HttpServer};
use clap::Parser;
use std::io::Write;
use std::sync::Arc;

use koko_server::config::{Cli, Config};
use koko_server::hub::Hub;
use koko_server::state::AppState;

fn is_private_network_preflight(
    method: &actix_web::http::Method,
    requested_private_network: Option<&str>,
    origin: Option<&str>,
    allowed_origins: &[String],
) -> bool {
    method == actix_web::http::Method::OPTIONS
        && requested_private_network == Some("true")
        && origin.is_some_and(|origin| allowed_origins.iter().any(|allowed| allowed == origin))
}

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
    let cors_origins = cfg.cors_origins.clone();
    log::info!("listening on http://{bind}");
    println!(" ready → http://{bind}   ·   ws://{bind}/ws\n");

    HttpServer::new(move || {
        let private_network_origins = cors_origins.clone();
        let cors = cors_origins
            .iter()
            .fold(Cors::default(), |cors, origin| cors.allowed_origin(origin));
        App::new()
            .wrap(
                cors.allowed_methods(["GET", "POST", "PUT", "DELETE", "OPTIONS"])
                    .allowed_headers([ACCEPT, AUTHORIZATION, CONTENT_TYPE])
                    .max_age(3600),
            )
            .wrap_fn(move |req, srv| {
                let allow_private_network = is_private_network_preflight(
                    req.method(),
                    req.headers()
                        .get("access-control-request-private-network")
                        .and_then(|value| value.to_str().ok()),
                    req.headers()
                        .get(ORIGIN)
                        .and_then(|value| value.to_str().ok()),
                    &private_network_origins,
                );
                let future = srv.call(req);
                async move {
                    let mut response = future.await?;
                    if allow_private_network {
                        response.headers_mut().insert(
                            actix_web::http::header::HeaderName::from_static(
                                "access-control-allow-private-network",
                            ),
                            actix_web::http::header::HeaderValue::from_static("true"),
                        );
                    }
                    Ok(response)
                }
            })
            .app_data(state.clone())
            .app_data(web::PayloadConfig::new(cfg.max_file_size as usize + 1024))
            .configure(koko_server::configure_routes)
    })
    .bind(&bind)?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::is_private_network_preflight;
    use actix_web::http::Method;

    #[test]
    fn private_network_preflight_requires_an_allowed_origin() {
        let allowed_origins = vec!["https://74mdi.github.io".to_owned()];

        assert!(is_private_network_preflight(
            &Method::OPTIONS,
            Some("true"),
            Some("https://74mdi.github.io"),
            &allowed_origins,
        ));
        assert!(!is_private_network_preflight(
            &Method::OPTIONS,
            Some("true"),
            Some("https://attacker.example"),
            &allowed_origins,
        ));
        assert!(!is_private_network_preflight(
            &Method::OPTIONS,
            None,
            Some("https://74mdi.github.io"),
            &allowed_origins,
        ));
    }
}
