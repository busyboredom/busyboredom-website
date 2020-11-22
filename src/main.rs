#[macro_use]
extern crate actix_web;

use std::borrow::Cow;
use std::{env, io};

use actix_session::{CookieSession, Session};
use actix_web::body::Body;
use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::http::StatusCode;
use actix_web::{guard, middleware, web, App, HttpRequest, HttpResponse, HttpServer, Result};
use mime_guess::from_path;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "static/"]
struct Asset;

fn handle_embedded_file(path: &str) -> HttpResponse {
    match Asset::get(path) {
        Some(content) => {
            let body: Body = match content {
                Cow::Borrowed(bytes) => bytes.into(),
                Cow::Owned(bytes) => bytes.into(),
            };
            let content_type = from_path(path).first_or_octet_stream();
            // If html, don't cache it for long.
            if content_type
                .subtype()
                .as_str()
                .to_lowercase()
                .contains("html")
            {
                return HttpResponse::Ok()
                    .set(CacheControl(vec![
                        CacheDirective::MaxAge(300u32),
                        CacheDirective::Public,
                    ]))
                    .content_type(content_type.as_ref())
                    .body(body);
            // Otherwise, store it forever (we will change the url when the content changes).
            } else {
                return HttpResponse::Ok()
                    .set(CacheControl(vec![
                        CacheDirective::MaxAge(604800u32),
                        CacheDirective::Public,
                    ]))
                    .content_type(content_type.as_ref())
                    .body(body);
            }
        }
        None => HttpResponse::build(StatusCode::OK)
            .content_type("text/html; charset=utf-8")
            .body(include_str!("../static/404.html")),
    }
}

fn dist(path: web::Path<(String,)>) -> HttpResponse {
    handle_embedded_file(&(path.0).0)
}

/// Simple index handler
async fn base(session: Session, _req: HttpRequest) -> HttpResponse {
    // Print content of request if compiled with debug profile.
    #[cfg(debug_assertions)]
    println!("{:?}", _req);

    // Session
    let mut counter = 1;
    if let Some(count) = session.get::<i32>("counter").unwrap() {
        println!("SESSION value: {}", count);
        counter = count + 1;
    }

    // Set counter to session
    session.set("counter", counter).unwrap();

    match Asset::get("base.html") {
        Some(content) => {
            let base: Vec<u8> = match content {
                Cow::Borrowed(bytes) => bytes.into(),
                Cow::Owned(bytes) => bytes,
            };

            match Asset::get("welcome.html") {
                Some(content) => {
                    let body: Vec<u8> = match content {
                        Cow::Borrowed(bytes) => bytes.into(),
                        Cow::Owned(bytes) => bytes,
                    };
                    let welcome = std::str::from_utf8(&body).unwrap();

                    let response = &std::str::from_utf8(&base)
                        .unwrap()
                        .replace("{{ welcome_content }}", welcome);

                    HttpResponse::Ok()
                        .set(CacheControl(vec![
                            CacheDirective::MaxAge(300u32),
                            CacheDirective::Public,
                        ]))
                        .content_type("text/html; charset=utf-8")
                        .body(response)
                }
                None => panic!("Unable to find welcome.html while building base.html"),
            }
        }
        None => panic!("Unable to find base.html while building base.html"),
    }
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

/// Robots.txt handler
#[get("/robots.txt")]
async fn robots_txt() -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/plain")
        .body(&include_bytes!("../static/robots.txt")[..]))
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
            // Register bindings
            .service(bindings)
            // Register wasm
            .service(frontend_wasm)
            // Register robots.txt
            .service(robots_txt)
            // Static directory
            .service(web::resource("/api/{_:.*}").route(web::get().to(dist)))
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
