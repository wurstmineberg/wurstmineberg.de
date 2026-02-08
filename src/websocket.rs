use {
    std::{
        borrow::Cow,
        hash::{
            Hash,
            Hasher,
        },
    },
    async_proto::Protocol,
    bitvec::prelude::*,
    mcanvil::{
        BlockState,
        Dimension,
    },
    serde::{
        Deserialize,
        Serialize,
    },
    serenity::model::prelude::*,
    uuid::Uuid,
};

#[derive(Protocol)]
pub enum ServerMessageV3 {
    /// Will be sent by the server once every 30 seconds.
    /// The client should reply with [`ClientMessage::Pong`].
    /// A client that does not receive any messages for 60 seconds may want to consider the connection to have failed.
    Ping,
    Error {
        debug: String,
        display: String,
    },
    ChunkData {
        dimension: Dimension,
        cx: i32,
        cy: i8,
        cz: i32,
        data: Option<[Box<[[BlockState; 16]; 16]>; 16]>,
    },
    PlayerData {
        id: UserIdResponse,
        uuid: Uuid,
        data: Option<nbt::Blob>,
    },
}

#[derive(Protocol)]
pub enum ServerMessageV4 {
    /// Will be sent by the server once every 30 seconds.
    /// The client should reply with [`ClientMessage::Pong`].
    /// A client that does not receive any messages for 60 seconds may want to consider the connection to have failed.
    Ping,
    Error {
        debug: String,
        display: String,
    },
    ChunkData {
        dimension: Dimension,
        cx: i32,
        cy: i8,
        cz: i32,
        palette: Vec<BlockState>,
        /// A bit vector of indices into the palette.
        /// Each index is `palette.len().next_power_of_two().ilog2()` bits long.
        data: BitVec<u8, Lsb0>,
    },
    PlayerData {
        id: UserIdResponse,
        uuid: Uuid,
        data: Option<nbt::Blob>,
    },
}

#[derive(Debug, Protocol)]
pub enum ClientMessage {
    /// Should be sent by the client when the server sends [`ServerMessage::Ping`].
    /// If the client fails to do so within 30 seconds, the server may close the connection.
    Pong,
    /// Request to receive the current state of the chunk at the given position in the main world, and also receive state updates whenever the chunk changes.
    /// The chunk is read from disk, so data does not update in real time but rather only once every few minutes.
    SubscribeToChunk {
        dimension: Dimension,
        /// The chunk x coordinate, equivalent to the block x coordinates of the blocks in the chunk divided by 16
        cx: i32,
        /// The chunk y coordinate, equivalent to the block y coordinates of the blocks in the chunk divided by 16
        cy: i8,
        /// The chunk z coordinate, equivalent to the block z coordinates of the blocks in the chunk divided by 16
        cz: i32,
    },
    SubscribeToChunks(Vec<(Dimension, i32, i8, i32)>),
    SubscribeToInventory {
        player: UserIdRequest,
    },
}

#[derive(Debug, Clone, Protocol)]
pub enum UserIdRequest {
    Wmbid(String),
    Discord(UserId),
}

#[derive(Serialize)]
#[serde(untagged)]
enum SerializeId {
    Wmbid(String),
    Discord(UserId),
}

impl From<UserIdResponse> for SerializeId {
    fn from(value: UserIdResponse) -> Self {
        match value {
            UserIdResponse::Wmbid(wmbid) => Self::Wmbid(wmbid),
            UserIdResponse::Discord(discord_id) | UserIdResponse::Both { discord_id, .. } => Self::Discord(discord_id),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Protocol)]
#[serde(untagged, into = "SerializeId")]
pub enum UserIdResponse {
    Wmbid(String),
    Discord(UserId),
    Both {
        wmbid: String,
        discord_id: UserId,
    },
}

impl UserIdResponse {
    pub fn wmbid(&self) -> Option<&str> {
        match self {
            Self::Discord(_) => None,
            Self::Wmbid(wmbid) | Self::Both { wmbid, .. } => Some(&wmbid),
        }
    }

    pub fn discord_id(&self) -> Option<UserId> {
        match self {
            Self::Wmbid(_) => None,
            Self::Discord(discord_id) | Self::Both { discord_id, .. } => Some(*discord_id),
        }
    }

    pub fn url_part(&self) -> Cow<'_, str> {
        match self {
            Self::Wmbid(wmbid) => Cow::Borrowed(wmbid),
            Self::Discord(discord_id) | Self::Both { discord_id, .. } => Cow::Owned(discord_id.to_string()),
        }
    }
}

impl PartialEq for UserIdResponse {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Discord(discord_id1) | Self::Both { discord_id: discord_id1, .. }, Self::Discord(discord_id2) | Self::Both { discord_id: discord_id2, .. }) => discord_id1 == discord_id2,
            (Self::Wmbid(wmbid1) | Self::Both { wmbid: wmbid1, .. }, Self::Wmbid(wmbid2) | Self::Both { wmbid: wmbid2, .. }) => wmbid1 == wmbid2,
            (Self::Discord(_), Self::Wmbid(_)) | (Self::Wmbid(_), Self::Discord(_)) => false,
        }
    }
}

impl Eq for UserIdResponse {}

impl Hash for UserIdResponse {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Discord(discord_id) | Self::Both { discord_id, .. } => discord_id.hash(state),
            Self::Wmbid(wmbid) => wmbid.hash(state),
        }
    }
}
