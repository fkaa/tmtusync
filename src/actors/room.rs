use actix::{Actor, Addr, AsyncContext, Context, Handler, Message, StreamHandler};

use crate::protocol::{badges, UserId, BadgeId, StreamInfo, Stream, ToSessionMessage, ParticipantInfo, ClientMessage, PlayState, ParticipantUpdate, UserMessage,ClientTime,ServerTime, Time};
use crate::actors::{WebsocketTransport};
use stop_token::{StopSource, StopToken};

use log::*;

use chrono::{TimeZone, DateTime, Utc};

use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::ops::Deref;
use std::fmt;

fn get_majority_time(times: &[f32], window: f32) -> f32 {
    0f32
}

#[derive(Debug, Clone)]
pub struct StreamMetadata {
    pub title: String,
    pub duration: String,

    pub imdb: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RoomMetadata {
    pub name: String,
    pub stream: StreamMetadata,
}

#[derive(Debug, Clone)]
pub struct MediaStream {
    pub slug: String,
    pub name: String,
    pub streams: Vec<Stream>,
    pub meta: StreamMetadata,
}

impl MediaStream {
    fn to_stream_info(&self, duration: f32, state: PlayState) -> StreamInfo {
        StreamInfo {
            slug: self.slug.clone(),
            name: self.name.clone(),
            streams: self.streams.clone(),
            duration,
            state
        }
    }
}

pub struct Room {
    name: String,
    cookies: HashMap<String, UserId>,
    participants: Vec<Participant>,
    free_user_id: u32,
    current_stream: Option<MediaStream>,
    room_state: PlayState,
    state_set: ServerTime,
    position_set: ServerTime,
    duration: f32,
}

impl Room {
    pub fn new(name: String, stream: Option<MediaStream>) -> Self {
        let now = Utc::now();

        Self {
            name,
            cookies: HashMap::new(),
            participants: Vec::new(),
            free_user_id: 0,
            current_stream: stream,
            room_state: PlayState::Pause,
            state_set: ServerTime(now),
            position_set: ServerTime(now),
            duration: 0f32,
        }
    }

    fn set_stream_position(&mut self, duration: f32) {
        debug!("Setting stream position to {}", duration);
        self.duration = duration;
    }

    fn get_stream_position(&self) -> f32 {
        let now = Utc::now();

        if self.room_state == PlayState::Pause {
            let since = *self.state_set - *self.position_set;
            self.duration + to_seconds(since)
        } else {
            let since = *ServerTime(now) - *self.state_set;
            self.duration + to_seconds(since)
        }
    }

    fn announce_participant_new(&mut self, message: ToSessionMessage) {
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

    fn announce_seek(&mut self, src: UserId, time: Time, duration: f32) {
        debug!("{:?} seeking to {}", src, duration);

        self.set_stream_position(duration);

        if let Some(mapping) = self.get_time_mapping(src) {
            self.position_set = mapping.convert(ClientTime(convert_time(time)));
        } else {
            warn!("Couldn't find time mapping for state setter {:?}", src);
        }

        let message = ToSessionMessage::DoSeek {
            user: src,
            duration,
        };

        for participant in &self.participants {
            if participant.user_id != src {
                participant.send_message(message.clone());
            }
        }
    }

    fn announce_state(&mut self, src: UserId, time: Time, state: PlayState) {
        debug!("{:?} set state to {:?}", src, state);

        self.room_state = state;
        let prev = self.state_set.clone();

        if let Some(mapping) = self.get_time_mapping(src) {
            self.state_set = mapping.convert(ClientTime(convert_time(time)));

            if state == PlayState::Pause {
                let since = *self.state_set - *prev;
                self.position_set = ServerTime(Utc::now());
                self.set_stream_position(self.duration + to_seconds(since));
            }
        } else {
            warn!("Couldn't find time mapping for state setter {:?}", src);
        }

        let message = ToSessionMessage::SetState {
            user: src,
            state,
        };

        for participant in &self.participants {
            if participant.user_id != src {
                participant.send_message(message.clone());
            }
        }
    }

    fn update_participant_time(&mut self, src: UserId, time: Time) {
        todo!()
    }

    fn announce_participant_updates(&mut self, updates: Vec<ParticipantUpdate>) {
        let message = ToSessionMessage::RoomUpdate {
            participants: updates,
        };

        for participant in &self.participants {
            participant.send_message(message.clone());
        }
    }

    fn get_time_mapping(&self, user_id: UserId) -> Option<&TimeMapping> {
        self.participants.iter().find(|p| p.user_id == user_id).and_then(|p| p.mapping.as_ref())
    }

    fn update_participant_state(
        &mut self,
        server_time: ServerTime,
        user_id: UserId,
        duration: f32,
        duration_time: Time,
        state: PlayState,
        state_time: Time,
        buffered: f32,
        time: Time,
    ) {
        if let Some(participant) = self.participants.iter_mut().find(|p| p.user_id == user_id) {
            participant.receive_state(
                server_time,
                duration,
                duration_time,
                state,
                state_time,
                buffered,
                time
            );
        } else {
            warn!("Tried to update non-existant participant!");
        }

        self.announce_participant_updates(self.get_room_updates());
    }

    fn add_participant(
        &mut self,
        name: String,
        avatar: BadgeId,
        cookie: String,
        user_id: UserId,
        room: Addr<Room>,
        transport: Addr<WebsocketTransport>,
        time: Time,
    ) {
        debug!(
            "Adding new participant {:?} ({:?}) to room {:?}",
            name, user_id, self.name
        );

        let mut badges = Vec::new();
        match user_id.0 {
            0 => badges.push(badges::MEDAL_GOLD),
            1 => badges.push(badges::MEDAL_SILVER),
            2 => badges.push(badges::MEDAL_BRONZE),
            _ => {}
        }
        if name == "tmtu" {
            badges.push(badges::ROSETTE);
        }

        let mut participant = Participant::new(
            name,
            avatar,
            badges,
            cookie,
            user_id,
            room,
            transport,
            time);
        let msg = participant.get_announce_message();
        self.announce_participant_new(msg);

        participant.send_message(self.get_room_state_for_uid(user_id));
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

    fn send_participant_ping(&mut self, user_id: UserId, ping_id: u32) {
        if let Some(participant) = self.participants.iter_mut().find(|p| p.user_id == user_id) {
            participant.send_ping(
                ping_id,
            );
        } else {
            warn!("Tried to ping non-existant participant {:?}", user_id);
        }
    }

    fn get_room_updates(&self) -> Vec<ParticipantUpdate> {
        let mut updates = Vec::new();

        let now = Utc::now();

        for p in &self.participants {
            if let Some(duration) = p.get_playing_time(ServerTime(now)) {
                updates.push(ParticipantUpdate {
                    user_id: p.user_id,
                    duration,
                    buffered: p.buffered,
                    state: p.state,
                    badges: p.badges.clone(),
                });
            } else {
                warn!("Skipped sending updates for {:?} since it was missing time mapping", p.user_id);
            }
        }

        updates
    }

    fn get_room_state_for_uid(&mut self, user_id: UserId) -> ToSessionMessage {
        ToSessionMessage::RoomState {
            user_id,
            participants: self.participants.iter().map(|p| ParticipantInfo {
                user_id: p.user_id,
                name: p.name.clone(),
                avatar: p.avatar,
                badges: p.badges.clone(),
            }).collect::<Vec<_>>(),
            current_stream: self.current_stream.as_ref().map(|s| s.to_stream_info(self.get_stream_position(), self.room_state)),
        }
    }
}

impl Actor for Room {
    type Context = Context<Self>;
}

pub struct SendPing(UserId, u32);

impl Message for SendPing {
    type Result = ();
}

impl Handler<SendPing> for Room {
    type Result = ();

    fn handle(&mut self, msg: SendPing, _ctx: &mut Self::Context) -> Self::Result {
        self.send_participant_ping(msg.0, msg.1);
    }
}

#[derive(Message)]
#[rtype(result = "Option<RoomMetadata>")]
pub struct GetRoomMeta;

impl Handler<GetRoomMeta> for Room {
    type Result = Option<RoomMetadata>;

    fn handle(&mut self, msg: GetRoomMeta, _ctx: &mut Self::Context) -> Self::Result {
        self.current_stream.as_ref().map(|s| RoomMetadata { name: self.name.clone(), stream: s.meta.clone() })
    }
}

impl Handler<ClientMessage> for Room {
    type Result = anyhow::Result<()>;

    fn handle(&mut self, msg: ClientMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg.message {
            UserMessage::Hello { name, avatar, time } => {
                self.add_participant(name, avatar, msg.cookie, msg.from, ctx.address(), msg.addr, time);
            }
            /*UserMessage::Pong(time) => {
                self.update_participant_time(msg.from, time);
            }*/
            UserMessage::Goodbye => {
                self.remove_participant(msg.from);
            }
            UserMessage::Seek { duration, time } => {
                self.announce_seek(msg.from, time, duration);
            }
            UserMessage::SetState { state, time } => {
                self.announce_state(msg.from, time, state);
            }
            UserMessage::State {
                duration,
                duration_time,
                state,
                state_time,
                buffered,
                time
            } => {
                self.update_participant_state(
                    msg.server_time,
                    msg.from,
                    duration,
                    duration_time,
                    state,
                    state_time,
                    buffered,
                    time
                );
            }
            _ => {}
        }

        Ok(())
    }
}

pub struct GetUserId(pub String);

impl Message for GetUserId {
    type Result = Option<UserId>;
}

impl Handler<GetUserId> for Room {
    type Result = Option<UserId>;

    fn handle(&mut self, msg: GetUserId, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(uid) = self.cookies.get(&msg.0) {
            Some(*uid)
        } else {
            let uid = UserId(self.free_user_id);
            self.free_user_id += 1;

            self.cookies.insert(msg.0, uid);

            Some(uid)
        }
    }
}

#[derive(Clone)]
pub struct TimingInfo {
    current_time: Duration,
    sent_from_client: Instant,
    received_on_server: Instant,
}

#[derive(Clone)]
pub struct TimeMapping {
    requested_time: ServerTime,
    server_time: ServerTime,
    client_time: ClientTime,
}

impl TimeMapping {
    pub fn new(requested_time: ServerTime, server_time: ServerTime, client_time: ClientTime) -> Self {
        Self { requested_time, server_time, client_time }
    }

    fn convert(&self, time: ClientTime) -> ServerTime {
        let diff = *time - *self.client_time;

        let server_time = *self.server_time + diff;

        ServerTime(server_time)
    }

    fn time_since(&self, time: ServerTime) -> chrono::Duration {
        *time - *self.server_time
    }
}

impl fmt::Debug for TimeMapping {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "Server({}), diff: {}, rtt: {}", self.server_time.0, self.server_time.0 - self.client_time.0, self.server_time.0 - self.requested_time.0)?;
        Ok(())
    }
}

async fn participant_ping_loop(user_id: UserId, room: Addr<Room>) {
    use tokio::time::delay_for;

    let mut ping_num = 0;

    loop {
        room.send(SendPing(user_id, ping_num)).await.unwrap();
        ping_num += 1;

        delay_for(Duration::from_millis(5000)).await;
    }
}

// TODO: permissions
pub struct Participant {
    user_id: UserId,
    name: String,
    avatar: BadgeId,
    badges: Vec<BadgeId>,
    cookie: String,

    duration: f32,
    duration_time: ClientTime,
    state: PlayState,
    state_time: ClientTime,

    buffered: f32,

    time: Option<TimingInfo>,
    mapping: Option<TimeMapping>,

    transport: Addr<WebsocketTransport>,

    last_ping: Option<ServerTime>,

    stop_source: StopSource,
}

impl Participant {
    pub fn new(
        name: String,
        avatar: BadgeId,
        badges: Vec<BadgeId>,
        cookie: String,
        user_id: UserId,
        room: Addr<Room>,
        transport: Addr<WebsocketTransport>,
        created: Time
    ) -> Self {
        let created = ClientTime(convert_time(created));
        let stop_source = StopSource::new();
        let stop_token = stop_source.stop_token();

        tokio::spawn(stop_token.stop_future(participant_ping_loop(user_id, room)));

        Self {
            user_id,
            name,
            avatar,
            badges,
            cookie,

            duration: 0f32,
            duration_time: created.clone(),
            state: PlayState::Pause,
            state_time: created.clone(),
            buffered: 0f32,

            time: None,
            mapping: None,

            transport,

            last_ping: None,
            stop_source,
        }
    }

    fn get_announce_message(&self) -> ToSessionMessage {
        let message = ToSessionMessage::NewParticipant {
            user_id: self.user_id,
            name: self.name.clone(),
            avatar: self.avatar,
            badges: self.badges.clone(),
        };

        message
    }

    fn send_message(&self, message: ToSessionMessage) {
        trace!("{:?} -> {:?}", self.user_id, message);

        self.transport.do_send(message);
    }

    fn get_playing_time(&self, at_time: ServerTime) -> Option<f32> {
        let mapping = self.mapping.as_ref()?;

        if self.state == PlayState::Pause {
            Some(self.duration)
        } else {
            let time = mapping.convert(self.duration_time.clone());
            let time_since = mapping.time_since(time);

            Some(self.duration + to_seconds(time_since))
        }
    }

    fn send_ping(&mut self, ping_id: u32) {
        self.last_ping = Some(ServerTime(Utc::now()));

        self.transport.do_send(ToSessionMessage::Ping);
    }

    fn receive_state(
        &mut self,
        server_time: ServerTime,
        duration: f32,
        duration_time: Time,
        state: PlayState,
        state_time: Time,
        buffered: f32,
        time: Time,
    ) {
        self.duration = duration;
        self.duration_time = ClientTime(convert_time(duration_time));

        self.state = state;
        self.state_time = ClientTime(convert_time(state_time));

        self.buffered = buffered;

        if let Some(ping_time) = &self.last_ping {
            let client_time = ClientTime(convert_time(time));

            let mapping = TimeMapping::new(ping_time.clone(), server_time, client_time);

            debug!("Received participant mapping: {:?}", mapping);

            self.mapping = Some(mapping);
        } else {
            warn!("No corresponding ping time for received pong!");
        }
    }
}

fn to_seconds(duration: chrono::Duration) -> f32 {
    (duration.num_milliseconds() as f64 / 1000.0) as f32
}

fn convert_time(time: Time) -> DateTime<Utc> {
    let secs = time.0 / 1000;
    let nano_secs = (time.0 % 1_000) * 100_000_0;

    Utc.timestamp(secs, nano_secs as u32)
}
