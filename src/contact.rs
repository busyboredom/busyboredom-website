use std::sync::{Arc, Mutex};

use actix_session::Session;
use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, Result};
use lettre::{
    message::{Mailbox, MultiPart, SinglePart},
    Message, SmtpTransport, Transport,
};
use log::{error, info, warn};
use serde::Deserialize;

use crate::captcha::*;
use crate::template_composition;
use crate::SharedAppData;

#[derive(Deserialize)]
struct ContactInfoQuery {
    method: String,
}

/// Captcha submission handler
#[get("/api/contact_info")]
async fn contact_info(web::Query(query): web::Query<ContactInfoQuery>) -> Result<HttpResponse> {
    let info = match &query.method[..] {
        "Email" => "charlie@busyboredom.com",
        "Matrix" => "@busyboredom:monero.social",
        "Linkedin" => "https://www.linkedin.com/in/charlie-wilkin-7b6027178/",
        _ => "Not found",
    };

    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/plain; charset=utf-8")
        .body(info))
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
    mailer: web::Data<Arc<SmtpTransport>>,
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
    let maybe_cached_solution: Option<[char; 8]> = match session.get::<[u8; CAPTCHA_ID_LEN]>("captcha_id")
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
    if let Some(cached_solution) = maybe_cached_solution {
        // Make sure the guess matches the session cookie solution and the locally cached one.
        if Some(form.captchachars.clone()) != solution
            || form.captchachars.clone() != cached_solution.iter().collect::<String>()
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
        .from("Contact Form <donotreply@busyboredom.com>".parse().unwrap())
        .to("Charlie Wilkin <charlie@busyboredom.com>".parse().unwrap())
        .subject("Contact Form Submission: ".to_owned() + &form.subject)
        .multipart(
            MultiPart::alternative()
                // Plain text version.
                .singlepart(SinglePart::plain(plain_message))
                // HTML version.
                .singlepart(SinglePart::html(html_message)),
        )
        .expect("failed to build email");

    // Send the email to myself.
    match mailer.send(&email) {
        Ok(_) => info!("Email sent successfully!"),
        Err(e) => {
            error!("Could not send email: {:?}", e);
        }
    }

    // Build an autoreply.
    if let Ok(autoreply_to) =
        format!("{} {} <{}>", form.firstname, form.lastname, form.email).parse::<Mailbox>()
    {
        let autoreply_message = format!(
            "Hello {},\n
        Your message has been received and you can expect a response within
        the next few days. Please have patience if my response time is slow
        (especially on weekdays).",
            form.firstname
        );
        let autoreply = Message::builder()
            .from(
                "Charlie Wilkin (Do Not Reply) <donotreply@busyboredom.com>"
                    .parse()
                    .unwrap(),
            )
            .to(autoreply_to)
            .subject("Auto-Reply for: ".to_owned() + &form.subject)
            .body(autoreply_message)
            .expect("failed to build email");

        // Send the autoreply.
        match mailer.send(&autoreply) {
            Ok(_) => info!("Autoreply sent successfully!"),
            Err(e) => {
                error!("Could not send autoreply: {:?}", e);
            }
        }
    } else {
        error!(
            "Failed to parse email address submitted in contact form: {} {} <{}>",
            form.firstname, form.lastname, form.email
        )
    }

    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(template_composition("base.html", "contact_submitted.html")))
}
