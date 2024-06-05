use std::time::Instant;

use actix::*;
use actix_web::{middleware::Logger, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use serde_json::json;

mod server;
mod session;

/// websocket入口
async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<server::EventServer>>,
) -> Result<HttpResponse, Error> {
    // 身份验证

    let h = req.headers();

    let client_id = h.get("PaperClientId");
    let sign = h.get("PaperSign");
    let timestamp = h.get("PaperTs");

    if client_id.is_none() || sign.is_none() || timestamp.is_none() {
        return Ok(HttpResponse::Unauthorized().finish());
    }

    let client_id = client_id.unwrap().to_str().unwrap();
    let sign = sign.unwrap().to_str().unwrap();
    let timestamp = timestamp.unwrap().to_str().unwrap();

    // 将时间戳转为数字
    let timestamp = timestamp.parse::<u64>();

    if timestamp.is_err() {
        return Ok(HttpResponse::Unauthorized().finish());
    }

    let timestamp = timestamp.unwrap();

    // https://paper-card.cn/api/test/sign
    let client = reqwest::Client::new();

    let req_json = json!({
        "client_id": client_id,
        "sign": sign,
        "ts": timestamp,
    });
    let req_json = req_json.to_string();

    let res = client
        .post("https://paper-card.cn/api/test/sign")
        .body(req_json)
        .send()
        .await;

    if res.is_err() {
        return Ok(HttpResponse::InternalServerError().finish());
    }

    let res = res.unwrap().text().await;

    if res.is_err() {
        return Ok(HttpResponse::InternalServerError().finish());
    }

    let res = res.unwrap();

    println!("签名测试结果：{}", res);

    // json解析
    let res: Result<serde_json::Value, serde_json::Error> = serde_json::from_str(&res);

    if res.is_err() {
        return Ok(HttpResponse::InternalServerError().finish());
    }

    let res = res.unwrap();

    let res = res.as_object();

    if res.is_none() {
        return Ok(HttpResponse::InternalServerError().finish());
    }

    let res = res.unwrap();

    let res = res.get("ec");

    if res.is_none() {
        return Ok(HttpResponse::InternalServerError().finish());
    }

    let res = res.unwrap().as_str();

    if res.is_none() {
        return Ok(HttpResponse::InternalServerError().finish());
    }

    let res = res.unwrap();

    if res != "ok" {
        return Ok(HttpResponse::Unauthorized().finish());
    }

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
