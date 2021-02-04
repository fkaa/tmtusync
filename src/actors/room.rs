use actix::{Actor, Addr, AsyncContext, Context, Handler, Message, StreamHandler};

use crate::actors::{PlayingState, UserId, WebsocketTransport, StreamInfo, ToSessionMessage, ParticipantInfo, FromSessionMessage, ClientMessage, PlayState};

use log::*;

use std::time::{Duration, Instant};

#[derive(Default)]
pub struct Room {
    name: String,
    participants: Vec<Participant>,
    free_user_id: u32,
    current_stream: Option<StreamInfo>
}

impl Room {
    pub fn new(name: String, stream: Option<StreamInfo>) -> Self {
        Self {
            name,
            participants: Vec::new(),
            free_user_id: 0,
            current_stream: stream,
        }
    }

    fn announce_participant_new(&mut self, participant: &Participant) {
        let message = ToSessionMessage::NewParticipant {
            user_id: participant.user_id,
            name: participant.name.clone(),
        };

        for participant in &self.participants {
            participant.send_message(message.clone());
        }
    }

    fn announce_participant_left(&mut self, user_id: UserId) {
        let message = ToSessionMessage::ByeParticipant {
            user_id,
        };

        for participant in &self.participants {
            participant.send_message(message.clone());
        }
    }

    fn announce_seek(&mut self, src: UserId, duration: f32) {
        let message = ToSessionMessage::DoSeek {
            duration,
        };

        for participant in &self.participants {
            if participant.user_id != src {
                participant.send_message(message.clone());
            }
        }
    }

    fn announce_state(&mut self, src: UserId, state: PlayState) {
        let message = ToSessionMessage::SetState {
            state,
        };

        for participant in &self.participants {
            if participant.user_id != src {
                participant.send_message(message.clone());
            }
        }
    }

    fn add_participant(
        &mut self,
        name: String,
        user_id: UserId,
        transport: Addr<WebsocketTransport>,
    ) {
        debug!(
            "Adding new participant {:?} ({:?}) to room {:?}",
            name, user_id, self.name
        );

        let mut participant = Participant::new(name, user_id, transport);
        self.announce_participant_new(&participant);

        participant.send_message(self.get_room_state());
        self.participants.push(participant);
    }

    fn remove_participant(
        &mut self,
        user_id: UserId,
    ) {
        debug!(
            "Removing participant ({:?}) from room {:?}",
            user_id, self.name
        );


        if let Some(idx) = self.participants.iter().position(|p| p.user_id == user_id) {
            self.participants.remove(idx);
        }

        self.announce_participant_left(user_id);
    }

    fn get_room_state(&self) -> ToSessionMessage {
        ToSessionMessage::RoomState {
            participants: self.participants.iter().map(|p| ParticipantInfo {
                user_id: p.user_id,
                name: p.name.clone(),
            }).collect::<Vec<_>>(),
            current_stream: self.current_stream.clone(),
        }
    }
}

impl Actor for Room {
    type Context = Context<Self>;
}

pub struct GetRoomName;

impl Message for GetRoomName {
    type Result = String;
}

impl Handler<GetRoomName> for Room {
    type Result = String;

    fn handle(&mut self, msg: GetRoomName, _ctx: &mut Self::Context) -> Self::Result {
        self.name.clone()
    }
}

/*pub struct HelloUser(pub String, pub UserId, pub Addr<WebsocketTransport>);

impl Message for HelloUser {
    type Result = ();
}

impl Handler<HelloUser> for Room {
    type Result = ();

    fn handle(&mut self, msg: HelloUser, _ctx: &mut Self::Context) -> Self::Result {
        self.add_participant(msg.0, msg.1, msg.2);
    }
}*/

impl Handler<ClientMessage> for Room {
    type Result = anyhow::Result<()>;

    fn handle(&mut self, msg: ClientMessage, _ctx: &mut Self::Context) -> Self::Result {
        match msg.message {
            FromSessionMessage::Hello { name } => {
                self.add_participant(name, msg.from, msg.addr);
            }
            FromSessionMessage::Goodbye => {
                self.remove_participant(msg.from);
            }
            FromSessionMessage::Seek { duration } => {
                self.announce_seek(msg.from, duration);
            }
            FromSessionMessage::SetState { state } => {
                self.announce_state(msg.from, state);
            }
            _ => {}
        }

        Ok(())
    }
}

pub struct GetUserId;

impl Message for GetUserId {
    type Result = Option<UserId>;
}

impl Handler<GetUserId> for Room {
    type Result = Option<UserId>;

    fn handle(&mut self, _msg: GetUserId, _ctx: &mut Self::Context) -> Self::Result {
        let uid = self.free_user_id;
        self.free_user_id += 1;

        Some(UserId(uid))
    }
}

pub struct TimingInfo {
    current_time: Duration,
    sent_from_client: Instant,
    received_on_server: Instant,
}

// TODO: permissions
pub struct Participant {
    user_id: UserId,
    name: String,
    state: PlayingState,
    time: Option<TimingInfo>,
    transport: Addr<WebsocketTransport>,
}

impl Participant {
    pub fn new(name: String, user_id: UserId, transport: Addr<WebsocketTransport>) -> Self {
        Self {
            user_id,
            name,
            transport,
            state: PlayingState::Playing,
            time: None,
        }
    }

    fn send_message(&self, message: ToSessionMessage) {
        debug!("{:?} -> {:?}", self.user_id, message);

        self.transport.do_send(message);
    }
}
