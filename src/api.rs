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
        fmt::{
            self,
            Write as _,
        },
        iter,
        num::NonZero,
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
    chrono_tz::Tz,
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
        BlockEntity,
        BlockState,
        ChunkSection,
        Dimension,
        Region,
    },
    notify::Watcher as _,
    rocket::{
        Either,
        State,
        fs::NamedFile,
        http::{
            ContentType,
            Status,
            impl_from_uri_param_identity,
            uri::{
                self,
                fmt::{
                    FromUriParam,
                    UriDisplay,
                },
            },
        },
        outcome::Outcome,
        request::{
            self,
            FromParam,
        },
        response::{
            Redirect,
            content::RawHtml,
        },
        serde::json::Json,
        uri,
    },
    rocket_util::{
        Origin,
        Response,
        html,
    },
    rocket_ws::WebSocket,
    serde::Serialize,
    serenity::model::prelude::*,
    sqlx::{
        PgPool,
        types::Json as PgJson,
    },
    tiny_skia::Pixmap,
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
    url::Url,
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
        about,
        auth,
        cal::{
            Event,
            EventKind,
        },
        http::{
            PageStyle,
            StatusOrError,
            Tab,
            asset,
            base_uri,
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

#[derive(Default)]
pub(crate) enum Version {
    V1,
    V2,
    V3,
    #[default]
    V4,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum VersionFromParamError {
    #[error(transparent)] ParseInt(#[from] std::num::ParseIntError),
    #[error("API version {0} does not exist yet")]
    Future(u8),
    #[error("API version path segment should start with “v”")]
    Prefix,
    #[error("API version 0 never existed")]
    V0,
}

impl FromParam<'_> for Version {
    type Error = VersionFromParamError;

    fn from_param(param: &str) -> Result<Self, Self::Error> {
        if let Some(version) = param.strip_prefix('v') {
            match version.parse()? {
                0 => Err(VersionFromParamError::V0),
                1 => Ok(Self::V1),
                2 => Ok(Self::V2),
                3 => Ok(Self::V3),
                4 => Ok(Self::V4),
                version => Err(VersionFromParamError::Future(version)),
            }
        } else {
            Err(VersionFromParamError::Prefix)
        }
    }
}

impl UriDisplay<uri::fmt::Path> for Version {
    fn fmt(&self, f: &mut uri::fmt::Formatter<'_, uri::fmt::Path>) -> fmt::Result {
        match self {
            Self::V1 => write!(f, "v1"),
            Self::V2 => write!(f, "v2"),
            Self::V3 => write!(f, "v3"),
            Self::V4 => write!(f, "v4"),
        }
    }
}

impl_from_uri_param_identity!([uri::fmt::Path] Version);

impl TryFrom<Version> for ActiveVersion {
    type Error = Status;

    fn try_from(version: Version) -> Result<Self, Self::Error> {
        match version {
            Version::V1 | Version::V2 => Err(Status::Gone),
            Version::V3 => Ok(Self::V3),
            Version::V4 => Ok(Self::V4),
        }
    }
}

impl From<ActiveVersion> for Version {
    fn from(version: ActiveVersion) -> Self {
        match version {
            ActiveVersion::V3 => Self::V3,
            ActiveVersion::V4 => Self::V4,
        }
    }
}

#[derive(Clone, Copy, Serialize)]
#[serde(into = "NonZero<u8>")]
enum ActiveVersion {
    V3,
    V4,
}

impl From<ActiveVersion> for NonZero<u8> {
    fn from(version: ActiveVersion) -> Self {
        match version {
            ActiveVersion::V3 => Self::new(3),
            ActiveVersion::V4 => Self::new(4),
        }.unwrap()
    }
}

impl FromUriParam<uri::fmt::Path, ActiveVersion> for Version {
    type Target = Version;

    fn from_uri_param(param: ActiveVersion) -> Self::Target {
        param.into()
    }
}

#[rocket::get("/api")]
pub(crate) fn index() -> Redirect {
    Redirect::temporary(uri!(docs(Version::default())))
}

#[rocket::get("/api/<version>")]
pub(crate) fn docs(me: Option<User>, uri: Origin<'_>, version: Version) -> Result<RawHtml<String>, Status> {
    let version = ActiveVersion::try_from(version)?;
    Ok(page(&me, &uri, PageStyle::default(), "Wurstmineberg API", Tab::More, html! {
        p {
            : "The ";
            strong : "Wurstmineberg API";
            : " is a part of the website intended to be used with apps other than web browsers. Some endpoints are only available for Wurstmineberg members; using your API key, you can access these without signing into Discord. If asked for login credentials, enter ";
            code : "api";
            : " as the username and your API key as the password.";
        }
        @if let Some(me) = &me {
            p {
                : "Your API key: ";
                code(class = "spoiler") : me.api_key;
            }
            p {
                : "If your API key falls into the wrong hands, please ";
                a(class = "btn btn-primary", href = format!("/people/{}/reset-key", me.id.url_part())) : "generate a new API key";
                : ". You will then have to sign in with the new key anywhere you're using the old one.";
            }
        } else {
            p {
                a(href = uri!(auth::discord_login(Some(uri!(docs(Version::V3)))))) : "Log in";
                : " to view your API key.";
            }
        }
        h1 : "Endpoints";
        h2 {
            a(href = uri!(calendar(version))) {
                code : uri!(calendar(version));
            }
        }
        p {
            : "Our special events calendar in ";
            a(href = "https://en.wikipedia.org/wiki/ICalendar") : "iCalendar";
            : " format. To subscribe:";
        }
        ul {
            li {
                : "In Google Calendar, select ";
                a(href = "https://calendar.google.com/calendar/u/0/r/settings/addbyurl") : "Add calendar → From URL";
            }
            li {
                : "In Apple Calendar, press ";
                kbd : "⌥";
                kbd : "⌘";
                kbd : "S";
                : " or select File → New Calendar Subscription";
            }
            li : "In Mozilla Thunderbird, select New Calendar → On the Network. Paste the link into the “Location” field and click “Find Calendars”, then “Properties”. Enable “Read Only” and click “OK”, then “Subscribe”.";
        }
        h2 {
            a(href = uri!(discord_voice_state(version))) {
                code : uri!(discord_voice_state(version));
            }
        }
        p : "Info about who is currently in which voice channels. API key required."; //TODO document JSON schema
        h2 {
            a(href = uri!(websocket(version))) {
                code : uri!(websocket(version));
            }
        }
        p {
            : "See ";
            a(href = "https://docs.rs/async-proto") : "https://docs.rs/async-proto";
            : " and ";
            a(href = "https://github.com/wurstmineberg/wurstmineberg.de/blob/main/src/websocket.rs") : "https://github.com/wurstmineberg/wurstmineberg.de/blob/main/src/websocket.rs";
            : " for the protocol.";
        } //TODO better docs
        h2 {
            a(href = uri!(money_overview(version))) {
                code : uri!(money_overview(version));
            }
        }
        p {
            : "Summary of the current financial situation, see ";
            a(href = uri!(_, about::get, "#finance")) : "our about page";
            : " for details.";
        } //TODO document JSON schema
        h2 {
            a(href = uri!(money_transactions(version))) {
                code : uri!(money_transactions(version));
            }
        }
        p {
            : "Anonymized history of financial transactions, see ";
            a(href = uri!(_, about::get, "#finance")) : "our about page";
            : " for details.";
        } //TODO document JSON schema
        h2 {
            a(href = uri!(people(version))) {
                code : uri!(people(version));
            }
        }
        p : "Information about the People of Wurstmineberg (current and former server members as well as guests)."; //TODO document JSON schema
        h2 {
            code {
                : "/api/v";
                : NonZero::<u8>::from(version);
                : "/person/<user>/avatar.json";
            }
        }
        p : "Information about available profile pictures of the given Person."; //TODO document JSON schema
        h2 {
            code {
                : "/api/v";
                : NonZero::<u8>::from(version);
                : "/person/<user>/skin/front.png";
            }
        }
        p : "A 16×32 image showing a front view of the player's skin (with hat layer).";
        h2 {
            code {
                : "/api/v";
                : NonZero::<u8>::from(version);
                : "/person/<user>/skin/head.png";
            }
        }
        p : "An 8×8 image showing the player's head (with hat layer).";
        h2 {
            a(href = uri!(worlds(version))) {
                code : uri!(worlds(version));
            }
        }
        p {
            : "An object mapping existing world names to short status summaries (like those returned by ";
            code {
                : "/api/v";
                : NonZero::<u8>::from(version);
                : "/world/<world>/status.json";
            }
            : " but the lists of online players are omitted unless specified using ";
            code : "?list=1";
            : ").";
        }
        h2 {
            code {
                : "/api/v";
                : NonZero::<u8>::from(version);
                : "/world/<world>/dim/<dimension>/chunk/<x>/<y>/<z>.json";
            }
        }
        p : "A JSON representation of a chunk section."; //TODO JSON schema
        h2 {
            code {
                : "/api/v";
                : NonZero::<u8>::from(version);
                : "/world/<world>/dim/<dimension>/chunk-column/<x>/<z>.json";
            }
        }
        p : "A JSON representation of a chunk column."; //TODO JSON schema
        h2 {
            code {
                : "/api/v";
                : NonZero::<u8>::from(version);
                : "/world/<world>/dim/<dimension>/region/<x>/<z>.mca";
            }
        }
        p {
            : "A raw region file in ";
            a(href = "https://minecraft.wiki/w/Anvil_file_format") : "Anvil";
            : " format.";
        }
        h2 {
            code {
                : "/api/v";
                : NonZero::<u8>::from(version);
                : "/world/<world>/level.json";
            }
        }
        p {
            : "A JSON representation of the ";
            a(href = "https://minecraft.wiki/w/Java_Edition_level_format#level.dat_format") {
                code : "level.dat";
            }
            : " file.";
        }
        h2 {
            code {
                : "/api/v";
                : NonZero::<u8>::from(version);
                : "/world/<world>/level.dat";
            }
        }
        p {
            : "The raw ";
            a(href = "https://minecraft.wiki/w/Java_Edition_level_format#level.dat_format") {
                code : "level.dat";
            }
            : " file in ";
            a(href = "https://minecraft.wiki/w/NBT_format") : "NBT";
            : " format.";
        }
        h2 {
            code {
                : "/api/v";
                : NonZero::<u8>::from(version);
                : "/world/<world>/player/<user>/playerdata.json";
            }
        }
        p {
            : "A JSON representation of the given Person's ";
            a(href = "https://minecraft.wiki/w/Player.dat_format") : "player state file";
            : ".";
        }
        h2 {
            code {
                : "/api/v";
                : NonZero::<u8>::from(version);
                : "/world/<world>/player/<user>/playerdata.dat";
            }
        }
        p {
            : "The raw ";
            a(href = "https://minecraft.wiki/w/NBT_format") : "NBT";
            : " version of the given Person's ";
            a(href = "https://minecraft.wiki/w/Player.dat_format") : "player state file";
            : ".";
        }
        h2 {
            code {
                : "/api/v";
                : NonZero::<u8>::from(version);
                : "/world/<world>/player/<user>/stats.json";
            }
        }
        p : "The player's stats formatted as JSON with stats grouped into objects by category."; //TODO JSON schema
        h2 {
            code {
                : "/api/v";
                : NonZero::<u8>::from(version);
                : "/world/<world>/status.json";
            }
        }
        p : "A short status summary for this world."; //TODO JSON schema
    }))
}

#[derive(Debug, thiserror::Error, rocket_util::Error)]
pub(crate) enum CalendarError {
    #[error(transparent)] Io(#[from] io::Error),
    #[error(transparent)] Sql(#[from] sqlx::Error),
}

impl<E: Into<CalendarError>> From<E> for StatusOrError<CalendarError> {
    fn from(e: E) -> Self {
        Self::Err(e.into())
    }
}

#[rocket::get("/api/<version>/calendar.ics")]
pub(crate) async fn calendar(db_pool: &State<PgPool>, version: Version) -> Result<Response<ICalendar<'_>>, StatusOrError<CalendarError>> {
    fn ics_datetime<Tz: TimeZone>(datetime: DateTime<Tz>) -> String {
        format!("{}", datetime.with_timezone(&Utc).format("%Y%m%dT%H%M%SZ"))
    }

    let _ /* no version differences */ = ActiveVersion::try_from(version)?;
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

#[rocket::get("/api/<version>/discord/voice-state.json")]
pub(crate) async fn discord_voice_state(me: User, version: Version) -> Result<NamedFile, StatusOrError<io::Error>> {
    let _ = me; // only required for authorization
    let _ /* no version differences */ = ActiveVersion::try_from(version)?;
    Ok(NamedFile::open(Path::new(BASE_PATH).join("discord").join("voice-state.json")).await.map_err(StatusOrError::Err)?) //TODO take voice state directly from wurstminebot task
}

#[rocket::get("/api/<version>/money/overview.json")]
pub(crate) fn money_overview(version: Version) -> Result<Redirect, Status> {
    let _ /* no version differences */ = ActiveVersion::try_from(version)?;
    Ok(Redirect::temporary("https://night.fenhl.net/wurstmineberg/money/overview.json")) // temporary redirect in case the schema changes on the backend or someone else takes over bookkeeping
}

#[rocket::get("/api/<version>/money/transactions.json")]
pub(crate) fn money_transactions(version: Version) -> Result<Redirect, Status> {
    let _ /* no version differences */ = ActiveVersion::try_from(version)?;
    //TODO deanonymize own transactions
    Ok(Redirect::temporary("https://night.fenhl.net/wurstmineberg/money/transactions.json")) // temporary redirect in case the schema changes on the backend or someone else takes over bookkeeping
}

#[derive(Serialize)]
struct DiscordData {
    snowflake: UserId,
    avatar: Option<Url>,
    joined: DateTime<Utc>,
    nick: Option<String>,
    roles: Vec<RoleId>,
    username: String,
    discriminator: Option<NonZero<u16>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Person {
    // can't use #[serde(flatten)] because it can't be combined with deny_unknown_fields on the flattened field
    #[serde(skip_serializing_if = "Option::is_none")]
    base: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    discord: Option<DiscordData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fav_color: Option<user::Color>,
    #[serde(default, skip_serializing_if = "user::DataMinecraft::is_default")]
    minecraft: user::DataMinecraft,
    #[serde(skip_serializing_if = "Option::is_none")]
    mojira: Option<String>,
    name: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    options: BTreeMap<String, bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    slack: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    status_history: Vec<user::StatusHistoryItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    twitch: Option<user::DataTwitch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timezone: Option<Tz>,
    #[serde(skip_serializing_if = "Option::is_none")]
    twitter: Option<user::DataTwitter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    website: Option<Url>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wiki: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct People {
    version: ActiveVersion,
    people: HashMap<user::Id, Person>,
}

#[derive(Debug, thiserror::Error, rocket_util::Error)]
pub(crate) enum Error {
    #[error(transparent)] Minecraft(#[from] systemd_minecraft::Error),
    #[error(transparent)] Nbt(#[from] nbt::Error),
    #[error(transparent)] Ping(#[from] craftping::Error),
    #[error(transparent)] PlayerHead(#[from] playerhead::Error),
    #[error(transparent)] Sql(#[from] sqlx::Error),
    #[error(transparent)] SystemTime(#[from] std::time::SystemTimeError),
    #[error(transparent)] Url(#[from] url::ParseError),
    #[error(transparent)] Uuid(#[from] uuid::Error),
    #[error(transparent)] Wheel(#[from] wheel::Error),
    #[error("unknown Minecraft UUID: {0}")]
    UnknownMinecraftUuid(Uuid),
}

impl<E: Into<Error>> From<E> for StatusOrError<Error> {
    fn from(e: E) -> Self {
        Self::Err(e.into())
    }
}

#[rocket::get("/api/<version>/people.json")]
pub(crate) async fn people(db_pool: &State<PgPool>, version: Version) -> Result<Json<People>, StatusOrError<Error>> {
    let version = ActiveVersion::try_from(version)?;
    let mut people = People {
        people: HashMap::default(),
        version,
    };
    let mut all_people = User::all(&**db_pool);
    while let Some(person) = all_people.try_next().await? {
        let name = person.to_string();
        people.people.insert(person.id.clone(), Person {
            base: person.data.base,
            description: person.data.description,
            discord: person.id.discord_id().and_then(|snowflake| Some((snowflake, person.discorddata?))).map(|(snowflake, discorddata)| DiscordData {
                avatar: discorddata.avatar,
                joined: discorddata.joined,
                nick: discorddata.nick,
                roles: discorddata.roles,
                username: discorddata.username,
                discriminator: discorddata.discriminator,
                snowflake,
            }),
            fav_color: person.data.fav_color,
            minecraft: person.data.minecraft,
            mojira: person.data.mojira,
            options: person.data.options,
            slack: person.data.slack,
            status_history: person.data.status_history,
            twitch: person.data.twitch,
            timezone: person.data.timezone,
            twitter: person.data.twitter,
            website: person.data.website,
            wiki: person.data.wiki,
            name,
        });
    }
    Ok(Json(people))
}

#[derive(Serialize)]
pub(crate) struct AvatarInfo {
    url: Url,
    pixelate: bool,
    fallbacks: Vec<AvatarFallback>,
}

#[derive(Serialize)]
struct AvatarFallback {
    url: Url,
    pixelate: bool,
}

#[rocket::get("/api/<version>/person/<user>/avatar.json")]
pub(crate) async fn user_avatar(db_pool: &State<PgPool>, version: Version, user: UserParam<'_>) -> Result<Option<Json<AvatarInfo>>, StatusOrError<Error>> {
    let version = ActiveVersion::try_from(version)?;
    let Some(user) = user.parse(&**db_pool).await? else { return Ok(None) };
    let mut fallbacks = Vec::default();
    // Discord avatar
    if let Some(discorddata) = user.discorddata && let Some(avatar) = discorddata.avatar {
        fallbacks.push(AvatarFallback {
            url: avatar,
            pixelate: false,
        });
    }
    // player head
    if user.data.minecraft.uuid.is_some() {
        fallbacks.push(AvatarFallback {
            url: uri!(base_uri(), player_head(version, &user.id)).to_string().parse()?,
            pixelate: true,
        });
    }
    // placeholder
    fallbacks.push(AvatarFallback {
        url: asset("/img/grid-unknown.png").parse()?,
        pixelate: true,
    });
    // API format
    let AvatarFallback { url, pixelate } = fallbacks.remove(0);
    Ok(Some(Json(AvatarInfo { url, pixelate, fallbacks })))
}

#[rocket::get("/api/<version>/person/<user>/skin/front.png")]
pub(crate) async fn player_skin_front(db_pool: &State<PgPool>, http_client: &State<reqwest::Client>, version: Version, user: UserParam<'_>) -> Result<Option<Response<Pixmap>>, StatusOrError<Error>> {
    let _ /* no version differences */ = ActiveVersion::try_from(version)?;
    let Some(user) = user.parse(&**db_pool).await? else { return Ok(None) };
    let Some(uuid) = user.minecraft_uuid() else { return Ok(None) };
    Ok(Some(Response(playerhead::front(http_client, uuid).await?)))
}

#[rocket::get("/api/<version>/person/<user>/skin/head.png")]
pub(crate) async fn player_head(db_pool: &State<PgPool>, http_client: &State<reqwest::Client>, version: Version, user: UserParam<'_>) -> Result<Option<Response<Pixmap>>, StatusOrError<Error>> {
    let _ /* no version differences */ = ActiveVersion::try_from(version)?;
    let Some(user) = user.parse(&**db_pool).await? else { return Ok(None) };
    let Some(uuid) = user.minecraft_uuid() else { return Ok(None) };
    Ok(Some(Response(playerhead::head(http_client, uuid).await?)))
}

#[derive(Serialize)]
pub(crate) struct WorldInfo {
    main: bool,
    running: bool,
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    list: Option<Vec<user::Id>>,
}

#[rocket::get("/api/<version>/server/worlds.json")]
pub(crate) async fn worlds(version: Version) -> Result<Json<BTreeMap<String, WorldInfo>>, StatusOrError<Error>> {
    let _ /* no version differences */ = ActiveVersion::try_from(version)?;
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

#[rocket::get("/api/<version>/server/worlds.json?list")]
pub(crate) async fn worlds_with_players(db_pool: &State<PgPool>, version: Version) -> Result<Json<BTreeMap<String, WorldInfo>>, StatusOrError<Error>> {
    let _ /* no version differences */ = ActiveVersion::try_from(version)?;
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

#[rocket::get("/api/<version>/world/<world>/player/<player>/playerdata.dat")]
pub(crate) async fn player_data(db_pool: &State<PgPool>, version: Version, world: systemd_minecraft::World, player: UserParam<'_>) -> Result<Option<(ContentType, File)>, StatusOrError<Error>> {
    let _ /* no version differences */ = ActiveVersion::try_from(version)?;
    let Some(player) = player.parse(&**db_pool).await? else { return Ok(None) };
    let Some(uuid) = player.minecraft_uuid() else { return Ok(None) };
    Ok(Some((
        ContentType::new("application", "prs.nbt"), // as suggested at https://old.reddit.com/r/AskProgramming/comments/1eldcjt/mime_type_of_minecraft_nbt/lgrs5p4/
        File::open(world.dir().join("world").join("playerdata").join(format!("{uuid}.dat"))).await?,
    )))
}

#[rocket::get("/api/<version>/world/<world>/player/<player>/playerdata.json")]
pub(crate) async fn player_data_json(db_pool: &State<PgPool>, version: Version, world: systemd_minecraft::World, player: UserParam<'_>) -> Result<Option<Json<nbt::Blob>>, StatusOrError<Error>> {
    let _ /* no version differences */ = ActiveVersion::try_from(version)?;
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
    let mut data = nbt::Blob::from_gzip_reader(&mut &*buf)?;
    if data.get("apiTimeLastModified").is_none() {
        let metadata = file.metadata().await?;
        data.insert("apiTimeLastModified", metadata.modified().at(path)?.duration_since(SystemTime::UNIX_EPOCH)?.as_secs_f64())?;
    }
    if data.get("apiTimeResultFetched").is_none() {
        data.insert("apiTimeResultFetched", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs_f64())?;
    }
    Ok(Some(Json(data)))
}

#[rocket::get("/api/<version>/world/<world>/status.json")]
pub(crate) async fn world_status(db_pool: &State<PgPool>, version: Version, world: systemd_minecraft::World) -> Result<Json<WorldInfo>, StatusOrError<Error>> {
    let _ /* no version differences */ = ActiveVersion::try_from(version)?;
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

/// WebSocket API differences
impl ActiveVersion {
    async fn write_custom_error(&self, sink: &WsSink, debug: impl fmt::Debug, display: impl fmt::Display) -> Result<(), async_proto::WriteError> {
        match self {
            Self::V3 => lock!(sink = sink; ServerMessageV3::Error {
                debug: format!("{debug:?}"),
                display: display.to_string(),
            }.write_ws021(&mut *sink).await),
            Self::V4 => lock!(sink = sink; ServerMessageV4::Error {
                debug: format!("{debug:?}"),
                display: display.to_string(),
            }.write_ws021(&mut *sink).await),
        }
    }

    async fn write_chunk(&self, sink: &WsSink, dimension: Dimension, cx: i32, cy: i8, cz: i32, chunk: Option<&ChunkSection>) -> Result<(), async_proto::WriteError> {
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

    async fn write_player(&self, sink: &WsSink, id: user::Id, uuid: Uuid, data: Option<nbt::Blob>) -> Result<(), async_proto::WriteError> {
        match self {
            Self::V3 => lock!(sink = sink; ServerMessageV3::PlayerData { id, uuid, data }.write_ws021(&mut *sink).await),
            Self::V4 => lock!(sink = sink; ServerMessageV4::PlayerData { id, uuid, data }.write_ws021(&mut *sink).await),
        }
    }

    async fn write_block_entities(&self, sink: &WsSink, dimension: Dimension, cx: i32, cz: i32, data: Vec<BlockEntity>) -> Result<(), async_proto::WriteError> {
        match self {
            Self::V3 => lock!(sink = sink; ServerMessageV3::BlockEntities { dimension, cx, cz, data }.write_ws021(&mut *sink).await),
            Self::V4 => lock!(sink = sink; ServerMessageV4::BlockEntities { dimension, cx, cz, data }.write_ws021(&mut *sink).await),
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum WsError {
    #[error(transparent)] ChunkColumnDecode(#[from] mcanvil::ChunkColumnDecodeError),
    #[error(transparent)] Elapsed(#[from] tokio::time::error::Elapsed),
    #[error(transparent)] Nbt(#[from] nbt::Error),
    #[error(transparent)] Notify(#[from] notify::Error),
    #[error(transparent)] Read(#[from] async_proto::ReadError),
    #[error(transparent)] RegionDecode(#[from] mcanvil::RegionDecodeError),
    #[error(transparent)] Sql(#[from] sqlx::Error),
    #[error(transparent)] Uuid(#[from] uuid::Error),
    #[error(transparent)] Wheel(#[from] wheel::Error),
    #[error(transparent)] Write(#[from] async_proto::WriteError),
    #[error("received empty error list from notify debouncer")]
    NotifyEmptyErrorList,
    #[error("received unknown path from notifier")]
    NotifyUnexpectedFile,
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

async fn client_session(db_pool: PgPool, mut rocket_shutdown: rocket::Shutdown, version: ActiveVersion, stream: WsStream, sink: WsSink) -> Result<(), WsError> {
    #[derive(Default, Clone, Copy)]
    struct Subscriptions {
        block_states: bool,
        block_entities: bool,
    }

    impl Subscriptions {
        fn add(&mut self, reason: ChunkUpdateReason) {
            match reason {
                ChunkUpdateReason::SubscribeBlockStates => self.block_states = true,
                ChunkUpdateReason::SubscribeBlockEntities => self.block_entities = true,
                ChunkUpdateReason::Notify => {}
            }
        }
    }

    #[derive(Clone, Copy)]
    enum ChunkUpdateReason {
        SubscribeBlockStates,
        SubscribeBlockEntities,
        Notify,
    }

    #[derive(Clone, Copy)]
    enum PlayerUpdateReason {
        Subscribe,
        Notify,
    }

    async fn update_chunks(version: ActiveVersion, world: &systemd_minecraft::World, region_cache: &Mutex<HashMap<(Dimension, i32, i32), HashMap<(u8, i8, u8), (Subscriptions, Option<DateTime<Utc>>)>>>, watcher: &Mutex<notify::RecommendedWatcher>, sink: &WsSink, chunks: impl IntoIterator<Item = (Dimension, i32, i8, i32)>, reason: ChunkUpdateReason) -> Result<(), WsError> {
        let chunks = chunks.into_iter().into_group_map_by(|(dimension, cx, _, cz)| (*dimension, cx.div_euclid(32), cz.div_euclid(32)));
        lock!(region_cache = region_cache; for ((dimension, rx, rz), chunks) in chunks {
            let chunk_cache = match region_cache.entry((dimension, rx, rz)) {
                hash_map::Entry::Occupied(entry) => entry.into_mut(),
                hash_map::Entry::Vacant(entry) => {
                    lock!(watcher = watcher; watcher.watch(&Region::path(world.dir().join("world"), dimension, [rx, rz]), notify::RecursiveMode::NonRecursive))?;
                    entry.insert(HashMap::default())
                }
            };
            let should_check = match reason {
                ChunkUpdateReason::SubscribeBlockStates => !chunks.iter().all(|(_, cx, cy, cz)| chunk_cache.get(&(cx.rem_euclid(32) as u8, *cy, cz.rem_euclid(32) as u8)).is_some_and(|(subscriptions, _)| subscriptions.block_states)),
                ChunkUpdateReason::SubscribeBlockEntities => !chunks.iter().all(|(_, cx, cy, cz)| chunk_cache.get(&(cx.rem_euclid(32) as u8, *cy, cz.rem_euclid(32) as u8)).is_some_and(|(subscriptions, _)| subscriptions.block_entities)),
                ChunkUpdateReason::Notify => true, // already filtered
            };
            if should_check {
                if let Some(mut region) = Region::find(world.dir().join("world"), dimension, [rx, rz]).await? {
                    for (_, cx, cy, cz) in chunks {
                        let cx_relative = cx.rem_euclid(32) as u8;
                        let cz_relative = cz.rem_euclid(32) as u8;
                        let new_timestamp = region.timestamps[32 * cz_relative as usize + cx_relative as usize];
                        match chunk_cache.entry((cx_relative, cy, cz_relative)) {
                            hash_map::Entry::Occupied(mut entry) => {
                                let (subscriptions, old_timestamp) = entry.get_mut();
                                subscriptions.add(reason);
                                if old_timestamp.is_none_or(|old_timestamp| new_timestamp != old_timestamp) {
                                    *old_timestamp = Some(new_timestamp);
                                    let col = region.chunk_column_relative([cx_relative, cz_relative])?;
                                    if subscriptions.block_states {
                                        let new_chunk = col.as_ref().and_then(|col| col.section_at(cy));
                                        version.write_chunk(sink, dimension, cx, cy, cz, new_chunk).await?;
                                    }
                                    if subscriptions.block_entities {
                                        version.write_block_entities(sink, dimension, cx, cz, col.map(|col| col.block_entities).unwrap_or_default()).await?;
                                    }
                                }
                            }
                            hash_map::Entry::Vacant(entry) => {
                                let mut subscriptions = Subscriptions::default();
                                subscriptions.add(reason);
                                entry.insert((subscriptions, Some(new_timestamp)));
                                let col = region.chunk_column_relative([cx_relative, cz_relative])?;
                                if subscriptions.block_states {
                                    let new_chunk = col.as_ref().and_then(|col| col.section_at(cy));
                                    version.write_chunk(sink, dimension, cx, cy, cz, new_chunk).await?;
                                }
                                if subscriptions.block_entities {
                                    version.write_block_entities(sink, dimension, cx, cz, col.map(|col| col.block_entities).unwrap_or_default()).await?;
                                }
                            }
                        }
                    }
                } else {
                    for (_, cx, cy, cz) in chunks {
                        let cx_relative = cx.rem_euclid(32) as u8;
                        let cz_relative = cz.rem_euclid(32) as u8;
                        match chunk_cache.entry((cx_relative, cy, cz_relative)) {
                            hash_map::Entry::Occupied(mut entry) => {
                                let (subscriptions, old_timestamp) = entry.get_mut();
                                subscriptions.add(reason);
                                if old_timestamp.is_some() {
                                    *old_timestamp = None;
                                    if subscriptions.block_states {
                                        version.write_chunk(sink, dimension, cx, cy, cz, None).await?;
                                    }
                                    if subscriptions.block_entities {
                                        version.write_block_entities(sink, dimension, cx, cz, Vec::default()).await?;
                                    }
                                }
                            }
                            hash_map::Entry::Vacant(entry) => {
                                let mut subscriptions = Subscriptions::default();
                                subscriptions.add(reason);
                                entry.insert((subscriptions, None));
                                if subscriptions.block_states {
                                    version.write_chunk(sink, dimension, cx, cy, cz, None).await?;
                                }
                                if subscriptions.block_entities {
                                    version.write_block_entities(sink, dimension, cx, cz, Vec::default()).await?;
                                }
                            }
                        }
                    }
                }
            }
        });
        Ok(())
    }

    async fn update_player(version: ActiveVersion, world: &systemd_minecraft::World, players_cache: &Mutex<HashMap<Uuid, Option<nbt::Blob>>>, watcher: &Mutex<notify::RecommendedWatcher>, sink: &WsSink, id: user::Id, uuid: Uuid, reason: PlayerUpdateReason) -> Result<(), WsError> {
        lock!(players_cache = players_cache; {
            let player_cache = match players_cache.entry(uuid) {
                hash_map::Entry::Occupied(entry) => entry.into_mut(),
                hash_map::Entry::Vacant(entry) => {
                    lock!(watcher = watcher; watcher.watch(&world.dir().join("world").join("playerdata").join(format!("{uuid}.dat")), notify::RecursiveMode::NonRecursive))?;
                    entry.insert(None)
                }
            };
            let should_check = match reason {
                PlayerUpdateReason::Subscribe => player_cache.is_none(),
                PlayerUpdateReason::Notify => true, // already filtered
            };
            if should_check {
                let path = world.dir().join("world").join("playerdata").join(format!("{uuid}.dat"));
                let mut file = match File::open(&path).await {
                    Ok(file) => Some(file),
                    Err(wheel::Error::Io { inner, .. }) if inner.kind() == io::ErrorKind::NotFound => None,
                    Err(e) => return Err(e.into()),
                };
                if let Some(mut file) = file {
                    let mut buf = Vec::default();
                    file.read_to_end(&mut buf).await.at(&path)?;
                    let mut data = nbt::Blob::from_gzip_reader(&mut &*buf)?;
                    if player_cache.as_ref().is_none_or(|player_cache| *player_cache != data) {
                        version.write_player(sink, id, uuid, Some(data.clone())).await?;
                        *player_cache = Some(data);
                    }
                } else {
                    if player_cache.is_some() {
                        version.write_player(sink, id, uuid, None).await?;
                        *player_cache = None;
                    }
                }
            }
        });
        Ok(())
    }

    let main_world = systemd_minecraft::World::default();
    let region_cache = Mutex::default();
    let players_cache = Mutex::default();
    let (watch_tx, mut watch_rx) = mpsc::channel(1_024);
    let watcher = Mutex::new(notify::recommended_watcher(move |res| watch_tx.blocking_send(res).allow_unreceived())?);
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
                    ClientMessage::SubscribeToChunk { dimension, cx, cy, cz } => update_chunks(version, &main_world, &region_cache, &watcher, &sink, iter::once((dimension, cx, cy, cz)), ChunkUpdateReason::SubscribeBlockStates).await?,
                    ClientMessage::SubscribeToChunks(chunks) => update_chunks(version, &main_world, &region_cache, &watcher, &sink, chunks, ChunkUpdateReason::SubscribeBlockStates).await?,
                    ClientMessage::SubscribeToInventory { player } => if let Some(user) = User::from_id_request(&db_pool, player.clone()).await? {
                        if let Some(uuid) = user.minecraft_uuid() {
                            update_player(version, &main_world, &players_cache, &watcher, &sink, user.id, uuid, PlayerUpdateReason::Subscribe).await?;
                        } else {
                            version.write_custom_error(&sink, player, "the requested user does not have a Minecraft UUID").await?;
                        }
                    } else {
                        version.write_custom_error(&sink, player, "the requested user ID does not exist").await?;
                    },
                    ClientMessage::SubscribeToBlockEntities { dimension, cx, cz } => update_chunks(version, &main_world, &region_cache, &watcher, &sink, iter::once((dimension, cx, 0, cz)), ChunkUpdateReason::SubscribeBlockEntities).await?,
                }
            }
            Some(res) = watch_rx.recv() => {
                let mut paths = HashSet::new();
                let event = res?;
                if event.kind.is_modify() {
                    paths.extend(event.paths);
                }
                while let Ok(res) = watch_rx.try_recv() {
                    let event = res?;
                    if event.kind.is_modify() {
                        paths.extend(event.paths);
                    }
                }
                for path in paths {
                    if let Ok(suffix) = path.strip_prefix(main_world.dir().join("world").join("playerdata")) {
                        let Ok(std::path::Component::Normal(name)) = suffix.components().exactly_one() else { return Err(WsError::NotifyUnexpectedFile) };
                        let uuid = name.to_str().ok_or(WsError::NotifyUnexpectedFile)?.strip_suffix(".dat").ok_or(WsError::NotifyUnexpectedFile)?.parse()?;
                        if let Some(user) = User::from_minecraft_uuid(&db_pool, uuid).await? {
                            update_player(version, &main_world, &players_cache, &watcher, &sink, user.id, uuid, PlayerUpdateReason::Notify).await?;
                        }
                    } else {
                        let region = Region::open(path).await?;
                        if let Some(chunks) = lock!(region_cache = region_cache; region_cache.get(&(region.dimension, region.coords[0], region.coords[1])).map(|chunks| chunks.keys().map(|&(cx, cy, cz)| (region.dimension, region.coords[0] * 32 + i32::from(cx), cy, region.coords[1] * 32 + i32::from(cz))).collect_vec())) {
                            update_chunks(version, &main_world, &region_cache, &watcher, &sink, chunks, ChunkUpdateReason::Notify).await?;
                        }
                    }
                }
            },
        }
    }
}

#[rocket::get("/api/<version>/websocket")]
pub(crate) fn websocket(db_pool: &State<PgPool>, me: Option<User>, uri: Origin<'_>, ws: request::Outcome<WebSocket, Never>, shutdown: rocket::Shutdown, version: Version) -> Result<Either<rocket_ws::Channel<'static>, (Status, RawHtml<String>)>, Status> {
    let version = ActiveVersion::try_from(version)?;
    let db_pool = (**db_pool).clone();
    Ok(match ws {
        Outcome::Success(ws) => Either::Left(ws.channel(move |stream| Box::pin(async move {
            let (ws_sink, ws_stream) = stream.split();
            let ws_sink = WsSink::new(Mutex::new(ws_sink));
            let ping_sink = ws_sink.clone();
            let ping_loop = match version {
                ActiveVersion::V3 => tokio::spawn(async move {
                    loop {
                        sleep(Duration::from_secs(30)).await;
                        if lock!(ping_sink = ping_sink; ServerMessageV3::Ping.write_ws021(&mut *ping_sink).await).is_err() { break } //TODO better error handling
                    }
                }),
                ActiveVersion::V4 => tokio::spawn(async move {
                    loop {
                        sleep(Duration::from_secs(30)).await;
                        if lock!(ping_sink = ping_sink; ServerMessageV4::Ping.write_ws021(&mut *ping_sink).await).is_err() { break } //TODO better error handling
                    }
                }),
            };
            if let Err(e) = client_session(db_pool, shutdown, version, ws_stream, ws_sink.clone()).await {
                let _ = lock!(ws_sink = ws_sink; match version {
                    ActiveVersion::V3 => ServerMessageV3::Error {
                        debug: format!("{e:?}"),
                        display: e.to_string(),
                    }.write_ws021(&mut *ws_sink).await,
                    ActiveVersion::V4 => ServerMessageV4::Error {
                        debug: format!("{e:?}"),
                        display: e.to_string(),
                    }.write_ws021(&mut *ws_sink).await,
                });
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
    })
}
