use std::time::{Duration, Instant};

use acceptxmr::{
    AcceptXmrError, InvoiceId, PaymentGateway, PaymentGatewayBuilder, Subscriber, SubscriberError,
};
use actix::{Actor, ActorContext, AsyncContext, StreamHandler};
use actix_session::Session;
use actix_web::{get, post, web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use bytestring::ByteString;
use lettre::{message::Mailbox, Message, SmtpTransport, Transport};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Time before lack of client response causes a timeout.
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);
/// Time between sending heartbeat pings.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(1);

pub async fn setup() -> PaymentGateway {
    // Read view key from file.
    let private_view_key = include_str!("../../secrets/xmr_private_view_key.txt")
        .to_string()
        .trim() // Remove line ending.
        .to_owned();

    // No need to keep the public spend key secret.
    let public_spend_key = "dd4c491d53ad6b46cda01ed6cb9bac57615d9eac8d5e4dd1c0363ac8dfd420a7";

    let payment_gateway = PaymentGatewayBuilder::new(&private_view_key, public_spend_key)
        .daemon_url("http://busyboredom.com:18089")
        .build();
    info!("Payment gateway created.");

    payment_gateway
        .run()
        .await
        .expect("failed to run payment gateway");
    info!("Payment gateway running.");

    // Watch for invoice updates and deal with them accordingly.
    let gateway_copy = payment_gateway.clone();
    std::thread::spawn(move || {
        // Watch all invoice updates.
        let mut subscriber = gateway_copy.subscribe_all();
        loop {
            let invoice = match subscriber.recv() {
                Ok(p) => p,
                Err(AcceptXmrError::Subscriber(_)) => panic!("Blockchain scanner crashed!"),
                Err(e) => {
                    error!("Error retrieving invoice update: {}", e);
                    continue;
                }
            };
            // If it's confirmed or expired, we probably shouldn't bother tracking it anymore.
            if (invoice.is_confirmed() && invoice.creation_height() < invoice.current_height())
                || invoice.is_expired()
            {
                debug!(
                    "Invoice to index {} is either confirmed or expired. Removing invoice now",
                    invoice.index()
                );
                if let Err(e) = gateway_copy.remove_invoice(invoice.id()) {
                    error!("Failed to remove fully confirmed invoice: {}", e);
                };
            }
        }
    });
    payment_gateway.clone()
}

#[derive(Deserialize, Serialize)]
struct CheckoutInfo {
    email: String,
    message: String,
}

/// Create new invoice and place cookie.
#[post("/projects/acceptxmr/check_out")]
async fn check_out(
    session: Session,
    checkout_info: web::Json<CheckoutInfo>,
    payment_gateway: web::Data<PaymentGateway>,
) -> Result<&'static str, actix_web::Error> {
    let invoice_id = payment_gateway
        .new_invoice(1, 2, 4, &json!(checkout_info).to_string())
        .await
        .unwrap();
    session.insert("id", invoice_id)?;
    Ok("Success")
}

/// WebSocket rout.
#[get("/projects/acceptxmr/ws/")]
async fn websocket(
    session: Session,
    req: HttpRequest,
    stream: web::Payload,
    payment_gateway: web::Data<PaymentGateway>,
    mailer: web::Data<SmtpTransport>,
) -> Result<HttpResponse, actix_web::Error> {
    let invoice_id = match session.get::<InvoiceId>("id") {
        Ok(Some(i)) => i,
        _ => return Ok(HttpResponse::NotFound().finish()),
    };
    let subscriber = match payment_gateway.subscribe(invoice_id) {
        Ok(Some(s)) => s,
        _ => return Ok(HttpResponse::NotFound().finish()),
    };
    ws::start(
        WebSocket::new(subscriber, mailer.get_ref().clone()),
        &req,
        stream,
    )
}

/// Define websocket HTTP actor
struct WebSocket {
    last_check: Instant,
    client_replied: bool,
    invoice_subscriber: Subscriber,
    mailer: SmtpTransport,
}

impl WebSocket {
    fn new(invoice_subscriber: Subscriber, mailer: SmtpTransport) -> Self {
        Self {
            last_check: Instant::now(),
            client_replied: true,
            invoice_subscriber,
            mailer,
        }
    }

    /// Check subscriber for invoice update, and send result to user if applicable.
    fn try_update(&mut self, ctx: &mut <Self as Actor>::Context) {
        match self.invoice_subscriber.recv_timeout(HEARTBEAT_INTERVAL) {
            // Send an update of we got one.
            Ok(invoice_update) => {
                // Send the update to the user.
                ctx.text(ByteString::from(
                    json!(
                        {
                            "address": invoice_update.address(),
                            "amount_paid": invoice_update.amount_paid(),
                            "amount_requested": invoice_update.amount_requested(),
                            "confirmations": invoice_update.confirmations(),
                            "confirmations_required": invoice_update.confirmations_required(),
                            "expiration_in": invoice_update.expiration_in(),
                        }
                    )
                    .to_string(),
                ));
                // If the invoice is confirmed or expired, stop checking for updates.
                if invoice_update.is_confirmed() {
                    self.send_email(&invoice_update.description());
                    ctx.close(Some(ws::CloseReason::from((
                        ws::CloseCode::Normal,
                        "Invoice Complete",
                    ))));
                    ctx.stop();
                } else if invoice_update.is_expired() {
                    ctx.close(Some(ws::CloseReason::from((
                        ws::CloseCode::Normal,
                        "Invoice Expired",
                    ))));
                    ctx.stop();
                }
            }
            // Do nothing if there was no update.
            Err(AcceptXmrError::Subscriber(SubscriberError::RecvTimeout(
                std::sync::mpsc::RecvTimeoutError::Timeout,
            ))) => {}
            // Otherwise, handle the error.
            Err(e) => {
                error!("Failed to receive invoice update: {}", e);
                ctx.stop();
            }
        }
    }

    fn send_email(&self, description: &str) {
        let description_json: CheckoutInfo = serde_json::from_str(description)
            .expect("failed to parse description as Checkout Info");

        let admin_email = Message::builder()
            .from("AcceptXMR Demo <charlie@busyboredom.com>".parse().unwrap())
            .to("Charlie Wilkin <charlie@busyboredom.com>".parse().unwrap())
            .subject("AcceptXMR Demo: ".to_owned() + &description_json.message)
            .body(format!(
                "Email: {}\nMessage: {}",
                &description_json.email, &description_json.message
            ))
            .expect("failed to build email");

        // Send the email to me.
        match self.mailer.send(&admin_email) {
            Ok(_) => info!("AcceptXMR Demo admin email sent successfully!"),
            Err(e) => {
                error!("Could not send AcceptXMR Demo admin email: {:?}", e);
            }
        }

        if description_json.email.parse::<Mailbox>().is_err() {
            error!(
                "Failed to parse email address of AcceptXMR demo user: {}",
                description_json.email
            );
            return;
        }
        let user_email = Message::builder()
            .from("AcceptXMR Demo <charlie@busyboredom.com>".parse().unwrap())
            .to(description_json.email.parse().unwrap())
            .subject("AcceptXMR Demo: ".to_owned() + &description_json.message)
            .body(
                format!(
                    "Thank you for trying the AcceptXMR demo! This is the message you sent: \"{}\"", 
                    description_json.message
                ) + "\n\nIf your message was a question, you can expect to hear back from me within a week or so."
            )
            .expect("failed to build email");

        // Send the email to user.
        match self.mailer.send(&user_email) {
            Ok(_) => info!("AcceptXMR Demo user email sent successfully!"),
            Err(e) => {
                error!("Could not send AcceptXMR Demo user email: {:?}", e);
            }
        }
    }
}

impl Actor for WebSocket {
    type Context = ws::WebsocketContext<Self>;
    /// This method is called on actor start. We start waiting for updates here, periodically
    /// stopping to sent a heartbeat ping.
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // Wait for and then send an update.
            if act.client_replied {
                act.try_update(ctx);
                ctx.ping(b"");
                act.client_replied = false;
                act.last_check = Instant::now();
            // Check heartbeat.
            } else if Instant::now().duration_since(act.last_check) > CLIENT_TIMEOUT {
                warn!("Websocket heartbeat failed. Closing websocket.");
                ctx.stop();
            }
        });
    }
}

/// Handle incoming websocket messages.
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Pong(_)) => {
                self.client_replied = true;
            }
            Ok(ws::Message::Close(reason)) => {
                match &reason {
                    Some(r) => debug!("Websocket client closing: {:#?}", r.description),
                    None => debug!("Websocket client closing"),
                }
                ctx.close(reason);
                ctx.stop();
            }
            Ok(m) => debug!("Received unexpected message from websocket client: {:?}", m),
            Err(e) => warn!("Received error from websocket client: {:?}", e),
        }
    }
}
