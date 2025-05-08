use {
    async_proto::Protocol,
    mcanvil::{
        BlockState,
        Dimension,
    },
};

#[derive(Protocol)]
pub enum ServerMessage {
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
        data: Option<[[[BlockState; 16]; 16]; 16]>,
    }
}

#[derive(Protocol)]
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
}
