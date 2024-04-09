#[macro_use]
extern crate actix_web;

use actix_session::{
    config::{CookieContentSecurity, PersistentSession, SessionLifecycle},
    storage::CookieSessionStore,
    SessionMiddleware,
};
use actix_web::{
    body::BoxBody,
    cookie,
    http::{
        header::{CacheControl, CacheDirective},
        StatusCode,
    },
    middleware, web, App, HttpRequest, HttpResponse, HttpServer, Result,
};
use cached::proc_macro::cached;
use clap::Parser;
use config::Config;
use lettre::{transport::smtp::authentication::Credentials, SmtpTransport};
use lru::LruCache;
use mime_guess::from_path;
use rand::{thread_rng, Rng};
use rust_embed::RustEmbed;
use serde::Deserialize;
use std::{
    convert::TryInto,
    env, io,
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};
use time::Duration;

mod captcha;
mod contact;
mod projects;
use crate::captcha::*;
use crate::contact::*;

const SESSION_KEY_LEN: usize = 64;
// Safe because we know it's non-zero. Can remove after
// https://github.com/rust-lang/rust/issues/69329
const CAPTCHA_CACHE_LEN: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(1000) };
const SECONDS_IN_YEAR: usize = 31536000;

#[derive(RustEmbed)]
#[folder = "static/"]
struct Assets;

fn handle_embedded_file(path: &str) -> HttpResponse {
    match Assets::get(path) {
        Some(content) => {
            let body = BoxBody::new(content.data.as_ref().to_owned());
            let content_type = from_path(path).first_or_octet_stream();
            return HttpResponse::Ok()
                .insert_header(CacheControl(vec![
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

async fn dist(path: web::Path<(String,)>) -> HttpResponse {
    handle_embedded_file(&(path.0))
}

/// Basic templating.
#[cached(size = 20)]
fn template_composition(base_path: &'static str, content: &'static str) -> String {
    match Assets::get(base_path) {
        Some(base_file) => {
            let base_bytes: Vec<u8> = base_file.data.as_ref().into();

            match Assets::get(content) {
                Some(content_file) => {
                    let content_bytes: Vec<u8> = content_file.data.as_ref().into();
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
async fn base(_req: HttpRequest) -> HttpResponse {
    // Print content of request if compiled with debug profile.
    #[cfg(debug_assertions)]
    println!("{_req:?}");

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

pub struct SharedAppData {
    captcha_cache: LruCache<[u8; CAPTCHA_ID_LEN], [char; CAPTCHA_LEN]>,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to config.toml file. Defaults to current directory.
    #[arg(short, long, default_value_t = String::from("./config.toml"))]
    config_file: String,
}

#[derive(Deserialize)]
struct Settings {
    data_dir: String,
}

#[derive(Deserialize)]
struct Secrets {
    email_password: String,
    xmr_private_viewkey: String,
    daemon_password: String,
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    env::set_var(
        "RUST_LOG",
        "debug,hyper=info,h2=info,rustls=info,sled=info,acceptxmr=trace",
    );
    env_logger::init();

    let args = Args::parse();
    let secrets = Config::builder()
        .add_source(config::Environment::default())
        .build()
        .unwrap()
        .try_deserialize::<Secrets>()
        .unwrap();

    let settings = Config::builder()
        .add_source(config::File::with_name(&args.config_file))
        .build()
        .unwrap()
        .try_deserialize::<Settings>()
        .unwrap();

    // Set random session key.
    let mut key_arr = [0u8; SESSION_KEY_LEN];
    thread_rng().fill(&mut key_arr[..]);
    let session_key = cookie::Key::generate();

    // Make shared application data object.
    let shared_data = web::Data::new(Mutex::new(SharedAppData {
        captcha_cache: LruCache::new(CAPTCHA_CACHE_LEN),
    }));

    // Make mailer.
    let mailer = Arc::new(
        SmtpTransport::relay("mail.privateemail.com")
            .expect("Could not build mailer")
            .credentials(Credentials::new(
                "donotreply@busyboredom.com".to_string(),
                secrets.email_password.clone(),
            ))
            .build(),
    );

    // Start acceptxmr demo payment gateway.
    let payment_gateway =
        web::Data::new(projects::acceptxmr::setup(mailer.clone(), secrets, settings).await);
    // Wrap mailer for use by actix.
    let wrapped_mailer = web::Data::new(mailer);

    HttpServer::new(move || {
        App::new()
            // Build application data.
            .app_data(wrapped_mailer.clone())
            .app_data(shared_data.clone())
            .app_data(payment_gateway.clone())
            // Comression middleware
            .wrap(middleware::Compress::default())
            // Cookie session middleware
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), session_key.clone())
                    .cookie_content_security(CookieContentSecurity::Private)
                    .cookie_name("busyboredom_private".to_string())
                    .cookie_secure(true)
                    .session_lifecycle(SessionLifecycle::PersistentSession(
                        PersistentSession::default().session_ttl(Duration::days(365)),
                    ))
                    .cookie_same_site(cookie::SameSite::Strict)
                    .build(),
            )
            // Enable logger - always register actix-web Logger middleware last
            .wrap(middleware::Logger::default())
            // Register bindings
            .service(bindings)
            // Register wasm
            .service(frontend_wasm)
            // Register robots.txt
            .service(robots_txt)
            // Contact info for contact page.
            .service(contact_info)
            // Contact form submission
            .service(contact_submitted)
            // Captcha generation
            .service(generate_captcha)
            // Captcha submission
            .service(submit_captcha)
            // AcceptXMR check out endpoint to submit message and prepare cookie.
            .service(projects::acceptxmr::checkout)
            // AcceptXMR gateway to get invoice updates.
            .service(projects::acceptxmr::update)
            // AcceptXMR websocket to get invoice updates.
            .service(projects::acceptxmr::websocket)
            // Static directory
            .service(web::resource("/api/{_:.*}").route(web::get().to(dist)))
            // Default
            .default_service(web::get().to(base))
    })
    .bind("[::]:8081")?
    .run()
    .await
}
