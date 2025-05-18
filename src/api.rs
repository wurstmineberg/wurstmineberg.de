use {
    std::{
        array,
        collections::hash_map::{
            self,
            HashMap,
        },
        convert::Infallible as Never,
        iter,
        sync::Arc,
        time::{
            Duration,
            SystemTime,
        },
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
    rocket::{
        Either,
        State,
        fs::NamedFile,
        http::{
            ContentType,
            Status,
        },
        outcome::Outcome,
        request,
        serde::json::Json,
    },
    rocket_ws::WebSocket,
    sqlx::PgPool,
    tokio::{
        io::{
            self,
            AsyncReadExt as _,
        },
        select,
        time::{
            MissedTickBehavior,
            interval,
            sleep,
        },
    },
    wheel::{
        fs::File,
        traits::IoResultExt as _,
    },
    wurstmineberg_web::websocket::{
        ClientMessage,
        ServerMessage,
    },
    crate::user::{
        User,
        UserParam,
    },
};
#[cfg(not(target_os = "linux"))] use crate::systemd_minecraft;

#[rocket::get("/api/v3/discord/voice-state.json")]
pub(crate) async fn discord_voice_state(me: User) -> io::Result<NamedFile> {
    let _ = me; // only required for authorization
    NamedFile::open("/opt/wurstmineberg/discord/voice-state.json").await
}

#[derive(Debug, thiserror::Error, rocket_util::Error)]
pub(crate) enum Error {
    #[error(transparent)] Nbt(#[from] nbt::Error),
    #[error(transparent)] Sql(#[from] sqlx::Error),
    #[error(transparent)] SystemTime(#[from] std::time::SystemTimeError),
    #[error(transparent)] Wheel(#[from] wheel::Error),
}

#[rocket::get("/api/v3/world/<world>/player/<player>/playerdata.dat")]
pub(crate) async fn player_data(db_pool: &State<PgPool>, world: systemd_minecraft::World, player: UserParam<'_>) -> Result<Option<(ContentType, File)>, Error> {
    let Some(player) = player.parse(&**db_pool).await? else { return Ok(None) };
    let Some(uuid) = player.minecraft_uuid() else { return Ok(None) };
    Ok(Some((
        ContentType::new("application", "prs.nbt"), // as suggested at https://old.reddit.com/r/AskProgramming/comments/1eldcjt/mime_type_of_minecraft_nbt/lgrs5p4/
        File::open(world.dir().join("world").join("playerdata").join(format!("{uuid}.dat"))).await?,
    )))
}

#[rocket::get("/api/v3/world/<world>/player/<player>/playerdata.json")]
pub(crate) async fn player_data_json(db_pool: &State<PgPool>, world: systemd_minecraft::World, player: UserParam<'_>) -> Result<Option<Json<nbt::Blob>>, Error> {
    let Some(player) = player.parse(&**db_pool).await? else { return Ok(None) };
    let Some(uuid) = player.minecraft_uuid() else { return Ok(None) };
    let path = world.dir().join("world").join("playerdata").join(format!("{uuid}.dat"));
    let mut file = match File::open(&path).await {
        Ok(file) => file,
        Err(wheel::Error::Io { inner, .. }) if inner.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let mut buf = Vec::default();
    file.read_to_end(&mut buf).await.at(&path)?;
    let mut data = nbt::from_gzip_reader::<_, nbt::Blob>(&*buf)?;
    if data.get("apiTimeLastModified").is_none() {
        let metadata = file.metadata().await?;
        data.insert("apiTimeLastModified", metadata.modified().at(path)?.duration_since(SystemTime::UNIX_EPOCH)?.as_secs_f64())?;
    }
    if data.get("apiTimeResultFetched").is_none() {
        data.insert("apiTimeResultFetched", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs_f64())?;
    }
    Ok(Some(Json(data)))
}

type WsStream = SplitStream<rocket_ws::stream::DuplexStream>;
type WsSink = Arc<Mutex<SplitSink<rocket_ws::stream::DuplexStream, rocket_ws::Message>>>;

#[rocket::get("/api/v3/websocket")]
pub(crate) fn websocket(ws: request::Outcome<WebSocket, Never>, shutdown: rocket::Shutdown) -> Either<rocket_ws::Channel<'static>, Status> {
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

    match ws {
        Outcome::Success(ws) => Either::Left(ws.channel(|stream| Box::pin(async move {
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
        }))),
        Outcome::Error(never) => match never {},
        Outcome::Forward(status) => Either::Right(status),
    }
}
