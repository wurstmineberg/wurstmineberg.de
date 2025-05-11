use {
    std::{
        array,
        collections::hash_map::{
            self,
            HashMap,
        },
        io,
        iter,
        sync::Arc,
        time::Duration,
    },
    async_proto::Protocol as _,
    chrono::prelude::*,
    futures::stream::{
        SplitSink,
        SplitStream,
        StreamExt as _,
    },
    itertools::Itertools as _,
    log_lock::*,
    mcanvil::{
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

    async fn client_session(mut rocket_shutdown: rocket::Shutdown, stream: WsStream, sink: WsSink) -> Result<(), Error> {
        async fn update_chunks(world: &systemd_minecraft::World, chunk_cache: &mut HashMap<(Dimension, i32, i8, i32), Option<DateTime<Utc>>>, sink: &WsSink, chunks: impl IntoIterator<Item = (Dimension, i32, i8, i32)>) -> Result<(), Error> {
            let chunks = chunks.into_iter().into_group_map_by(|(dimension, cx, _, cz)| (*dimension, cx.div_euclid(32), cz.div_euclid(32)));
            for ((dimension, rx, rz), chunks) in chunks {
                if let Some(mut region) = Region::find(world.dir().join("world"), dimension, [rx, rz]).await? {
                    for (_, cx, cy, cz) in chunks {
                        let cx_relative = cx.rem_euclid(32) as u8;
                        let cz_relative = cz.rem_euclid(32) as u8;
                        let new_timestamp = region.timestamps[32 * cz_relative as usize + cx_relative as usize];
                        let new_chunk = region.chunk_column_relative([cx_relative, cz_relative]).await?.and_then(|col| col.into_section_at(cy)).map(|chunk| array::from_fn(|y|
                            Box::new(array::from_fn(|z|
                                array::from_fn(|x|
                                    chunk.block_relative([x as u8, y as u8, z as u8]).into_owned()
                                )
                            ))
                        ));
                        match chunk_cache.entry((dimension, cx, cy, cz)) {
                            hash_map::Entry::Occupied(mut entry) => {
                                let old_timestamp = entry.get_mut();
                                if old_timestamp.is_none_or(|old_timestamp| new_timestamp != old_timestamp) {
                                    lock!(sink = sink; ServerMessage::ChunkData {
                                        data: new_chunk.clone(),
                                        dimension, cx, cy, cz,
                                    }.write_ws021(&mut *sink).await)?;
                                    *old_timestamp = Some(new_timestamp);
                                }
                            }
                            hash_map::Entry::Vacant(entry) => {
                                lock!(sink = sink; ServerMessage::ChunkData {
                                    data: new_chunk.clone(),
                                    dimension, cx, cy, cz,
                                }.write_ws021(&mut *sink).await)?;
                                entry.insert(Some(new_timestamp));
                            }
                        }
                    }
                } else {
                    for (_, cx, cy, cz) in chunks {
                        match chunk_cache.entry((dimension, cx, cy, cz)) {
                            hash_map::Entry::Occupied(mut entry) => {
                                let old_timestamp = entry.get_mut();
                                if old_timestamp.is_some() {
                                    lock!(sink = sink; ServerMessage::ChunkData {
                                        data: None,
                                        dimension, cx, cy, cz,
                                    }.write_ws021(&mut *sink).await)?;
                                    *old_timestamp = None;
                                }
                            }
                            hash_map::Entry::Vacant(entry) => {
                                lock!(sink = sink; ServerMessage::ChunkData {
                                    data: None,
                                    dimension, cx, cy, cz,
                                }.write_ws021(&mut *sink).await)?;
                                entry.insert(None);
                            }
                        }
                    }
                }
            }
            Ok(())
        }

        let main_world = systemd_minecraft::World::default();
        let mut save_data_interval = interval(Duration::from_secs(10 * 45));
        save_data_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let mut chunk_cache = HashMap::new();
        let mut read = ClientMessage::read_ws_owned021(stream); //TODO timeout after 60 seconds?
        loop {
            select! {
                () = &mut rocket_shutdown => break Ok(()),
                res = &mut read => {
                    let (stream, msg) = res?;
                    read = ClientMessage::read_ws_owned021(stream); //TODO timeout after 60 seconds?
                    match msg {
                        ClientMessage::Pong => {}
                        ClientMessage::SubscribeToChunk { dimension, cx, cy, cz } => update_chunks(&main_world, &mut chunk_cache, &sink, iter::once((dimension, cx, cy, cz))).await?,
                        ClientMessage::SubscribeToChunks(chunks) => update_chunks(&main_world, &mut chunk_cache, &sink, chunks).await?,
                    }
                }
                _ = save_data_interval.tick() => {
                    let chunks = chunk_cache.keys().copied().collect_vec();
                    update_chunks(&main_world, &mut chunk_cache, &sink, chunks).await?;
                }
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
