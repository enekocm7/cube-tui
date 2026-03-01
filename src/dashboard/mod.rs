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
pub fn run_dashboard() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    rt.block_on(async {
        let app = Router::new()
            .route("/api/sessions", get(|| async { api_sessions() }))
            .route("/", get(|| async { serve_asset("index.html") }))
            .route(
                "/{*path}",
                get(|Path(path): Path<String>| async move { serve_asset(&path) }),
            );

        let port = 7799_u16;
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
        let url = format!("http://localhost:{port}");

        eprintln!("Dashboard running at {url}");

        if let Err(e) = open::that(&url) {
            eprintln!("Could not open browser automatically: {e}");
        }

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("Failed to bind to address");

        axum::serve(listener, app).await.expect("Server error");
    });
}

