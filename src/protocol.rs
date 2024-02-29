use actix::{Addr, Message};

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use std::time::{Duration, Instant};
use std::ops::Deref;
use std::fmt;

use crate::WebsocketTransport;

pub struct BadgeData {
    pub name: &'static str,
    pub tooltip: &'static str,
}

impl BadgeData {
    const fn new(name: &'static str, tooltip: &'static str) -> Self {
        BadgeData {
            name,
            tooltip,
        }
    }
}

pub mod badges {
    use super::{BadgeId, BadgeData};

    pub const USER_SUIT: BadgeId = BadgeId(0);
    pub const USER_GREEN: BadgeId = BadgeId(1);
    pub const USER_RED: BadgeId = BadgeId(2);
    pub const USER_ORANGE: BadgeId = BadgeId(3);
    pub const TICK: BadgeId = BadgeId(4);
    pub const CROSS: BadgeId = BadgeId(5);
    pub const HOURGLASS: BadgeId = BadgeId(6);
    pub const RUBY: BadgeId = BadgeId(7);
    pub const ROSETTE: BadgeId = BadgeId(8);
    pub const RAINBOW: BadgeId = BadgeId(9);
    pub const MEDAL_BRONZE: BadgeId = BadgeId(10);
    pub const MEDAL_SILVER: BadgeId = BadgeId(11);
    pub const MEDAL_GOLD: BadgeId = BadgeId(12);
    pub const CONTROL_PLAY: BadgeId = BadgeId(13);
    pub const CONTROL_PLAY_BLUE: BadgeId = BadgeId(14);
    pub const CONTROL_PAUSE: BadgeId = BadgeId(15);
    pub const CONTROL_PAUSE_BLUE: BadgeId = BadgeId(16);
    pub const USER_GRAY: BadgeId = BadgeId(17);
    pub const USER_FEMALE: BadgeId = BadgeId(18);

    pub const BADGE_DATA: [BadgeData; 19] = [
        BadgeData::new("user_suit", "Person in suit"),
        BadgeData::new("user_green", "Person in green"),
        BadgeData::new("user_red", "Person in red"),
        BadgeData::new("user_orange", "Person in orange"),
        BadgeData::new("tick", "User is ready"),
        BadgeData::new("cross", "User is not ready"),
        BadgeData::new("hourglass", "User is loading"),
        BadgeData::new("ruby", "This person is a gem"),
        BadgeData::new("rosette", "This person graduated from grade school"),
        BadgeData::new("rainbow", "This person loves colors"),
        BadgeData::new("medal_bronze_1", "This person came in 3rd place"),
        BadgeData::new("medal_silver_1", "This person came in 2nd place"),
        BadgeData::new("medal_gold_1", "This person came in 1st place"),
        BadgeData::new("control_play", "User is playing"),
        BadgeData::new("control_play_blue", "User is playing"),
        BadgeData::new("control_pause", "User is paused"),
        BadgeData::new("control_pause_blue", "User is paused"),
        BadgeData::new("user_gray", "Person"),
        BadgeData::new("user_female", "Person"),
    ];
}

/// Identifier for a badge, used to display some status for a user.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
pub struct BadgeId(pub u32);

impl fmt::Display for BadgeId {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Unique identifier for a user. Handed out by the server.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
pub struct UserId(pub u32);

/// Milliseconds since UNIX time epoch.
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Time(pub i64);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParticipantInfo {
    pub user_id: UserId,
    pub name: String,
    pub avatar: BadgeId,
    pub badges: Vec<BadgeId>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParticipantUpdate {
    pub user_id: UserId,
    pub duration: f32,
    pub buffered: f32,
    pub state: PlayState,
    pub badges: Vec<BadgeId>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NewParticipant {
    user_id: UserId,
    name: String,
    avatar: BadgeId,
    badges: Vec<BadgeId>,
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
    pub duration: f32,
    pub state: PlayState,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ToSessionMessage {
    /// Initial payload describing how the room looks like
    RoomState {
        user_id: UserId,
        participants: Vec<ParticipantInfo>,
        current_stream: Option<StreamInfo>,
    },

    RoomUpdate {
        participants: Vec<ParticipantUpdate>,
    },

    NewParticipant {
        user_id: UserId,
        name: String,
        avatar: BadgeId,
        badges: Vec<BadgeId>,
    },
    ByeParticipant {
        user_id: UserId,
    },

    NewStream(StreamInfo),

    SetState {
        user: UserId,
        state: PlayState
    },
    DoSeek {
        user: UserId,
        duration: f32,
    },

    Ping,

    ChatMessage(ChatMessage),

    Error(String),
}

/// The state the media player can be in.
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Copy, Clone)]
pub enum PlayState {
    Play,
    Pause,
}

/// A message sent by the user to the server.
#[derive(Serialize, Deserialize, Debug)]
pub enum UserMessage {
    /// First message sent by a user, indicating their name.
    Hello { name: String, avatar: BadgeId, time: Time },

    /// User has left the room.
    Goodbye,

    /// The state of how the user's player looks like. This is sent as a response to a server ping
    /// _and_ whenever the state of either `duration` or `state` changes unexpectedly (eg. video
    /// buffering)
    State {
        /// The current time of the user's media.
        duration: f32,
        /// The time when the time of the user's media was last updated.
        duration_time: Time,

        /// The current state of the user's media.
        state: PlayState,
        /// Time when the play state was set.
        state_time: Time,

        /// Amount of video buffered from the perspective of the current time.
        buffered: f32,

        /// Time when the state was sent by the user.
        time: Time,
    },

    /// A user request to seek in the current media.
    Seek {
        /// The time the user wants to seek to.
        duration: f32,
        /// The time when the message was sent by the user.
        time: Time,
    },

    /// A user request to set the playing state of the current media.
    SetState {
        /// The state the user wants to change to.
        state: PlayState,
        /// The time when the message was sent by the user.
        time: Time,
    },
}

pub struct ClientMessage {
    /// Who sent the message.
    pub from: UserId,

    /// User cookie.
    pub cookie: String,

    /// The actor address that we can send replies to.
    pub addr: Addr<WebsocketTransport>,

    /// The actual message.
    pub message: UserMessage,

    /// When the message was received on the server.
    pub server_time: ServerTime,
}

/// A wrapper type for timestamps _from_ the client, not yet translated into the server's time.
#[derive(Clone, Debug)]
pub struct ClientTime(pub DateTime<Utc>);

impl Deref for ClientTime {
    type Target = DateTime<Utc>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A wrapper type for timestamps on the server.
#[derive(Clone, Debug)]
pub struct ServerTime(pub DateTime<Utc>);

impl Deref for ServerTime {
    type Target = DateTime<Utc>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Message for ToSessionMessage {
    type Result = anyhow::Result<()>;
}

impl Message for ClientMessage {
    type Result = anyhow::Result<()>;
}
