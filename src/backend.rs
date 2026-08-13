use axum::{
    routing::get,
    Router,
    response::IntoResponse,
    http::{Request, StatusCode, header},
    middleware::{self, Next},
};
use std::net::SocketAddr;
use std::env;
use tower_http::services::{ServeDir, ServeFile};

async fn auth_middleware(
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<axum::response::Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    let expected_token = env::var("ADMIN_TOKEN").unwrap_or_default();

    if let Some(auth_header) = auth_header {
        if auth_header.starts_with("Bearer ") {
            let token = &auth_header[7..];
            if token == expected_token {
                return Ok(next.run(req).await);
            }
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

async fn get_config() -> impl IntoResponse {
    "Config OK"
}

pub async fn run() {
    tracing_subscriber::fmt::init();

    // The dist directory created by trunk
    let serve_dir = ServeDir::new("dist")
        .not_found_service(ServeFile::new("dist/index.html"));

    let admin_routes = Router::new()
        .route("/config", get(get_config))
        .route_layer(middleware::from_fn(auth_middleware));

    let app = Router::new()
        .nest("/api/admin", admin_routes)
        .fallback_service(serve_dir);

    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:4407".to_string());
    let addr: SocketAddr = bind_addr.parse().expect("Invalid BIND_ADDR");
    
    tracing::info!("Listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
