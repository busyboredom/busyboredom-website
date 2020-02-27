#[macro_use]
extern crate actix_web;

use std::{env, io};

use actix_session::{CookieSession, Session};
use actix_web::http::StatusCode;
use actix_web::{
    guard, middleware, web, App, HttpRequest, HttpResponse, HttpServer,
    Result,
};

/// welcome page
#[get("/api/welcome")]
async fn welcome() -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/welcome.html")))
}

/// wasm binding handler
#[get("/api/bindings")]
async fn bindings() -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("application/javascript")
        .body(include_str!("../wasm/pkg/frontend.js")))
}

/// wasm handler
#[get("/api/wasm")]
async fn frontend_wasm() -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("application/wasm")
        .body(&include_bytes!("../wasm/pkg/frontend_bg.wasm")[..]))
}

/// 404 handler
#[get("/api/error-404")]
async fn p404() -> Result<HttpResponse> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/404.html")))
}

/// favicon handler
#[get("/favicon")]
async fn favicon() -> Result<&'static [u8]> {
    Ok(include_bytes!("../static/favicon.ico"))
}


/// simple index handler
async fn base(session: Session, _req: HttpRequest) -> Result<HttpResponse> {
    // Print content of request if compiled with debug profile. 
    #[cfg(debug_assertions)]
    println!("{:?}", _req);

    // session
    let mut counter = 1;
    if let Some(count) = session.get::<i32>("counter")? {
        println!("SESSION value: {}", count);
        counter = count + 1;
    }

    // set counter to session
    session.set("counter", counter)?;

    // response
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/base.html")))
}

/// handler with path parameters like `/user/{name}/`
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
            // comression middleware
            .wrap(middleware::Compress::default())
            // cookie session middleware
            .wrap(CookieSession::signed(&[0; 32]).secure(false))
            // enable logger - always register actix-web Logger middleware last
            .wrap(middleware::Logger::default())
            // register welcome page
            .service(welcome)
            // register bindings
            .service(bindings)
            // register wasm
            .service(frontend_wasm)
            // register favicon
            .service(favicon)
            // register 404
            .service(p404)
            // with path parameters
            .service(web::resource("/user/{name}").route(web::get().to(with_param)))
            // default
            .default_service(
                web::resource("")
                    .route(web::get().to(base))
                    // all requests that are not `GET`
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
