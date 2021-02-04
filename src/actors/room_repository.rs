use actix::{Actor, Addr, AsyncContext, Context, Handler, Message, StreamHandler};

use std::collections::HashMap;

use crate::actors::Room;

// TODO: persist rooms in database
#[derive(Default)]
pub struct RoomRepository {
    rooms: HashMap<String, Addr<Room>>,
}

impl Actor for RoomRepository {
    type Context = Context<Self>;
}

pub struct FindRoom(pub String);

impl Message for FindRoom {
    type Result = Option<Addr<Room>>;
}

impl Handler<FindRoom> for RoomRepository {
    type Result = Option<Addr<Room>>;

    fn handle(&mut self, msg: FindRoom, _ctx: &mut Self::Context) -> Self::Result {
        self.rooms.get(&msg.0).map(|a| a.clone())
    }
}

pub struct RegisterRoom(pub String, pub Addr<Room>);

impl Message for RegisterRoom {
    type Result = ();
}

impl Handler<RegisterRoom> for RoomRepository {
    type Result = ();

    fn handle(&mut self, msg: RegisterRoom, _ctx: &mut Self::Context) -> Self::Result {
        self.rooms.insert(msg.0, msg.1);
    }
}

pub struct RemoveRoom(pub String);

impl Message for RemoveRoom {
    type Result = ();
}

impl Handler<RemoveRoom> for RoomRepository {
    type Result = ();

    fn handle(&mut self, msg: RemoveRoom, _ctx: &mut Self::Context) -> Self::Result {
        self.rooms.remove(&msg.0);
    }
}
