use {
    std::{
        array,
        collections::hash_map::{
            self,
            HashMap,
        },
        io,
        sync::Arc,
        time::Duration,
    },
    async_proto::Protocol as _,
    futures::stream::{
        SplitSink,
        SplitStream,
        StreamExt as _,
    },
    log_lock::*,
    mcanvil::{
        BlockState,
        Dimension,
        Region,
    },
    rocket::fs::NamedFile,
    rocket_ws::WebSocket,
    tokio::{
        select,
        time::{
            MissedTickBehavior,
            interval,
            sleep,
        },
    },
    wurstmineberg_web::websocket::{
        ClientMessage,
        ServerMessage,
    },
    crate::user::User,
};
#[cfg(not(target_os = "linux"))] use crate::systemd_minecraft;

#[rocket::get("/api/v3/discord/voice-state.json")]
pub(crate) async fn discord_voice_state(me: User) -> io::Result<NamedFile> {
    let _ = me; // only required for authorization
    NamedFile::open("/opt/wurstmineberg/discord/voice-state.json").await
}

type WsStream = SplitStream<rocket_ws::stream::DuplexStream>;
type WsSink = Arc<Mutex<SplitSink<rocket_ws::stream::DuplexStream, rocket_ws::Message>>>;

#[rocket::get("/api/v3/websocket")]
pub(crate) fn websocket(ws: WebSocket, shutdown: rocket::Shutdown) -> rocket_ws::Channel<'static> {
    #[derive(Debug, thiserror::Error)]
    enum Error {
        #[error(transparent)] ChunkColumnDecode(#[from] mcanvil::ChunkColumnDecodeError),
        #[error(transparent)] Read(#[from] async_proto::ReadError),
        #[error(transparent)] RegionDecode(#[from] mcanvil::RegionDecodeError),
        #[error(transparent)] Write(#[from] async_proto::WriteError),
    }

    async fn client_session(mut rocket_shutdown: rocket::Shutdown, mut stream: WsStream, sink: WsSink) -> Result<(), Error> {
        fn chunk_owned(world: &systemd_minecraft::World, dimension: Dimension, cx: i32, cy: i8, cz: i32) -> Result<Option<[Box<[[BlockState; 16]; 16]>; 16]>, Error> {
            let rx = cx.div_euclid(32);
            let rz = cz.div_euclid(32);
            Ok(if let Some(region) = Region::find(world.dir().join("world"), dimension, [rx, rz])? { //TODO Region::find_async
                region.chunk_column([cx, cz])?.and_then(|col| col.into_section_at(cy)).map(|chunk| array::from_fn(|y|
                    Box::new(array::from_fn(|z|
                        array::from_fn(|x|
                            chunk.block_relative([x as u8, y as u8, z as u8]).into_owned()
                        )
                    ))
                ))
            } else {
                None
            })
        }

        let main_world = systemd_minecraft::World::default();
        let mut save_data_interval = interval(Duration::from_secs(10 * 45));
        save_data_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let mut chunk_cache = HashMap::new();
        loop {
            select! {
                //TODO timeout after 60 seconds?
                () = &mut rocket_shutdown => break Ok(()),
                res = ClientMessage::read_ws021(&mut stream) => match res? {
                    ClientMessage::Pong => {}
                    ClientMessage::SubscribeToChunk { dimension, cx, cy, cz } => {
                        let new_chunk = chunk_owned(&main_world, dimension, cx, cy, cz)?;
                        match chunk_cache.entry((dimension, cx, cy, cz)) {
                            hash_map::Entry::Occupied(mut entry) => {
                                let old_chunk = entry.get_mut();
                                if new_chunk != *old_chunk {
                                    lock!(sink = sink; ServerMessage::ChunkData {
                                        data: new_chunk.clone(),
                                        dimension, cx, cy, cz,
                                    }.write_ws021(&mut *sink).await)?;
                                    *old_chunk = new_chunk;
                                }
                            }
                            hash_map::Entry::Vacant(entry) => {
                                lock!(sink = sink; ServerMessage::ChunkData {
                                    data: new_chunk.clone(),
                                    dimension, cx, cy, cz,
                                }.write_ws021(&mut *sink).await)?;
                                entry.insert(new_chunk);
                            }
                        }
                    }
                },
                _ = save_data_interval.tick() => for (&(dimension, cx, cy, cz), old_chunk) in &mut chunk_cache {
                    let new_chunk = chunk_owned(&main_world, dimension, cx, cy, cz)?;
                    if new_chunk != *old_chunk {
                        lock!(sink = sink; ServerMessage::ChunkData {
                            data: new_chunk.clone(),
                            dimension, cx, cy, cz,
                        }.write_ws021(&mut *sink).await)?;
                        *old_chunk = new_chunk;
                    }
                },
            }
        }
    }

    ws.channel(|stream| Box::pin(async move {
        let (ws_sink, ws_stream) = stream.split();
        let ws_sink = WsSink::new(Mutex::new(ws_sink));
        let ping_sink = ws_sink.clone();
        let ping_loop = tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(30)).await;
                if lock!(ping_sink = ping_sink; ServerMessage::Ping.write_ws021(&mut *ping_sink).await).is_err() { break } //TODO better error handling
            }
        });
        if let Err(e) = client_session(shutdown, ws_stream, ws_sink.clone()).await {
            let _ = lock!(ws_sink = ws_sink; ServerMessage::Error {
                debug: format!("{e:?}"),
                display: String::default(),
            }.write_ws021(&mut *ws_sink).await);
        }
        ping_loop.abort();
        Ok(())
    }))
}
