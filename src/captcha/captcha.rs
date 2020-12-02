use std::convert::TryInto;
use std::sync::Mutex;

use actix_session::Session;
use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, Result};
use captcha::{filters, Captcha};
use log::info;
use rand::{thread_rng, Rng};
use serde::Deserialize;

use crate::SharedAppData;

pub const CAPTCHA_LEN: usize = 8;
pub const CAPTCHA_ID_LEN: usize = 16;

/// Captcha generation handler
#[get("/api/generate_captcha")]
pub async fn generate_captcha(
    session: Session,
    app_data: web::Data<Mutex<SharedAppData>>,
) -> Result<HttpResponse> {
    let mut captcha = Captcha::new();
    captcha
        .add_chars(CAPTCHA_LEN.try_into().expect("Captcha too long"))
        .apply_filter(filters::Noise::new(0.3))
        .apply_filter(filters::Wave::new(2.5, 10.0).horizontal())
        .apply_filter(filters::Wave::new(3.0, 10.0).vertical())
        .view(300, 84)
        .apply_filter(filters::Cow::new().min_radius(60).max_radius(70).circles(1))
        .apply_filter(filters::Dots::new(7).min_radius(3).max_radius(5));
    let mut solution: [char; CAPTCHA_LEN] = Default::default();
    solution.copy_from_slice(&captcha.chars()[..]);
    let img = captcha.as_png().expect("Failed to generate captcha PNG");

    // Set random captcha ID.
    let mut id = [0u8; CAPTCHA_ID_LEN];
    thread_rng().fill(&mut id[..]);
    app_data
        .lock()
        .expect("Unable to get lock on captcha cache")
        .captcha_cache
        .put(id, solution);
    info!(
        "Put captcha ID = {:?} and solution = {:?} in local cache",
        id, solution
    );

    // Add captcha solution to private session cookie.
    session
        .set("captcha", captcha.chars_as_string())
        .expect("Unable to add captcha solution to session");

    // Add captcha id to private session cookie.
    session
        .set("captcha_id", id)
        .expect("Unable to add captcha id to session");

    Ok(HttpResponse::build(StatusCode::OK)
        .set(CacheControl(vec![CacheDirective::NoStore]))
        .content_type("image/png")
        .body(img))
}

#[derive(Deserialize)]
struct CaptchaSubmitQuery {
    captcha: String,
}

/// Captcha submission handler
#[get("/api/submit_captcha")]
async fn submit_captcha(
    session: Session,
    web::Query(guess): web::Query<CaptchaSubmitQuery>,
) -> Result<HttpResponse> {
    let mut pass_status = "Fail";

    let answer = match session.get("captcha") {
        Ok(answer) => answer,
        Err(_) => None,
    };
    if Some(guess.captcha) == answer {
        pass_status = "Pass";
    }

    Ok(HttpResponse::build(StatusCode::OK)
        .set(CacheControl(vec![CacheDirective::NoStore]))
        .content_type("text/plain; charset=utf-8")
        .body(pass_status))
}
