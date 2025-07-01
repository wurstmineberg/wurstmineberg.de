use {
    std::{
        array,
        collections::{
            BTreeMap,
            HashSet,
            hash_map::{
                self,
                HashMap,
            },
        },
        convert::Infallible as Never,
        iter,
        path::Path,
        pin::pin,
        sync::Arc,
        time::{
            Duration,
            SystemTime,
        },
    },
    async_proto::Protocol as _,
    bitvec::prelude::*,
    chrono::prelude::*,
    futures::stream::{
        self,
        SplitSink,
        SplitStream,
        StreamExt as _,
        TryStreamExt as _,
    },
    ics::ICalendar,
    itertools::Itertools as _,
    log_lock::*,
    mcanvil::{
        BlockState,
        ChunkSection,
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
        response::content::RawHtml,
        serde::json::Json,
    },
    rocket_util::{
        Origin,
        Response,
        html,
    },
    rocket_ws::WebSocket,
    serde::Serialize,
    sqlx::{
        PgPool,
        types::Json as PgJson,
    },
    tokio::{
        io::{
            self,
            AsyncReadExt as _,
        },
        select,
        sync::mpsc,
        time::{
            sleep,
            timeout,
        },
    },
    uuid::Uuid,
    wheel::{
        fs::File,
        traits::{
            IoResultExt as _,
            SendResultExt as _,
        },
    },
    wurstmineberg_web::websocket::{
        ClientMessage,
        ServerMessageV3,
        ServerMessageV4,
    },
    crate::{
        BASE_PATH,
        cal::{
            Event,
            EventKind,
        },
        http::{
            PageStyle,
            Tab,
            page,
        },
        user::{
            self,
            User,
            UserParam,
        },
    },
};
#[cfg(not(target_os = "linux"))] use crate::systemd_minecraft;

#[derive(Debug, thiserror::Error, rocket_util::Error)]
pub(crate) enum CalendarError {
    #[error(transparent)] Io(#[from] io::Error),
    #[error(transparent)] Sql(#[from] sqlx::Error),
}

fn ics_datetime<Tz: TimeZone>(datetime: DateTime<Tz>) -> String {
    format!("{}", datetime.with_timezone(&Utc).format("%Y%m%dT%H%M%SZ"))
}

#[rocket::get("/api/v3/calendar.ics")]
pub(crate) async fn calendar(db_pool: &State<PgPool>) -> Result<Response<ICalendar<'_>>, CalendarError> {
    let mut cal = ICalendar::new("2.0", concat!("wurstmineberg.de/", env!("CARGO_PKG_VERSION")));
    let mut events = sqlx::query_as!(Event, r#"SELECT id, start_time AS "start_time: DateTime<Utc>", end_time AS "end_time: DateTime<Utc>", kind as "kind: PgJson<EventKind>" FROM calendar"#).fetch(&**db_pool);
    while let Some(event) = events.try_next().await? {
        let mut cal_event = ics::Event::new(format!("event{}@wurstmineberg.de", event.id), ics_datetime(Utc::now()));
        cal_event.push(ics::properties::Summary::new(ics::escape_text(event.title(db_pool).await?)));
        if let Some(loc) = event.ics_location() {
            cal_event.push(ics::properties::Location::new(ics::escape_text(loc)));
        }
        cal_event.push(ics::properties::DtStart::new(ics_datetime(event.start_time)));
        cal_event.push(ics::properties::DtEnd::new(ics_datetime(event.end_time)));
        cal.add_event(cal_event);
    }
    Ok(Response(cal))
}

#[rocket::get("/api/v3/discord/voice-state.json")]
pub(crate) async fn discord_voice_state(me: User) -> io::Result<NamedFile> {
    let _ = me; // only required for authorization
    NamedFile::open(Path::new(BASE_PATH).join("discord").join("voice-state.json")).await
}

#[derive(Debug, thiserror::Error, rocket_util::Error)]
pub(crate) enum Error {
    #[error(transparent)] Minecraft(#[from] systemd_minecraft::Error),
    #[error(transparent)] Nbt(#[from] nbt::Error),
    #[error(transparent)] Ping(#[from] craftping::Error),
    #[error(transparent)] Sql(#[from] sqlx::Error),
    #[error(transparent)] SystemTime(#[from] std::time::SystemTimeError),
    #[error(transparent)] Uuid(#[from] uuid::Error),
    #[error(transparent)] Wheel(#[from] wheel::Error),
    #[error("unknown Minecraft UUID: {0}")]
    UnknownMinecraftUuid(Uuid),
}

#[derive(Serialize)]
pub(crate) struct WorldInfo {
    main: bool,
    running: bool,
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    list: Option<Vec<user::Id>>,
}

#[rocket::get("/api/v3/server/worlds.json")]
pub(crate) async fn worlds() -> Result<Json<BTreeMap<String, WorldInfo>>, Error> {
    stream::iter(systemd_minecraft::World::all().await?)
        .map(Ok)
        .and_then(async |world| Ok((world.to_string(), WorldInfo {
            main: world == systemd_minecraft::World::default(),
            running: world.is_running().await?,
            version: world.version().await?,
            list: None,
        })))
        .try_collect().await
        .map(Json)
}

#[rocket::get("/api/v3/server/worlds.json?list")]
pub(crate) async fn worlds_with_players(db_pool: &State<PgPool>) -> Result<Json<BTreeMap<String, WorldInfo>>, Error> {
    stream::iter(systemd_minecraft::World::all().await?)
        .map(Ok)
        .and_then(async |world| Ok((world.to_string(), WorldInfo {
            main: world == systemd_minecraft::World::default(),
            running: world.is_running().await?,
            version: world.version().await?,
            list: Some(match world.ping().await {
                Ok(ping) => {
                    let sample = ping.sample.unwrap_or_default();
                    let mut list = Vec::with_capacity(sample.len());
                    for player in sample {
                        let uuid = player.id.parse()?;
                        list.push(
                            User::from_minecraft_uuid(&**db_pool, uuid).await?
                                .ok_or_else(|| Error::UnknownMinecraftUuid(uuid))?
                                .id
                        );
                    }
                    list
                }
                Err(craftping::Error::Io(e)) if e.kind() == io::ErrorKind::ConnectionRefused => Vec::default(),
                Err(e) => return Err(e.into()),
            }),
        })))
        .try_collect().await
        .map(Json)
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

#[rocket::get("/api/v3/world/<world>/status.json")]
pub(crate) async fn world_status(db_pool: &State<PgPool>, world: systemd_minecraft::World) -> Result<Json<WorldInfo>, Error> {
    Ok(Json(WorldInfo {
        main: world == systemd_minecraft::World::default(),
        running: world.is_running().await?,
        version: world.version().await?,
        list: Some(match world.ping().await {
            Ok(ping) => {
                let sample = ping.sample.unwrap_or_default();
                let mut list = Vec::with_capacity(sample.len());
                for player in sample {
                    let uuid = player.id.parse()?;
                    list.push(
                        User::from_minecraft_uuid(&**db_pool, uuid).await?
                            .ok_or_else(|| Error::UnknownMinecraftUuid(uuid))?
                            .id
                    );
                }
                list
            }
            Err(craftping::Error::Io(e)) if e.kind() == io::ErrorKind::ConnectionRefused => Vec::default(),
            Err(e) => return Err(e.into()),
        }),
    }))
}

type WsStream = SplitStream<rocket_ws::stream::DuplexStream>;
type WsSink = Arc<Mutex<SplitSink<rocket_ws::stream::DuplexStream, rocket_ws::Message>>>;

#[derive(Clone, Copy)]
enum WsApiVersion {
    V3,
    V4,
}

impl WsApiVersion {
    async fn write_chunk(&self, sink: &WsSink, dimension: Dimension, cx: i32, cy: i8, cz: i32, chunk: Option<ChunkSection>) -> Result<(), async_proto::WriteError> {
        match self {
            Self::V3 => lock!(sink = sink; ServerMessageV3::ChunkData {
                data: chunk.map(|chunk| array::from_fn(|y|
                    Box::new(array::from_fn(|z|
                        array::from_fn(|x|
                            chunk.block_relative([x as u8, y as u8, z as u8]).into_owned()
                        )
                    ))
                )),
                dimension, cx, cy, cz,
            }.write_ws021(&mut *sink).await),
            Self::V4 => {
                let mut palette = Vec::default();
                let mut entries = Vec::default();
                if let Some(chunk) = chunk {
                    entries = Vec::with_capacity(16 * 16 * 16);
                    for y in 0..16 {
                        for z in 0..16 {
                            for x in 0..16 {
                                let block = chunk.block_relative([x, y, z]);
                                entries.push(if let Some(idx) = palette.iter().position(|iter_block| *iter_block == *block) {
                                    idx
                                } else {
                                    palette.push(block.into_owned());
                                    palette.len() - 1
                                });
                            }
                        }
                    }
                } else {
                    palette.push(BlockState::default());
                }
                let bits_per_entry = palette.len().checked_next_power_of_two().expect("16 * 16 * 16 > usize::MAX").ilog2().try_into().expect("(16 * 16 * 16).ilog2() > usize::MAX");
                let mut data = bitvec![u8, Lsb0; 0; 16 * 16 * 16 * bits_per_entry];
                if bits_per_entry > 0 {
                    for (entry, slice) in entries.into_iter().zip(data.chunks_mut(bits_per_entry)) {
                        slice.store_be(entry);
                    }
                }
                lock!(sink = sink; ServerMessageV4::ChunkData {
                    dimension, cx, cy, cz, palette, data,
                }.write_ws021(&mut *sink).await)
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum WsError {
    #[error(transparent)] ChunkColumnDecode(#[from] mcanvil::ChunkColumnDecodeError),
    #[error(transparent)] Elapsed(#[from] tokio::time::error::Elapsed),
    #[error(transparent)] Notify(#[from] notify::Error),
    #[error(transparent)] Read(#[from] async_proto::ReadError),
    #[error(transparent)] RegionDecode(#[from] mcanvil::RegionDecodeError),
    #[error(transparent)] Write(#[from] async_proto::WriteError),
    #[error("received empty error list from notify debouncer")]
    NotifyEmptyErrorList,
}

impl From<Vec<notify::Error>> for WsError {
    fn from(mut errors: Vec<notify::Error>) -> Self {
        if errors.is_empty() {
            Self::NotifyEmptyErrorList
        } else {
            Self::Notify(errors.swap_remove(0))
        }
    }
}

async fn client_session(mut rocket_shutdown: rocket::Shutdown, version: WsApiVersion, stream: WsStream, sink: WsSink) -> Result<(), WsError> {
    #[derive(Clone, Copy)]
    enum UpdateChunksReason {
        Subscribe,
        Notify,
    }

    async fn update_chunks(version: WsApiVersion, world: &systemd_minecraft::World, region_cache: &Mutex<HashMap<(Dimension, i32, i32), HashMap<(u8, i8, u8), Option<DateTime<Utc>>>>>, watcher: &Mutex<notify_debouncer_full::Debouncer<notify::RecommendedWatcher, notify_debouncer_full::RecommendedCache>>, sink: &WsSink, chunks: impl IntoIterator<Item = (Dimension, i32, i8, i32)>, reason: UpdateChunksReason) -> Result<(), WsError> {
        let chunks = chunks.into_iter().into_group_map_by(|(dimension, cx, _, cz)| (*dimension, cx.div_euclid(32), cz.div_euclid(32)));
        stream::iter(chunks)
            .map(Ok)
            .try_for_each_concurrent(None, move |((dimension, rx, rz), chunks)| async move {
                lock!(region_cache = region_cache; {
                    let chunk_cache = match region_cache.entry((dimension, rx, rz)) {
                        hash_map::Entry::Occupied(entry) => entry.into_mut(),
                        hash_map::Entry::Vacant(entry) => {
                            lock!(watcher = watcher; watcher.watch(&Region::path(world.dir().join("world"), dimension, [rx, rz]), notify::RecursiveMode::NonRecursive))?;
                            entry.insert(HashMap::default())
                        }
                    };
                    let should_check = match reason {
                        UpdateChunksReason::Subscribe => !chunks.iter().all(|(_, cx, cy, cz)| chunk_cache.contains_key(&(cx.rem_euclid(32) as u8, *cy, cz.rem_euclid(32) as u8))),
                        UpdateChunksReason::Notify => true, // already filtered
                    };
                    if should_check {
                        if let Some(mut region) = Region::find(world.dir().join("world"), dimension, [rx, rz]).await? {
                            for (_, cx, cy, cz) in chunks {
                                let cx_relative = cx.rem_euclid(32) as u8;
                                let cz_relative = cz.rem_euclid(32) as u8;
                                let new_timestamp = region.timestamps[32 * cz_relative as usize + cx_relative as usize];
                                let new_chunk = region.chunk_column_relative([cx_relative, cz_relative]).await?.and_then(|col| col.into_section_at(cy));
                                match chunk_cache.entry((cx_relative, cy, cz_relative)) {
                                    hash_map::Entry::Occupied(mut entry) => {
                                        let old_timestamp = entry.get_mut();
                                        if old_timestamp.is_none_or(|old_timestamp| new_timestamp != old_timestamp) {
                                            *old_timestamp = Some(new_timestamp);
                                            version.write_chunk(sink, dimension, cx, cy, cz, new_chunk).await?;
                                        }
                                    }
                                    hash_map::Entry::Vacant(entry) => {
                                        entry.insert(Some(new_timestamp));
                                        version.write_chunk(sink, dimension, cx, cy, cz, new_chunk).await?;
                                    }
                                }
                            }
                        } else {
                            for (_, cx, cy, cz) in chunks {
                                let cx_relative = cx.rem_euclid(32) as u8;
                                let cz_relative = cz.rem_euclid(32) as u8;
                                match chunk_cache.entry((cx_relative, cy, cz_relative)) {
                                    hash_map::Entry::Occupied(mut entry) => {
                                        let old_timestamp = entry.get_mut();
                                        if old_timestamp.is_some() {
                                            *old_timestamp = None;
                                            version.write_chunk(sink, dimension, cx, cy, cz, None).await?;
                                        }
                                    }
                                    hash_map::Entry::Vacant(entry) => {
                                        entry.insert(None);
                                        version.write_chunk(sink, dimension, cx, cy, cz, None).await?;
                                    }
                                }
                            }
                        }
                    }
                });
                Ok::<_, WsError>(())
            })
            .await?;
        Ok(())
    }

    let main_world = systemd_minecraft::World::default();
    let region_cache = Mutex::default();
    let (watch_tx, mut watch_rx) = mpsc::channel(1_024);
    let watcher = Mutex::new(notify_debouncer_full::new_debouncer(Duration::from_secs(45), None, move |res| watch_tx.blocking_send(res).allow_unreceived())?);
    let mut read = pin!(timeout(Duration::from_secs(60), ClientMessage::read_ws_owned021(stream)));
    loop {
        select! {
            biased;
            () = &mut rocket_shutdown => break Ok(()),
            res = &mut read => {
                let (stream, msg) = res??;
                read.set(timeout(Duration::from_secs(60), ClientMessage::read_ws_owned021(stream)));
                match msg {
                    ClientMessage::Pong => {}
                    ClientMessage::SubscribeToChunk { dimension, cx, cy, cz } => update_chunks(version, &main_world, &region_cache, &watcher, &sink, iter::once((dimension, cx, cy, cz)), UpdateChunksReason::Subscribe).await?,
                    ClientMessage::SubscribeToChunks(chunks) => update_chunks(version, &main_world, &region_cache, &watcher, &sink, chunks, UpdateChunksReason::Subscribe).await?,
                }
            }
            Some(res) = watch_rx.recv() => {
                let mut paths = HashSet::new();
                for event in res? {
                    if event.kind.is_modify() {
                        paths.extend(event.event.paths);
                    }
                }
                for region_path in paths {
                    let region = Region::open(region_path).await?;
                    if let Some(chunks) = lock!(region_cache = region_cache; region_cache.get(&(region.dimension, region.coords[0], region.coords[1])).map(|chunks| chunks.keys().map(|&(cx, cy, cz)| (region.dimension, region.coords[0] * 32 + i32::from(cx), cy, region.coords[1] * 32 + i32::from(cz))).collect_vec())) {
                        update_chunks(version, &main_world, &region_cache, &watcher, &sink, chunks, UpdateChunksReason::Notify).await?;
                    }
                }
            },
        }
    }
}

#[rocket::get("/api/v3/websocket")]
pub(crate) fn websocket_v3(me: Option<User>, uri: Origin<'_>, ws: request::Outcome<WebSocket, Never>, shutdown: rocket::Shutdown) -> Either<rocket_ws::Channel<'static>, (Status, RawHtml<String>)> {
    match ws {
        Outcome::Success(ws) => Either::Left(ws.channel(|stream| Box::pin(async move {
            let (ws_sink, ws_stream) = stream.split();
            let ws_sink = WsSink::new(Mutex::new(ws_sink));
            let ping_sink = ws_sink.clone();
            let ping_loop = tokio::spawn(async move {
                loop {
                    sleep(Duration::from_secs(30)).await;
                    if lock!(ping_sink = ping_sink; ServerMessageV3::Ping.write_ws021(&mut *ping_sink).await).is_err() { break } //TODO better error handling
                }
            });
            if let Err(e) = client_session(shutdown, WsApiVersion::V3, ws_stream, ws_sink.clone()).await {
                let _ = lock!(ws_sink = ws_sink; ServerMessageV3::Error {
                    debug: format!("{e:?}"),
                    display: String::default(),
                }.write_ws021(&mut *ws_sink).await);
            }
            ping_loop.abort();
            Ok(())
        }))),
        Outcome::Error(never) => match never {},
        Outcome::Forward(status) => Either::Right((status, page(&me, &uri, PageStyle::default(), "Bad Request — Wurstmineberg", Tab::More, html! {
            h1 : "Error 400: Bad Request";
            p {
                : "This API endpoint requires a ";
                a(href = "https://en.wikipedia.org/wiki/WebSocket") : "WebSocket";
                : " client. See ";
                a(href = "https://docs.rs/async-proto") : "https://docs.rs/async-proto";
                : " and ";
                a(href = "https://github.com/wurstmineberg/wurstmineberg.de/blob/main/src/websocket.rs") : "https://github.com/wurstmineberg/wurstmineberg.de/blob/main/src/websocket.rs";
                : " for the protocol.";
            }
        }))),
    }
}

#[rocket::get("/api/v4/websocket")]
pub(crate) fn websocket_v4(me: Option<User>, uri: Origin<'_>, ws: request::Outcome<WebSocket, Never>, shutdown: rocket::Shutdown) -> Either<rocket_ws::Channel<'static>, (Status, RawHtml<String>)> {
    match ws {
        Outcome::Success(ws) => Either::Left(ws.channel(|stream| Box::pin(async move {
            let (ws_sink, ws_stream) = stream.split();
            let ws_sink = WsSink::new(Mutex::new(ws_sink));
            let ping_sink = ws_sink.clone();
            let ping_loop = tokio::spawn(async move {
                loop {
                    sleep(Duration::from_secs(30)).await;
                    if lock!(ping_sink = ping_sink; ServerMessageV4::Ping.write_ws021(&mut *ping_sink).await).is_err() { break } //TODO better error handling
                }
            });
            if let Err(e) = client_session(shutdown, WsApiVersion::V4, ws_stream, ws_sink.clone()).await {
                let _ = lock!(ws_sink = ws_sink; ServerMessageV4::Error {
                    debug: format!("{e:?}"),
                    display: String::default(),
                }.write_ws021(&mut *ws_sink).await);
            }
            ping_loop.abort();
            Ok(())
        }))),
        Outcome::Error(never) => match never {},
        Outcome::Forward(status) => Either::Right((status, page(&me, &uri, PageStyle::default(), "Bad Request — Wurstmineberg", Tab::More, html! {
            h1 : "Error 400: Bad Request";
            p {
                : "This API endpoint requires a ";
                a(href = "https://en.wikipedia.org/wiki/WebSocket") : "WebSocket";
                : " client. See ";
                a(href = "https://docs.rs/async-proto") : "https://docs.rs/async-proto";
                : " and ";
                a(href = "https://github.com/wurstmineberg/wurstmineberg.de/blob/main/src/websocket.rs") : "https://github.com/wurstmineberg/wurstmineberg.de/blob/main/src/websocket.rs";
                : " for the protocol.";
            }
        }))),
    }
}
