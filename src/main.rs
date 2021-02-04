use actix::{Addr, Actor};
use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web_actors::ws;

use askama_actix::{TemplateIntoResponse};

use serde::{Serialize, Deserialize};

mod actors {
    mod room;
    mod room_repository;
    mod participant;
    mod websocket_transport;

    pub use self::{
        room_repository::*,
        room::*,
        participant::*,
        websocket_transport::*,
    };
}

use crate::actors::{
    Room,
    PlayingState,
    RoomRepository,
    GetRoomName,
    RegisterRoom,
    FindRoom,
    WebsocketTransport,
    Stream,
    StreamInfo,
};

use std::time::Duration;


#[derive(askama::Template)]
#[template(path = "hello.html")]
struct HelloTemplate<'a> {
    name: &'a str,
}

#[derive(askama::Template)]
#[template(path = "room.html")]
struct RoomTemplate<'a> {
    name: &'a str,
}

async fn find_room(room_repository: &Addr<RoomRepository>, req: &HttpRequest) -> Option<Addr<Room>> {
    let path: std::path::PathBuf = req.match_info().query("name").parse().unwrap();

    let f = path.file_name().unwrap().to_str().unwrap().to_string();

    room_repository.send(FindRoom(f)).await.unwrap()
}

#[get("/room/ws/{name}")]
async fn room_websocket_session(
    req: HttpRequest,
    stream: web::Payload,
    data: web::Data<AppData>,
) -> impl Responder {
    let room = find_room(&data.room_repo, &req).await;

    if let Some(room) = room {
        let transport = WebsocketTransport::new(room).await;

        ws::start(transport, &req, stream).unwrap()
    } else {
        HttpResponse::NotFound().body("nope")
    }
}

#[get("/room/{name}")]
async fn room_page(
    req: HttpRequest,
    data: web::Data<AppData>,
) -> Result<HttpResponse, actix_web::Error> {
    let room = find_room(&data.room_repo, &req).await;

    if let Some(room) = room {
        let name = room.send(GetRoomName).await.unwrap();

        RoomTemplate { name: &name }.into_response()
    } else {
        Ok(HttpResponse::NotFound().body("nope"))
    }
}

#[derive(Clone)]
struct AppData {
    room_repo: Addr<RoomRepository>,
}

#[actix_rt::main]
async fn start() -> std::io::Result<()> {
    let room_repo = RoomRepository::default().start();

    let stream = StreamInfo {
        slug: String::from("fa870aa2-0bff-4042-abac-cbfb57f11080"),
        name: String::from("Mando"),
        streams: vec![
            Stream { quality: 0, playlist: String::from("stream_0/stream_0.m3u8") },
            Stream { quality: 1, playlist: String::from("stream_1/stream_1.m3u8") },
            Stream { quality: 2, playlist: String::from("stream_2/stream_2.m3u8") },
        ]
    };
    let room = Room::new(String::from("ElectricBananaBand"), Some(stream)).start();

    room_repo.send(RegisterRoom(String::from("ElectricBananaBand"), room)).await.unwrap();

    let data = AppData { room_repo };

    HttpServer::new(move || {
        App::new()
            .data(data.clone())
            .service(room_websocket_session)
            .service(room_page)
            .service(actix_files::Files::new("/static", "static/"))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    start()?;

    Ok(())
}
