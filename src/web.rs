use std::sync::Arc;
use std::time::Duration;

use axum::{
    Router,
    extract::State,
    response::Json,
    routing::{get, post},
};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

use crate::config::Settings;
use crate::disc::{Disc, DiscType};
use crate::handbrake::HandbrakeProcess;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DriveStatus {
    pub device: String,
    pub disc_present: bool,
    pub disc_type: Option<DiscType>,
    pub disc_title: Option<String>,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HandbrakeJob {
    pub id: String,
    pub source: String,
    pub destination: String,
    pub progress: f32,
    pub status: String,
    pub started_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemStatus {
    pub drives: Vec<DriveStatus>,
    pub handbrake_jobs: Vec<HandbrakeJob>,
    pub queue_size: usize,
}

#[derive(Clone)]
pub struct AppState {
    pub settings: Settings,
    pub system_status: Arc<RwLock<SystemStatus>>,
    pub handbrake_process: HandbrakeProcess,
}

pub async fn run_web_server(
    settings: Settings,
    handbrake_process: HandbrakeProcess,
) -> Result<(), failure::Error> {
    let system_status = Arc::new(RwLock::new(SystemStatus {
        drives: Vec::new(),
        handbrake_jobs: Vec::new(),
        queue_size: 0,
    }));

    let app_state = AppState {
        settings: settings.clone(),
        system_status: system_status.clone(),
        handbrake_process,
    };

    // Start background task to update system status
    let status_updater = tokio::spawn(update_system_status(app_state.clone()));

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));

    // Check if port is already in use
    if let Err(e) = tokio::net::TcpListener::bind(&addr).await {
        if e.kind() == std::io::ErrorKind::AddrInUse {
            info!("Web interface already running on {}", addr);
            return Ok(());
        }
        return Err(failure::format_err!(
            "Failed to bind to address {}: {}",
            addr,
            e
        ));
    }

    let app = Router::new()
        .route("/", get(serve_app))
        .route("/api/status", get(get_status))
        .route("/api/eject/:device", post(eject_disc))
        .nest_service("/static", ServeDir::new("style"))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| failure::format_err!("Failed to bind to address {}: {}", addr, e))?;

    info!("Web interface available at http://{}", addr);

    // Run the server with graceful shutdown handling
    let server_result = axum::serve(listener, app).await;

    status_updater.abort();

    match server_result {
        Ok(_) => Ok(()),
        Err(e) => {
            warn!("Web server stopped: {}", e);
            Ok(()) // Don't fail the entire rip process if web server stops
        }
    }
}

async fn update_system_status(app_state: AppState) {
    let mut interval = tokio::time::interval(Duration::from_secs(2));

    loop {
        interval.tick().await;

        let mut drives = Vec::new();
        for device in &app_state.settings.options.devices {
            let disc = Disc::new(device);
            let disc_present = tokio::fs::File::open(device).await.is_ok();

            drives.push(DriveStatus {
                device: device.clone(),
                disc_present,
                disc_type: disc.r#type,
                disc_title: if disc_present {
                    Some(disc.title())
                } else {
                    None
                },
                status: if disc_present {
                    match disc.r#type {
                        Some(DiscType::Dvd) | Some(DiscType::BluRay) => "Ready to rip".to_string(),
                        Some(DiscType::Data) => "Data disc".to_string(),
                        Some(DiscType::Music) => "Music disc".to_string(),
                        None => "Unknown disc type".to_string(),
                    }
                } else {
                    "No disc".to_string()
                },
            });
        }

        // Get actual handbrake jobs and queue size
        let handbrake_jobs = app_state
            .handbrake_process
            .get_active_jobs()
            .await
            .into_iter()
            .map(|job| HandbrakeJob {
                id: job.id,
                source: job.source,
                destination: job.destination,
                progress: job.progress,
                status: job.status,
                started_at: job.started_at,
            })
            .collect();

        let queue_size = app_state.handbrake_process.get_queue_size().await;

        let mut status = app_state.system_status.write().await;
        status.drives = drives;
        status.handbrake_jobs = handbrake_jobs;
        status.queue_size = queue_size;
    }
}

async fn serve_app() -> axum::response::Html<String> {
    let html = include_str!("../templates/index.html");
    axum::response::Html(html.to_string())
}

async fn get_status(State(app_state): State<AppState>) -> Json<SystemStatus> {
    let status = app_state.system_status.read().await;
    Json(status.clone())
}

async fn eject_disc(
    axum::extract::Path(device): axum::extract::Path<String>,
    State(_app_state): State<AppState>,
) -> Json<serde_json::Value> {
    let device_path = format!("/dev/{}", device);
    let disc = Disc::new(&device_path);
    crate::disc::eject(&disc).await;

    Json(serde_json::json!({
        "success": true,
        "message": format!("Ejected disc from {}", device_path)
    }))
}
