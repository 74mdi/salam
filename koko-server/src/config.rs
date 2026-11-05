//! Server configuration: CLI flags with env-var overrides.

use std::path::PathBuf;

use clap::Parser;

/// Koko server — one binary, one SQLite file, zero external services.
#[derive(Parser, Debug, Clone)]
#[command(name = "koko-server", version, about, long_about = None)]
pub struct Cli {
    /// Localhost mode: bind 127.0.0.1 only and open registration.
    #[arg(long)]
    pub localhost: bool,

    /// Socket address to bind (ignored when --localhost is set).
    #[arg(long, short, default_value_t = String::from("0.0.0.0:8080"), env = "KOKO_BIND")]
    pub bind: String,

    /// Path to the SQLite database file.
    #[arg(long, default_value_t = String::from("koko.db"), env = "KOKO_DB")]
    pub db: String,

    /// Directory where media files are stored.
    #[arg(long, default_value_t = String::from("data/media"), env = "KOKO_MEDIA_DIR")]
    pub media_dir: String,

    /// Per-user media quota in bytes (default 4 GiB).
    #[arg(long, default_value_t = 4 * 1024 * 1024 * 1024, env = "KOKO_MEDIA_QUOTA")]
    pub media_quota: i64,

    /// Maximum size of a single uploaded file in bytes (default 25 MiB).
    #[arg(long, default_value_t = 25 * 1024 * 1024, env = "KOKO_MAX_FILE_SIZE")]
    pub max_file_size: i64,

    /// Maximum concurrent WebSocket connections (default 512).
    #[arg(long, default_value_t = 512, env = "KOKO_MAX_CONNECTIONS")]
    pub max_connections: usize,

    /// Allow account registration (implied by --localhost).
    #[arg(long, default_value_t = false, env = "KOKO_OPEN_REGISTRATION")]
    pub open_registration: bool,

    /// Session token lifetime in days (default 30).
    #[arg(long, default_value_t = 30, env = "KOKO_TOKEN_TTL_DAYS")]
    pub token_ttl_days: i64,
}

/// Fully-resolved runtime configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// `true` when running in localhost mode.
    pub localhost: bool,
    /// Effective bind address.
    pub bind: String,
    /// Database file path.
    pub db_path: PathBuf,
    /// Media directory.
    pub media_dir: PathBuf,
    /// Per-user media quota (bytes).
    pub media_quota: i64,
    /// Max single file (bytes).
    pub max_file_size: i64,
    /// Max concurrent WS connections.
    pub max_connections: usize,
    /// Whether registration is open.
    pub open_registration: bool,
    /// Session lifetime (seconds).
    pub token_ttl_secs: i64,
}

impl Config {
    /// Resolve flags into runtime config.
    pub fn from_cli(cli: Cli) -> Self {
        let localhost = cli.localhost;
        // Localhost mode is loopback-only, but an explicit --bind port is
        // still honored (host part is discarded).
        let bind = if localhost {
            let port = cli
                .bind
                .rsplit(':')
                .next()
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(8080);
            format!("127.0.0.1:{port}")
        } else {
            cli.bind
        };
        Config {
            localhost,
            bind,
            db_path: PathBuf::from(cli.db),
            media_dir: PathBuf::from(cli.media_dir),
            media_quota: cli.media_quota,
            max_file_size: cli.max_file_size,
            max_connections: cli.max_connections,
            open_registration: localhost || cli.open_registration,
            token_ttl_secs: cli.token_ttl_days * 86_400,
        }
    }

    /// A defaults config pointing at `dir` (used by tests).
    pub fn for_test(dir: &std::path::Path) -> Self {
        Config {
            localhost: false,
            bind: "127.0.0.1:0".into(),
            db_path: dir.join("koko.db"),
            media_dir: dir.join("media"),
            media_quota: 64 * 1024 * 1024,
            max_file_size: 25 * 1024 * 1024,
            max_connections: 512,
            open_registration: true,
            token_ttl_secs: 86_400,
        }
    }

    /// Human-readable summary for the startup banner.
    pub fn summary(&self) -> String {
        format!(
            " mode          {}\n bind          {}\n database      {}\n media         {} (quota {}, max file {})\n connections   {} max\n registration  {}",
            if self.localhost { "localhost (loopback only)" } else { "server" },
            self.bind,
            self.db_path.display(),
            self.media_dir.display(),
            humansize(self.media_quota),
            humansize(self.max_file_size),
            self.max_connections,
            if self.open_registration { "open" } else { "closed" },
        )
    }
}

/// Tiny byte formatter (no external deps) — "4.0 GiB", "25.0 MiB".
#[allow(clippy::cast_precision_loss)] // display-only
fn humansize(bytes: i64) -> String {
    const KIB: f64 = 1024.0;
    let b = bytes as f64;
    let (val, unit) = if b >= KIB * KIB * KIB {
        (b / (KIB * KIB * KIB), "GiB")
    } else if b >= KIB * KIB {
        (b / (KIB * KIB), "MiB")
    } else if b >= KIB {
        (b / KIB, "KiB")
    } else {
        (b, "B")
    };
    if unit == "B" {
        format!("{bytes} B")
    } else {
        format!("{val:.1} {unit}")
    }
}
