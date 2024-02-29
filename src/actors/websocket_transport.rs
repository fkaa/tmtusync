use actix::{Handler, Message, Actor, Addr, AsyncContext, ActorContext, StreamHandler};
use actix_web_actors::ws;

use stop_token::StopSource;

use log::*;

use crate::actors::{GetUserId, Room};

use chrono::{DateTime, Utc, TimeZone};

use std::time::{Duration, Instant};

use crate::protocol::{ServerTime, ClientTime, UserId,UserMessage,ClientMessage,ToSessionMessage};

pub struct WebsocketTransport {
    room: Addr<Room>,
    user_id: UserId,
    cookie: String,
    stop_source: StopSource,
}

impl WebsocketTransport {
    pub async fn new(cookie: String, user_id: UserId, room: Addr<Room>) -> Self {
        let stop_source = StopSource::new();

        Self {
            room,
            user_id,
            cookie,
            stop_source,
        }
    }

    fn handle_message(
        &mut self,
        server_time: ServerTime,
        message: UserMessage,
        ctx: &mut ws::WebsocketContext<Self>,
    ) {
        self.room.do_send(ClientMessage {
            from: self.user_id,
            cookie: self.cookie.clone(),
            message: message,
            addr: ctx.address(),
            server_time,
        });
    }

    fn handle_websocket_text(&mut self, txt: String, ctx: &mut ws::WebsocketContext<Self>) {
        trace!("{:?} <- {:?}", self.user_id, txt);
        let now = Utc::now();

        match serde_json::from_str::<UserMessage>(&txt) {
            Ok(message) => {
                trace!("{:?} <- {:?}", self.user_id, message);

                self.handle_message(ServerTime(now), message, ctx);
            }
            Err(e) => {
                warn!("Error parsing message from participant {:?}: {:?}", self.user_id, e);

                let err_message =
                    serde_json::to_string(&ToSessionMessage::Error(format!("{:?}", e)))
                        .unwrap();

                ctx.text(err_message);
            }
        }
    }
}

impl Actor for WebsocketTransport {
    type Context = ws::WebsocketContext<Self>;

    fn stopped(&mut self, ctx: &mut Self::Context) {
        self.room.do_send(ClientMessage {
            from: self.user_id,
            cookie: self.cookie.clone(),
            message: UserMessage::Goodbye,
            addr: ctx.address(),
            server_time: ServerTime(Utc::now()),
        });
    }
}

impl Handler<ToSessionMessage> for WebsocketTransport {
    type Result = anyhow::Result<()>;

    fn handle(&mut self, msg: ToSessionMessage, ctx: &mut Self::Context) -> Self::Result {
        let json = serde_json::to_string(&msg)?;

        ctx.text(json);

        Ok(())
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebsocketTransport {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(txt)) => {
                self.handle_websocket_text(txt, ctx);
            },
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            },
            _ => {}
        }
    }
}
