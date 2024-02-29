use actix::{Addr, Actor};
use actix_web::{get, post, web, http::header, App, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_identity::{Identity, CookieIdentityPolicy, IdentityService};
use actix_web_actors::ws;
use actix_files::NamedFile;

use askama_actix::{TemplateIntoResponse};

use serde::{Serialize, Deserialize};
use log::*;

mod protocol;

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

use crate::protocol::{
    Stream,
    BadgeData,
    BadgeId,
    badges,
    badges::BADGE_DATA,
};

use crate::actors::{
    Room,
    MediaStream,
    GetUserId,
    StreamMetadata,
    RoomMetadata,
    PlayingState,
    RoomRepository,
    GetRoomMeta,
    RegisterRoom,
    FindRoom,
    WebsocketTransport,
};

use std::{
    path::PathBuf,
    time::Duration
};

#[derive(Deserialize, Debug)]
pub struct LoginData {
    pub nickname: String,
    pub avatar: u32,
    pub room: String,
}

#[derive(askama::Template)]
#[template(path = "hello.html")]
struct HelloTemplate<'a> {
    name: &'a str,
}

#[derive(askama::Template)]
#[template(path = "room.html")]
struct RoomTemplate<'a> {
    meta: RoomMetadata,
    nickname: &'a str,
    avatar: BadgeId,
    badges: &'a [BadgeId],
    code: &'a str,
    badge_data: &'a [BadgeData],
}

#[derive(askama::Template)]
#[template(path = "create_room.html")]
struct CreateRoomTemplate {
    files: Vec<String>,
}

async fn find_room(room_repository: &Addr<RoomRepository>, code: String) -> Option<Addr<Room>> {
    room_repository.send(FindRoom(code)).await.unwrap()
}

#[get("/websocket/{name}")]
async fn room_websocket_session(
    req: HttpRequest,
    identity: Identity,
    stream: web::Payload,
    path: web::Path<(String,)>,
    data: web::Data<AppData>,
) -> impl Responder {
    let room = find_room(&data.room_repo, path.into_inner().0).await;
    let cookie = identity.identity();

    if let (Some(room), Some(cookie)) = (room, cookie) {
        if let Some(id) = room.send(GetUserId(cookie.clone())).await.unwrap() {
            let transport = WebsocketTransport::new(cookie, id, room).await;

            ws::start(transport, &req, stream).unwrap()
        } else {
            error!("already logged in");

            HttpResponse::NotFound().body("already logged in")
        }
    } else {
        HttpResponse::NotFound().body("nope")
    }
}

#[get("/create")]
async fn create_room_page(
    req: HttpRequest,
    data: web::Data<AppData>,
) -> Result<HttpResponse, actix_web::Error> {
    let files = vec![
        String::from("Test1"),
        String::from("Test2"),
        String::from("Test3"),
    ];

    CreateRoomTemplate { files }.into_response()
}

/*#[get("/room/{name}")]
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
}*/

#[post("/")]
async fn index_auth(
    req: HttpRequest,
    params: web::Form<LoginData>,
    identity: Identity,
    data: web::Data<AppData>,
) -> Result<HttpResponse, actix_web::Error> {
    let room = find_room(&data.room_repo, params.room.to_string()).await;


    let new_cookie = format!("{}-{}", params.nickname, params.room);

    if let Some(cookie) = identity.identity() {
        if cookie != new_cookie {
            info!("invalidating cookie with new: {}", new_cookie);
            identity.remember(new_cookie);
        } else {
            info!("reusing cookie: {}", cookie);
        }
    } else {
        info!("remembering new cookie: {}", new_cookie);
        identity.remember(new_cookie);
    }

    if let Some(room) = room {
        let meta = room.send(GetRoomMeta).await.unwrap().unwrap();

        if params.nickname == "tmtu" {
            if let Some(addr) = req.connection_info().realip_remote_addr() {
                dbg!(addr);
                if !addr.starts_with("192.168.1.1") {
                    return
                        Ok(HttpResponse::NotFound().body("fuck off"))
                }
            }
        }

        let badges = if params.nickname == "tmtu" {
            &[badges::RUBY][..]
        } else {
            &[][..]
        };
        RoomTemplate {
            meta,
            nickname: &params.nickname,
            avatar: if params.nickname == "tmtu" {
                badges::USER_GRAY
            } else {
                BadgeId(params.avatar)
            },
            code: &params.room,
            badges: &badges,
            badge_data: &BADGE_DATA[..]
        }.into_response()
    } else {
        Ok(HttpResponse::Found()
            .header(header::LOCATION, "/#retry")
            .finish())
    }
}

#[get("/")]
async fn index(
    _req: HttpRequest,
    identity: Identity,
) -> Result<NamedFile, actix_web::Error> {
    let path: PathBuf = "./static/index.html".parse().unwrap();
    Ok(NamedFile::open(path)?)
}

#[derive(Clone)]
struct AppData {
    room_repo: Addr<RoomRepository>,
}

async fn register_room(room_repo: &Addr<RoomRepository>, stream: MediaStream, code: String) {
    let room = Room::new(code.clone(), Some(stream)).start();

    room_repo.send(RegisterRoom(code, room)).await.unwrap();
}

#[actix_rt::main]
async fn start() -> std::io::Result<()> {
    let room_repo = RoomRepository::default().start();

    let stream = MediaStream {
        slug: String::from("test2"),
        // slug: String::from("5731d81b-c8bf-4409-80ae-2b2c914aa30a"),
        name: String::from("Mechazawa"),
        streams: vec![
            Stream { quality: 0, playlist: String::from("master.m3u8") },
        ],
        meta: StreamMetadata {
            title: String::from("Infernal Affairs II"),
            duration: String::from("1h 15m 2s"),
            imdb: Some(String::from("https://www.imdb.com/title/tt0369060/")),
        },
    };

    register_room(&room_repo, stream, String::from("GZ4KQ")).await;

    let data = AppData { room_repo };

    HttpServer::new(move || {
        App::new()
            .wrap(
                IdentityService::new(
                    CookieIdentityPolicy::new(&[0; 32])
                        .name("auth-cookie")
                        .secure(false))
            )
            .data(data.clone())
            .service(room_websocket_session)
            //.service(room_page)
            .service(create_room_page)
            .service(index)
            .service(index_auth)
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
