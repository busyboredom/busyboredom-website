use actix_web::http::StatusCode;
use actix_web::{HttpResponse, Result};

/// Project "This Website" page
#[get("/api/projects/this_website")]
async fn this_website() -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../../static/projects/this_website.html")))
}
