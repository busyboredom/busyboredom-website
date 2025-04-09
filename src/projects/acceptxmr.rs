use std::{
    future::Future,
    pin::Pin,
    task::Poll,
    time::{Duration, Instant},
};

use acceptxmr::{
    storage::stores::Sqlite, Invoice, InvoiceId, PaymentGateway, PaymentGatewayBuilder, Subscriber,
};
use actix::{prelude::Stream, Actor, ActorContext, AsyncContext, StreamHandler};
use actix_session::Session;
use actix_web::{
    get,
    http::header::{CacheControl, CacheDirective},
    post, web, HttpRequest, HttpResponse,
};
use actix_web_actors::ws;
use bytestring::ByteString;
use lettre::{message::Mailbox, Message, SmtpTransport, Transport};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::{Secrets, Settings};

/// Time before lack of client response causes a timeout.
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);
/// Time between sending heartbeat pings.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(4);

pub(crate) async fn setup(
    mailer: Arc<SmtpTransport>,
    secrets: Secrets,
    settings: Settings,
) -> PaymentGateway<Sqlite> {
    // Read view key from file.
    let private_view_key = secrets.xmr_private_viewkey;
    let daemon_password = secrets.daemon_password;

    // No need to keep the public spend key secret.
    let primary_address = "49KLp1DYdn8H344GXKDtKs9Aq8GGQBWnACxut4eHtMeYG1GNRhEmbzFCySA8WicJdQ6jVEqCKeSo4hpV6vFd9iXyH9hm4qq";

    let invoice_storage = Sqlite::new(
        &(settings.data_dir + "/AcceptXMR_DB/"),
        "invoices",
        "output keys",
        "height",
    )
    .expect("failed to open invoice storage");
    let payment_gateway = PaymentGatewayBuilder::new(
        private_view_key,
        primary_address.to_string(),
        invoice_storage,
    )
    //.daemon_url("https://node.busyboredom.com:18089".to_string())
    .daemon_url("https://node.sethforprivacy.com:443".to_string())
    //.daemon_login("busyboredom".to_string(), daemon_password)
    .build()
    .await
    .expect("failed to build payment gateway");
    info!("Payment gateway created.");

    loop {
        match payment_gateway.run().await {
            Ok(_) => break,
            Err(e) => {
                error!("Failed to run payment gateway: {e}");
                std::thread::sleep(Duration::from_secs(5));
            }
        }
    }
    info!("Payment gateway running.");

    // Watch for invoice updates and deal with them accordingly.
    let gateway_copy = payment_gateway.clone();
    tokio::spawn(async move {
        // Watch all invoice updates.
        let mut subscriber = gateway_copy.subscribe_all();
        loop {
            let invoice = match subscriber.blocking_recv() {
                Some(p) => p,
                // Global subscriber should never close.
                None => panic!("Blockchain scanner crashed!"),
            };

            // If it's confirmed, send the confirmation email.
            if invoice.is_confirmed() {
                send_email(&mailer, &invoice);
            }

            // If it's confirmed or expired, we probably shouldn't bother tracking it anymore.
            if (invoice.is_confirmed() && invoice.creation_height() < invoice.current_height())
                || invoice.is_expired()
            {
                debug!(
                    "Invoice to index {} is either confirmed or expired. Removing invoice now",
                    invoice.index()
                );
                if let Err(e) = gateway_copy.remove_invoice(invoice.id()).await {
                    error!("Failed to remove fully confirmed invoice: {e}");
                };
            }
        }
    });
    payment_gateway.clone()
}

fn send_email(mailer: &SmtpTransport, invoice: &Invoice) {
    let description_json: CheckoutInfo = serde_json::from_str(invoice.description())
        .expect("failed to parse description as Checkout Info");

    let admin_email = Message::builder()
        .from(
            "AcceptXMR Demo <donotreply@busyboredom.com>"
                .parse()
                .unwrap(),
        )
        .to("Charlie Wilkin <charlie@busyboredom.com>".parse().unwrap())
        .subject("AcceptXMR Demo: ".to_owned() + &description_json.message)
        .body(format!(
            "Email: {}\nMessage: {}",
            &description_json.email, &description_json.message
        ))
        .expect("failed to build email");

    // Send the email to me.
    match mailer.send(&admin_email) {
        Ok(_) => info!("AcceptXMR Demo admin email sent successfully!"),
        Err(e) => {
            error!("Could not send AcceptXMR Demo admin email: {e:?}");
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
        .from("AcceptXMR Demo <donotreply@busyboredom.com>".parse().unwrap())
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
    match mailer.send(&user_email) {
        Ok(_) => info!("AcceptXMR Demo user email sent successfully!"),
        Err(e) => {
            error!("Could not send AcceptXMR Demo user email: {e:?}");
        }
    }
}

#[derive(Deserialize, Serialize, Default)]
struct CheckoutInfo {
    email: String,
    message: String,
}

/// Create new invoice and place cookie.
#[post("/projects/acceptxmr/checkout")]
async fn checkout(
    session: Session,
    checkout_info: Option<web::Json<CheckoutInfo>>,
    payment_gateway: web::Data<PaymentGateway<Sqlite>>,
) -> Result<HttpResponse, actix_web::Error> {
    let checkout_info = match checkout_info {
        Some(json_info) => {
            let info = json_info.into_inner();
            session.insert("checkout_info", &info)?;
            info
        }
        None => {
            // If not provided, see if there's one in the session cookie.
            session.get("checkout_info")?.unwrap_or_default()
        }
    };
    let invoice_id = payment_gateway
        .new_invoice(1_000_000_000, 2, 5, json!(checkout_info).to_string())
        .await
        .unwrap();
    session.insert("id", invoice_id)?;
    Ok(HttpResponse::Ok()
        .append_header(CacheControl(vec![CacheDirective::NoStore]))
        .finish())
}

// Get invoice update without waiting for websocket.
#[get("/update")]
async fn update(
    session: Session,
    payment_gateway: web::Data<PaymentGateway<Sqlite>>,
) -> Result<HttpResponse, actix_web::Error> {
    if let Ok(Some(invoice_id)) = session.get::<InvoiceId>("id") {
        if let Ok(Some(invoice)) = payment_gateway.get_invoice(invoice_id).await {
            return Ok(HttpResponse::Ok()
                .append_header(CacheControl(vec![CacheDirective::NoStore]))
                .json(json!(
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
    Ok(HttpResponse::Gone()
        .append_header(CacheControl(vec![CacheDirective::NoStore]))
        .finish())
}

/// WebSocket rout.
#[get("/projects/acceptxmr/ws/")]
async fn websocket(
    session: Session,
    req: HttpRequest,
    stream: web::Payload,
    payment_gateway: web::Data<PaymentGateway<Sqlite>>,
) -> Result<HttpResponse, actix_web::Error> {
    let invoice_id = match session.get::<InvoiceId>("id") {
        Ok(Some(i)) => i,
        _ => {
            return Ok(HttpResponse::NotFound()
                .append_header(CacheControl(vec![CacheDirective::NoStore]))
                .finish())
        }
    };
    let subscriber = match payment_gateway.subscribe(invoice_id) {
        Some(s) => s,
        _ => {
            return Ok(HttpResponse::NotFound()
                .append_header(CacheControl(vec![CacheDirective::NoStore]))
                .finish())
        }
    };
    ws::start(WebSocket::new(subscriber), &req, stream)
}

/// Define websocket HTTP actor
struct WebSocket {
    last_heartbeat: Instant,
    invoice_subscriber: Option<Subscriber>,
}

impl WebSocket {
    fn new(invoice_subscriber: Subscriber) -> Self {
        Self {
            last_heartbeat: Instant::now(),
            invoice_subscriber: Some(invoice_subscriber),
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
}

impl Actor for WebSocket {
    type Context = ws::WebsocketContext<Self>;
    /// This method is called on actor start. We add the invoice subscriber as a stream here, and
    /// start heartbeat checks as well.
    fn started(&mut self, ctx: &mut Self::Context) {
        if let Some(subscriber) = self.invoice_subscriber.take() {
            <WebSocket as StreamHandler<Invoice>>::add_stream(InvoiceStream(subscriber), ctx);
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
            Ok(ws::Message::Ping(m)) => {
                self.last_heartbeat = Instant::now();
                ctx.pong(&m)
            }
            Ok(ws::Message::Close(reason)) => {
                match &reason {
                    Some(r) => debug!("Websocket client closing: {:#?}", r.description),
                    None => debug!("Websocket client closing"),
                }
                ctx.close(reason);
                ctx.stop();
            }
            Ok(m) => debug!("Received unexpected message from websocket client: {m:?}"),
            Err(e) => warn!("Received error from websocket client: {e:?}"),
        }
    }
}

/// Handle incoming invoice updates.
impl StreamHandler<Invoice> for WebSocket {
    fn handle(&mut self, msg: Invoice, ctx: &mut Self::Context) {
        // Send the update to the user.
        ctx.text(ByteString::from(
            json!(
                {
                    "address": msg.address(),
                    "amount_paid": msg.amount_paid(),
                    "amount_requested": msg.amount_requested(),
                    "uri": msg.uri(),
                    "confirmations": msg.confirmations(),
                    "confirmations_required": msg.confirmations_required(),
                    "expiration_in": msg.expiration_in(),
                }
            )
            .to_string(),
        ));
        // If the invoice is confirmed or expired, stop checking for updates.
        if msg.is_confirmed() {
            ctx.close(Some(ws::CloseReason::from((
                ws::CloseCode::Normal,
                "Invoice Complete",
            ))));
            ctx.stop();
        } else if msg.is_expired() {
            ctx.close(Some(ws::CloseReason::from((
                ws::CloseCode::Normal,
                "Invoice Expired",
            ))));
            ctx.stop();
        }
    }
}

// Wrapping `Subscriber` and implementing `Stream` on the wrapper allows us to use it as an efficient
// asynchronous stream for the Actix websocket.
struct InvoiceStream(Subscriber);

impl Stream for InvoiceStream {
    type Item = Invoice;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.0).poll(cx)
    }
}
