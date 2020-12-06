#[macro_use]
extern crate actix_web;

use std::borrow::Cow;
use std::convert::TryInto;
use std::sync::Mutex;
use std::{env, io};

use actix_session::{CookieSession, Session};
use actix_web::body::Body;
use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::http::StatusCode;
use actix_web::{
    cookie, guard, middleware, web, App, HttpRequest, HttpResponse, HttpServer, Result,
};
use cached::proc_macro::cached;
use lettre::transport::smtp::authentication::Credentials;
use lettre::SmtpTransport;
use lru::LruCache;
use mime_guess::from_path;
use rand::{thread_rng, Rng};
use rust_embed::RustEmbed;

mod captcha;
mod contact;
use crate::captcha::*;
use crate::contact::*;

const SESSION_KEY_LEN: usize = 64;
const CAPTCHA_CACHE_LEN: usize = 1000;
const SECONDS_IN_YEAR: usize = 31536000;

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
            return HttpResponse::Ok()
                .set(CacheControl(vec![
                    CacheDirective::MaxAge(SECONDS_IN_YEAR.try_into().unwrap()),
                    CacheDirective::Public,
                ]))
                .content_type(content_type.as_ref())
                .body(body);
        }
        None => HttpResponse::build(StatusCode::OK)
            .content_type("text/html; charset=utf-8")
            .body(include_str!("../static/404.html")),
    }
}

fn dist(path: web::Path<(String,)>) -> HttpResponse {
    handle_embedded_file(&(path.0).0)
}

/// Basic templating.
#[cached(size = 20)]
fn template_composition(base: &'static str, content: &'static str) -> String {
    match Asset::get(base) {
        Some(base_file) => {
            let base_bytes: Vec<u8> = match base_file {
                Cow::Borrowed(bytes) => bytes.into(),
                Cow::Owned(bytes) => bytes,
            };

            match Asset::get(content) {
                Some(content_file) => {
                    let content_bytes: Vec<u8> = match content_file {
                        Cow::Borrowed(bytes) => bytes.into(),
                        Cow::Owned(bytes) => bytes,
                    };
                    let content_str = std::str::from_utf8(&content_bytes).unwrap();

                    return std::str::from_utf8(&base_bytes)
                        .unwrap()
                        .replace("<p id=\"placeholder\"></p>", content_str);
                }
                None => panic!("Unable to find embedded content file"),
            }
        }
        None => panic!("Unable to find embedded base file"),
    }
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

    handle_embedded_file("base.html")
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

struct AppData {
    mailer: SmtpTransport,
}

pub struct SharedAppData {
    captcha_cache: LruCache<[u8; CAPTCHA_ID_LEN], [char; CAPTCHA_LEN]>,
}

#[actix_rt::main]
async fn main() -> io::Result<()> {
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    // Retrieve mail password from file outside of repository.
    let mut mail_secret = include_str!("../../secrets/email.txt").to_string();
    mail_secret.pop();

    // Set random session key.
    let mut key_arr = [0u8; SESSION_KEY_LEN];
    thread_rng().fill(&mut key_arr[..]);
    let session_key: [u8; SESSION_KEY_LEN] = key_arr;

    // Make shared application data object.
    let shared_data = web::Data::new(Mutex::new(SharedAppData {
        captcha_cache: LruCache::new(CAPTCHA_CACHE_LEN),
    }));

    HttpServer::new(move || {
        App::new()
            // Build application data.
            .data(AppData {
                mailer: SmtpTransport::relay("mail.privateemail.com")
                    .expect("Could not build mailer")
                    .credentials(Credentials::new(
                        "charlie@busyboredom.com".to_string(),
                        mail_secret.to_owned(),
                    ))
                    .build(),
            })
            // Build shared application data.
            .app_data(shared_data.clone())
            // Comression middleware
            .wrap(middleware::Compress::default())
            // Cookie session middleware
            .wrap(
                CookieSession::private(&session_key)
                    .name("busyboredom_private")
                    .secure(true)
                    .max_age(SECONDS_IN_YEAR.try_into().unwrap())
                    .same_site(cookie::SameSite::Strict),
            )
            // Enable logger - always register actix-web Logger middleware last
            .wrap(middleware::Logger::default())
            // Register bindings
            .service(bindings)
            // Register wasm
            .service(frontend_wasm)
            // Register robots.txt
            .service(robots_txt)
            // Contact form submission
            .service(contact_submitted)
            // Captcha generation
            .service(generate_captcha)
            // Captcha submission
            .service(submit_captcha)
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
