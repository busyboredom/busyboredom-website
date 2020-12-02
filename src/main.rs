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
use lettre::message::{header, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use log::{error, info, warn};
use lru::LruCache;
use mime_guess::from_path;
use rand::{thread_rng, Rng};
use rust_embed::RustEmbed;
use serde::Deserialize;

mod captcha;
use crate::captcha::*;

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

#[derive(Deserialize)]
struct ContactForm {
    firstname: String,
    lastname: String,
    email: String,
    subject: String,
    message: String,
    captchachars: String,
}

/// Contact form handler
#[post("/contact-submitted")]
async fn contact_submitted(
    app_data: web::Data<AppData>,
    shared_data: web::Data<Mutex<SharedAppData>>,
    form: web::Form<ContactForm>,
    session: Session,
) -> Result<HttpResponse> {
    // Get solution from session cookie.
    let solution: Option<String> = match session.get("captcha") {
        Ok(answer) => answer,
        Err(_) => None,
    };
    // Get the local cached solution.
    let cached_solution: Option<[char; 8]> = match session.get::<[u8; CAPTCHA_ID_LEN]>("captcha_id")
    {
        Ok(Some(id)) => {
            let cache = &mut shared_data
                .lock()
                .expect("Unable to get lock on captcha cache")
                .captcha_cache;
            match cache.get(&id) {
                Some(&chars) => {
                    info!(
                        "Got captcha ID = {:?} and solution = {:?} in local cache.",
                        id, chars
                    );
                    // Remove the locally cached solution to prevent double submission.
                    cache.pop(&id);
                    Some(chars)
                }
                None => {
                    warn!("No charactars found in captcha cache for ID \"{:?}\"", id);
                    None
                }
            }
        }
        Ok(None) => {
            warn!("No captcha ID in session.");
            None
        }
        Err(_) => {
            warn!("Error retrieving captcha ID from session.");
            None
        }
    };
    // Make sure there IS a local hached solution.
    if cached_solution.is_some() {
        // Make sure the guess matches the session cookie solution and the locally cached one.
        if Some(form.captchachars.clone()) != solution
            || form.captchachars.clone() != cached_solution.unwrap().iter().collect::<String>()
        {
            // Otherwise, fail it and return.
            error!("Could not send email, captcha not passed");
            return Ok(HttpResponse::build(StatusCode::OK)
                .content_type("text/plain; charset=utf-8")
                .body("Captcha response didn't match what the server expected."));
        }
    } else {
        error!("Could not send email, captcha ID/solution not in local cache");
        return Ok(HttpResponse::build(StatusCode::OK)
            .content_type("text/plain; charset=utf-8")
            .body("Captcha response didn't match what the server expected."));
    }

    let html_message = format!(
        "<b>First Name: </b>{}<br>
        <b>Last Name: </b>{}<br>
        <b>Email: </b>{}<br>
        <br>
        <b>Message:</b><br>
        {}",
        form.firstname, form.lastname, form.email, form.message
    );

    let plain_message = format!(
        "First Name: {}\nLast Name: {}\nEmail: {}\n\nMessage:\n{}",
        form.firstname, form.lastname, form.email, form.message
    );

    let email = Message::builder()
        .from("Contact Form <charlie@busyboredom.com>".parse().unwrap())
        .to("Charlie Wilkin <charlie@busyboredom.com>".parse().unwrap())
        .subject("Contact Form Submission: ".to_owned() + &form.subject)
        .multipart(
            MultiPart::alternative()
                // Plain text version.
                .singlepart(
                    SinglePart::eight_bit()
                        .header(header::ContentType(
                            "text/plain; charset=utf8".parse().unwrap(),
                        ))
                        .body(plain_message),
                )
                // HTML version.
                .singlepart(
                    SinglePart::quoted_printable()
                        .header(header::ContentType(
                            "text/html; charset=utf8".parse().unwrap(),
                        ))
                        .body(html_message),
                ),
        )
        .expect("failed to build email");

    // Send the email to myself.
    match app_data.mailer.send(&email) {
        Ok(_) => info!("Email sent successfully!"),
        Err(e) => {
            error!("Could not send email: {:?}", e);
        }
    }

    // Build an autoreply.
    let autoreply_to = format!("{} {} <{}>", form.firstname, form.lastname, form.email);
    let autoreply_message = format!(
        "Hello {},\n
        Your message has been recieved and you can expect a response within the next few days.
        Please have patience if my response time is slow (especially on weekdays).",
        form.firstname
    );
    let autoreply = Message::builder()
        .from("Charlie Wilkin <charlie@busyboredom.com>".parse().unwrap())
        .to(autoreply_to.parse().unwrap())
        .subject("Auto-Reply for: ".to_owned() + &form.subject)
        .body(autoreply_message)
        .expect("failed to build email");

    // Send the autoreply.
    match app_data.mailer.send(&autoreply) {
        Ok(_) => info!("Autoreply sent successfully!"),
        Err(e) => {
            error!("Could not send autoreply: {:?}", e);
        }
    }

    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(template_composition("base.html", "contact_submitted.html")))
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
