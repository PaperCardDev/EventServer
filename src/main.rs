use std::time::Instant;

use actix::*;
use actix_web::{
    middleware::Logger, web, App, Error, HttpRequest, HttpResponse, HttpServer,
};
use actix_web_actors::ws;

mod server;
mod session;

/// websocket入口
async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<server::EventServer>>,
) -> Result<HttpResponse, Error> {
    // TODO 身份验证

    let client_id = "todo_no_client_id";

    ws::start(
        session::WsSession {
            id: 0,
            client_id: client_id.to_owned(),
            hb: Instant::now(),
            addr: srv.get_ref().clone(),
        },
        &req,
        stream,
    )
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // start chat server actor
    let server = server::EventServer::new().start();

    log::info!("starting HTTP server at http://localhost:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(server.clone()))
            .route("/ws", web::get().to(ws_route))
            .wrap(Logger::default())
    })
    // .workers(2)
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
