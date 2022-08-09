use std::{
    future::Future,
    pin::Pin,
    task::Poll,
    time::{Duration, Instant},
};

use acceptxmr::{
    AcceptXmrError, Invoice, InvoiceId, PaymentGateway, PaymentGatewayBuilder, Subscriber,
};
use actix::{prelude::Stream, Actor, ActorContext, AsyncContext, StreamHandler};
use actix_session::Session;
use actix_web::{get, post, web, HttpRequest, HttpResponse, http::header::{CacheControl, CacheDirective}};
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
    let daemon_password = include_str!("../../secrets/daemon_password.txt")
        .to_string()
        .trim() // Remove line ending.
        .to_owned();

    // No need to keep the public spend key secret.
    let primary_address = "4A1WSBQdCbUCqt3DaGfmqVFchXScF43M6c5r4B6JXT3dUwuALncU9XTEnRPmUMcB3c16kVP9Y7thFLCJ5BaMW3UmSy93w3w";

    let payment_gateway = PaymentGatewayBuilder::new(private_view_key, primary_address.to_string())
        .daemon_url("https://busyboredom.com:18089".to_string())
        .daemon_login("busyboredom".to_string(), daemon_password)
        .build()
        .expect("failed to build payment gateway");
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
#[post("/projects/acceptxmr/checkout")]
async fn checkout(
    session: Session,
    checkout_info: web::Json<CheckoutInfo>,
    payment_gateway: web::Data<PaymentGateway>,
) -> Result<HttpResponse, actix_web::Error> {
    let invoice_id = payment_gateway
        .new_invoice(1_000_000_000, 2, 5, json!(checkout_info).to_string())
        .await
        .unwrap();
    session.insert("id", invoice_id)?;
    Ok(HttpResponse::Ok().append_header(CacheControl(vec![CacheDirective::NoStore])).finish())
}

// Get invoice update without waiting for websocket.
#[get("/update")]
async fn update(
    session: Session,
    payment_gateway: web::Data<PaymentGateway>,
) -> Result<HttpResponse, actix_web::Error> {
    if let Ok(Some(invoice_id)) = session.get::<InvoiceId>("id") {
        if let Ok(Some(invoice)) = payment_gateway.get_invoice(invoice_id) {
            return Ok(HttpResponse::Ok().append_header(CacheControl(vec![CacheDirective::NoStore])).json(json!(
                {
                    "address": invoice.address(),
                    "amount_paid": invoice.amount_paid(),
                    "amount_requested": invoice.amount_requested(),
                    "uri": invoice.uri(),
                    "confirmations": invoice.confirmations(),
                    "confirmations_required": invoice.confirmations_required(),
                    "expiration_in": invoice.expiration_in(),
                }
            )));
        };
    }
    Ok(HttpResponse::Gone().append_header(CacheControl(vec![CacheDirective::NoStore])).finish())
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
        _ => return Ok(HttpResponse::NotFound().append_header(CacheControl(vec![CacheDirective::NoStore])).finish()),
    };
    let subscriber = match payment_gateway.subscribe(invoice_id) {
        Ok(Some(s)) => s,
        _ => return Ok(HttpResponse::NotFound().append_header(CacheControl(vec![CacheDirective::NoStore])).finish()),
    };
    ws::start(
        WebSocket::new(subscriber, mailer.get_ref().clone()),
        &req,
        stream,
    )
}

/// Define websocket HTTP actor
struct WebSocket {
    last_heartbeat: Instant,
    invoice_subscriber: Option<Subscriber>,
    mailer: SmtpTransport,
}

impl WebSocket {
    fn new(invoice_subscriber: Subscriber, mailer: SmtpTransport) -> Self {
        Self {
            last_heartbeat: Instant::now(),
            invoice_subscriber: Some(invoice_subscriber),
            mailer,
        }
    }

    /// Sends ping to client every `HEARTBEAT_INTERVAL` and checks for responses from client
    fn heartbeat(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.last_heartbeat) > CLIENT_TIMEOUT {
                // heartbeat timed out
                warn!("Websocket Client heartbeat failed, disconnecting!");
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
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
                    "Thank you for trying the AcceptXMR demo! This is the message you sent:\n\"{}\"", 
                    description_json.message
                ) + "\n\nIf your message was a question, you can expect to hear back from me within\na week or so."
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
    /// This method is called on actor start. We add the invoice subscriber as a stream here, and
    /// start heartbeat checks as well.
    fn started(&mut self, ctx: &mut Self::Context) {
        if let Some(subscriber) = self.invoice_subscriber.take() {
            <WebSocket as StreamHandler<Result<Invoice, AcceptXmrError>>>::add_stream(
                InvoiceStream(subscriber),
                ctx,
            );
        }
        self.heartbeat(ctx);
    }
}

/// Handle incoming websocket messages.
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Pong(_)) => {
                self.last_heartbeat = Instant::now();
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

/// Handle incoming invoice updates.
impl StreamHandler<Result<Invoice, AcceptXmrError>> for WebSocket {
    fn handle(&mut self, msg: Result<Invoice, AcceptXmrError>, ctx: &mut Self::Context) {
        match msg {
            Ok(invoice_update) => {
                // Send the update to the user.
                ctx.text(ByteString::from(
                    json!(
                        {
                            "address": invoice_update.address(),
                            "amount_paid": invoice_update.amount_paid(),
                            "amount_requested": invoice_update.amount_requested(),
                            "uri": invoice_update.uri(),
                            "confirmations": invoice_update.confirmations(),
                            "confirmations_required": invoice_update.confirmations_required(),
                            "expiration_in": invoice_update.expiration_in(),
                        }
                    )
                    .to_string(),
                ));
                // If the invoice is confirmed or expired, stop checking for updates.
                if invoice_update.is_confirmed() {
                    self.send_email(invoice_update.description());
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
            Err(e) => {
                error!("Failed to receive invoice update: {}", e);
                ctx.stop();
            }
        }
    }
}

// Wrapping `Subscriber` and implementing `Stream` on the wrapper allows us to use it as an efficient
// asynchronous stream for the Actix websocket.
struct InvoiceStream(Subscriber);

impl Stream for InvoiceStream {
    type Item = Result<Invoice, AcceptXmrError>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.0).poll(cx)
    }
}
