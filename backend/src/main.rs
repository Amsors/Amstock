mod auth;
mod error;
mod models;
mod printing;
mod routes;
mod validation;

use auth::AuthState;
use axum::{
    Router,
    http::{HeaderValue, Method},
    middleware,
    routing::{get, post},
};
use printing::{PrintConfig, PrintMode};
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::{env, path::PathBuf};
use tower_http::{cors::CorsLayer, limit::RequestBodyLimitLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
pub struct AppState {
    pool: SqlitePool,
    image_dir: PathBuf,
    print_config: PrintConfig,
    auth: AuthState,
}

#[derive(Debug)]
struct Args {
    print_mode: PrintMode,
    printer_python: Option<PathBuf>,
    printer_script: Option<PathBuf>,
    label_preview_dir: Option<PathBuf>,
    printer_host: String,
    printer_port: u16,
    printer_timeout: f64,
    no_open_label_preview: bool,
    printer_no_cut: bool,
}

impl Args {
    fn help() -> &'static str {
        "Amstock 家用物资管理后端\n\n\
         标签选项：\n\
           --print-mode <preview|printer>   生成并打开 PNG，或连接真实打印机\n\
           --printer-python <PATH>          Python 解释器路径\n\
           --printer-script <PATH>          Rust/Python 桥接脚本路径\n\
           --label-preview-dir <PATH>       PNG 输出目录\n\
           --printer-host <HOST>            打印机地址（默认 192.168.31.114）\n\
           --printer-port <PORT>            打印机端口（默认 9100）\n\
           --printer-timeout <SECONDS>      网络超时（默认 3）\n\
           --no-open-label-preview          生成预览但不调用系统看图程序\n\
           --printer-no-cut                 真实打印后不切纸\n"
    }

    fn parse() -> std::result::Result<Self, String> {
        let mut parsed = Self {
            print_mode: PrintMode::Preview,
            printer_python: None,
            printer_script: None,
            label_preview_dir: None,
            printer_host: "192.168.31.114".into(),
            printer_port: 9100,
            printer_timeout: 3.0,
            no_open_label_preview: false,
            printer_no_cut: false,
        };
        let mut arguments = env::args().skip(1);
        while let Some(argument) = arguments.next() {
            let mut value = || {
                arguments
                    .next()
                    .ok_or_else(|| format!("{argument} 缺少参数值"))
            };
            match argument.as_str() {
                "--help" | "-h" => {
                    print!("{}", Self::help());
                    std::process::exit(0);
                }
                "--version" | "-V" => {
                    println!(env!("CARGO_PKG_VERSION"));
                    std::process::exit(0);
                }
                "--print-mode" => parsed.print_mode = value()?.parse()?,
                "--printer-python" => parsed.printer_python = Some(value()?.into()),
                "--printer-script" => parsed.printer_script = Some(value()?.into()),
                "--label-preview-dir" => parsed.label_preview_dir = Some(value()?.into()),
                "--printer-host" => parsed.printer_host = value()?,
                "--printer-port" => {
                    parsed.printer_port = value()?
                        .parse()
                        .map_err(|_| "--printer-port 必须是 1–65535 的整数".to_string())?
                }
                "--printer-timeout" => {
                    parsed.printer_timeout = value()?
                        .parse()
                        .map_err(|_| "--printer-timeout 必须是秒数".to_string())?
                }
                "--no-open-label-preview" => parsed.no_open_label_preview = true,
                "--printer-no-cut" => parsed.printer_no_cut = true,
                _ => return Err(format!("未知启动参数：{argument}\n\n{}", Self::help())),
            }
        }
        if !parsed.printer_timeout.is_finite() || parsed.printer_timeout <= 0.0 {
            return Err("--printer-timeout 必须大于 0".into());
        }
        if parsed.printer_port == 0 {
            return Err("--printer-port 必须是 1–65535 的整数".into());
        }
        Ok(parsed)
    }
}

#[tokio::main]
async fn main() -> anyhow_main::Result<()> {
    let args = Args::parse()
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error))?;
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "amstock_backend=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url =
        env::var("AMSTOCK_DATABASE_URL").unwrap_or_else(|_| "sqlite://data/amstock.db".into());
    let image_dir =
        PathBuf::from(env::var("AMSTOCK_IMAGE_DIR").unwrap_or_else(|_| "data/images".into()));
    if let Some(path) = database_url.strip_prefix("sqlite://")
        && let Some(parent) = std::path::Path::new(path).parent()
    {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::create_dir_all(&image_dir).await?;
    let options: SqliteConnectOptions = database_url
        .parse::<SqliteConnectOptions>()?
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("backend directory must have a parent")
        .to_path_buf();
    let print_config = PrintConfig {
        mode: args.print_mode,
        python: args
            .printer_python
            .unwrap_or_else(|| project_root.join("printer/.venv/bin/python")),
        script: args
            .printer_script
            .unwrap_or_else(|| project_root.join("printer/amstock_printer.py")),
        output_dir: args
            .label_preview_dir
            .unwrap_or_else(|| project_root.join("backend/data/label-previews")),
        host: args.printer_host,
        port: args.printer_port,
        timeout: args.printer_timeout,
        open_preview: !args.no_open_label_preview,
        cut: !args.printer_no_cut,
    };
    tracing::info!(mode = %print_config.mode, "label printing configured");
    let state = AppState {
        pool,
        image_dir,
        print_config,
        auth: AuthState::from_env()?,
    };
    let protected_api = routes::router()
        .route("/auth/session", get(auth::session))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth,
        ));
    let protected_images = Router::new()
        .route("/images/{serial}", get(routes::get_image))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth,
        ));
    let app = Router::new()
        .route("/api/health", get(|| async { "ok" }))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/logout", post(auth::logout))
        .nest("/api", protected_api)
        .merge(protected_images)
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024))
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(HeaderValue::from_static("http://localhost:43691"))
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                .allow_headers(tower_http::cors::Any),
        )
        .with_state(state);
    let bind = env::var("AMSTOCK_BIND").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!(%bind, "Amstock backend listening");
    axum::serve(listener, app).await?;
    Ok(())
}

mod anyhow_main {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
}
