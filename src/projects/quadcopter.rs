use actix_web::http::StatusCode;
use actix_web::{HttpResponse, Result};

/// Project "Quadcopter" page
#[get("/api/projects/quadcopter")]
async fn quadcopter() -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../../static/projects/quadcopter.html")))
}
