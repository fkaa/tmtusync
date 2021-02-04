use actix::{Handler, Message, Actor, Addr, AsyncContext, ActorContext, StreamHandler};
use actix_web_actors::ws;

use stop_token::StopSource;

use log::*;
use serde::{Deserialize, Serialize};

use crate::actors::{GetUserId, Room};

use std::time::Duration;

pub struct WebsocketTransport {
    room: Addr<Room>,
    user_id: UserId,
    stop_source: StopSource,
}

impl WebsocketTransport {
    pub async fn new(room: Addr<Room>) -> Self {
        let user_id = room.send(GetUserId).await.unwrap().unwrap();

        let stop_source = StopSource::new();

        Self {
            room,
            user_id,
            stop_source,
        }
    }

    fn handle_message(
        &mut self,
        message: FromSessionMessage,
        ctx: &mut ws::WebsocketContext<Self>,
    ) {
        self.room.do_send(ClientMessage {
            from: self.user_id,
            message: message,
            addr: ctx.address()
        });
    }
}

impl Actor for WebsocketTransport {
    type Context = ws::WebsocketContext<Self>;

    fn stopped(&mut self, ctx: &mut Self::Context) {
        self.room.do_send(ClientMessage {
            from: self.user_id,
            message: FromSessionMessage::Goodbye,
            addr: ctx.address()
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
                trace!("{:?} <- {:?}", self.user_id, txt);

                match serde_json::from_str::<FromSessionMessage>(&txt) {
                    Ok(message) => {
                        debug!("{:?} <- {:?}", self.user_id, message);

                        self.handle_message(message, ctx);
                    }
                    Err(e) => {
                        warn!("Error parsing message from participant {:?}: {:?}", self.user_id, e);

                        let err_message =
                            serde_json::to_string(&ToSessionMessage::Error(format!("{:?}", e)))
                                .unwrap();

                        ctx.text(err_message);
                    }
                }
            },
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            },
            _ => {}
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
pub struct UserId(pub u32);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParticipantInfo {
    pub user_id: UserId,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NewParticipant {
    user_id: UserId,
    name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatMessage {
    from: UserId,
    msg: String,
}

/// Info about a media stream, containing a sortable quality number and the file name of the HLS
/// playlist.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Stream {
    pub quality: u32,
    pub playlist: String,
}

/// Info about a media stream, containing the directory slug for the data and a list of all
/// available HLS playlists.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StreamInfo {
    pub slug: String,
    pub name: String,
    pub streams: Vec<Stream>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ToSessionMessage {
    /// Initial payload describing how the room looks like
    RoomState {
        participants: Vec<ParticipantInfo>,
        current_stream: Option<StreamInfo>,
    },

    NewParticipant {
        user_id: UserId,
        name: String,
    },
    ByeParticipant {
        user_id: UserId,
    },

    NewStream(StreamInfo),

    SetState {
        state: PlayState
    },
    DoSeek {
        duration: f32,
    },

    ChatMessage(ChatMessage),

    Error(String),
}

impl Message for ToSessionMessage {
    type Result = anyhow::Result<()>;
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum PlayState {
    Play,
    Pause,
}

// TODO: error
#[derive(Serialize, Deserialize, Debug)]
pub struct SessionState {
    position: Duration,
    buffered: Duration,
    player_state: PlayState,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SessionHello {
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum FromSessionMessage {
    Hello { name: String },
    Goodbye,
    State(SessionState),
    Buffering(Duration),
    Seek {
        duration: f32,
    },
    SetState {
        state: PlayState
    },
    Message(String),
}

pub struct ClientMessage {
    pub from: UserId,
    pub addr: Addr<WebsocketTransport>,
    pub message: FromSessionMessage,
}

impl Message for ClientMessage {
    type Result = anyhow::Result<()>;
}
