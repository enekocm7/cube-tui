#[cfg(feature = "dashboard")]
use axum::extract::Path;
#[cfg(feature = "dashboard")]
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
#[cfg(feature = "dashboard")]
use axum::response::{IntoResponse, Response};
#[cfg(feature = "dashboard")]
use axum::routing::get;
#[cfg(feature = "dashboard")]
use axum::{Json, Router};
#[cfg(feature = "dashboard")]
use rust_embed::RustEmbed;

#[cfg(feature = "dashboard")]
#[derive(RustEmbed)]
#[folder = "dashboard/dist"]
struct DashboardAssets;

#[cfg(feature = "dashboard")]
fn serve_asset(path: &str) -> Response {
    match DashboardAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path)
                .first_or_octet_stream()
                .to_string();
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(&mime)
                    .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
            );
            (StatusCode::OK, headers, content.data.to_vec()).into_response()
        }
        None => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

#[cfg(feature = "dashboard")]
fn api_sessions() -> Response {
    crate::persistence::load().map_or_else(
        || Json(Vec::<serde_json::Value>::new()).into_response(),
        |sessions| Json(sessions).into_response(),
    )
}

#[cfg(feature = "dashboard")]
pub fn run_dashboard(port: u16) -> ! {
    let rt = crate::utils::runtime::runtime();
    let result = rt.block_on(async move { run_dashboard_async(port).await });
    match result {
        Ok(()) => std::process::exit(0),
        Err(err) => {
            eprintln!("Dashboard error: {err}");
            std::process::exit(1);
        }
    }
}

#[cfg(feature = "dashboard")]
async fn run_dashboard_async(port: u16) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/api/sessions", get(|| async { api_sessions() }))
        .route("/", get(|| async { serve_asset("index.html") }))
        .route(
            "/{*path}",
            get(|Path(path): Path<String>| async move { serve_asset(&path) }),
        );

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let url = format!("http://localhost:{port}");

    println!("Dashboard running at {url}");

    if let Err(e) = open::that(&url) {
        eprintln!("Could not open browser automatically: {e}");
    }

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind to address: {e}"))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {e}"))?;
    Ok(())
}
