#[macro_use]
extern crate actix_web;

use std::{env, io};

use actix_session::{CookieSession, Session};
use actix_web::http::StatusCode;
use actix_web::{guard, middleware, web, App, HttpRequest, HttpResponse, HttpServer, Result};

mod projects;

use projects::*;

/// Resume page
#[get("/api/resume")]
async fn resume() -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("application/pdf")
        .body(&include_bytes!("../static/resume.html")[..]))
}

/// Resume PDF
#[get("/resume.pdf")]
async fn resume_pdf() -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("application/pdf")
        .body(&include_bytes!("../static/resume.pdf")[..]))
}

/// Welcome page
#[get("/api/welcome")]
async fn welcome() -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/welcome.html")))
}

/// Contact page
#[get("/api/contact")]
async fn contact() -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/contact.html")))
}

/// Contact image
#[get("/contact.png")]
async fn contact_image() -> Result<&'static [u8]> {
    Ok(include_bytes!("../static/contact.png"))
}

/// Wasm binding handler
#[get("/api/bindings")]
async fn bindings() -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("application/javascript")
        .body(include_str!("../wasm/pkg/frontend.js")))
}

/// Wasm handler
#[get("/api/wasm")]
async fn frontend_wasm() -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("application/wasm")
        .body(&include_bytes!("../wasm/pkg/frontend_bg.wasm")[..]))
}

/// Coming Soon handler
#[get("/api/coming_soon")]
async fn coming_soon() -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/coming_soon.html")))
}

/// 404 handler
#[get("/api/error_404")]
async fn p404() -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/404.html")))
}

/// Favicon handler
#[get("/favicon")]
async fn favicon() -> Result<&'static [u8]> {
    Ok(include_bytes!("../static/favicon.ico"))
}

/// CSS Normalization
#[get("/normalize.css")]
async fn normalize_css() -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/css")
        .body(&include_str!("../static/normalize.css")[..]))
}

/// Simple index handler
async fn base(session: Session, _req: HttpRequest) -> Result<HttpResponse> {
    // Print content of request if compiled with debug profile.
    #[cfg(debug_assertions)]
    println!("{:?}", _req);

    // Session
    let mut counter = 1;
    if let Some(count) = session.get::<i32>("counter")? {
        println!("SESSION value: {}", count);
        counter = count + 1;
    }

    // Set counter to session
    session.set("counter", counter)?;

    // Response
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/base.html")))
}

/// Handler with path parameters like `/user/{name}/`
async fn with_param(req: HttpRequest, path: web::Path<(String,)>) -> HttpResponse {
    println!("{:?}", req);

    HttpResponse::Ok()
        .content_type("text/plain")
        .body(format!("Hello {}!", path.0))
}

#[actix_rt::main]
async fn main() -> io::Result<()> {
    env::set_var("RUST_LOG", "actix_web=debug,actix_server=info");
    env_logger::init();

    HttpServer::new(|| {
        App::new()
            // Comression middleware
            .wrap(middleware::Compress::default())
            // Cookie session middleware
            .wrap(CookieSession::signed(&[0; 32]).secure(false))
            // Enable logger - always register actix-web Logger middleware last
            .wrap(middleware::Logger::default())
            // Register project "Quadcopter"
            .service(quadcopter)
            .service(quadcopter_overview)
            .service(quadcopter_closeup)
            // Register project "This Website"
            .service(this_website)
            // Register resume page
            .service(resume)
            // Register resume pdf
            .service(resume_pdf)
            // Register welcome page
            .service(welcome)
            // Register contact page
            .service(contact)
            // Register contact image
            .service(contact_image)
            // Register bindings
            .service(bindings)
            // Register wasm
            .service(frontend_wasm)
            // Register favicon
            .service(favicon)
            // Register 404
            .service(coming_soon)
            // Register 404
            .service(p404)
            // Register CSS Normalization
            .service(normalize_css)
            // With path parameters
            .service(web::resource("/user/{name}").route(web::get().to(with_param)))
            // Default
            .default_service(
                web::resource("")
                    .route(web::get().to(base))
                    // All requests that are not `GET`
                    .route(
                        web::route()
                            .guard(guard::Not(guard::Get()))
                            .to(HttpResponse::MethodNotAllowed),
                    ),
            )
    })
    .bind("0.0.0.0:8081")?
    .run()
    .await
}
