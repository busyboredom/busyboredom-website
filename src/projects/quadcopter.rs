use actix_web::http::StatusCode;
use actix_web::{HttpResponse, Result};

/// Quadcopter overview image.
#[get("/projects/quadcopter/overview.jpg")]
async fn quadcopter_overview() -> Result<&'static [u8]> {
    Ok(include_bytes!("../../static/projects/quadcopter-images/overview.jpg"))
}

/// Quadcopter  closeup image.
#[get("/projects/quadcopter/closeup.jpg")]
async fn quadcopter_closeup() -> Result<&'static [u8]> {
    Ok(include_bytes!("../../static/projects/quadcopter-images/closeup.jpg"))
}

/// Project "Quadcopter" page.
#[get("/api/projects/quadcopter")]
async fn quadcopter() -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../../static/projects/quadcopter.html")))
}