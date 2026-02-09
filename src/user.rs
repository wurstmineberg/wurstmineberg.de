use {
    std::{
        borrow::Cow,
        cmp::Reverse,
        collections::{
            BTreeMap,
            HashMap,
        },
        fmt,
        num::NonZero,
    },
    chrono::prelude::*,
    chrono_tz::{
        Etc,
        Europe,
        Tz,
    },
    futures::stream::{
        Stream,
        TryStreamExt as _,
    },
    lazy_regex::{
        regex_captures,
        regex_is_match,
    },
    rocket::{
        FromForm,
        State,
        form::{
            self,
            Context,
            Contextual,
            Form,
        },
        http::uri::{
            self,
            fmt::FromUriParam,
        },
        request::FromParam,
        response::content::RawHtml,
        uri,
    },
    rocket_csrf::CsrfToken,
    rocket_util::{
        ContextualExt as _,
        CsrfForm,
        Origin,
        ToHtml,
        html,
    },
    serde::{
        Deserialize,
        Serialize,
    },
    serenity::{
        all::{
            Context as DiscordCtx,
            EditMember,
        },
        model::prelude::*,
    },
    serenity_utils::RwFuture,
    sqlx::{
        PgExecutor,
        PgPool,
        types::Json,
    },
    url::Url,
    uuid::Uuid,
    wurstmineberg_web::websocket::UserIdRequest,
    crate::{
        api::{
            self,
            Version,
        },
        discord::{
            self,
            PgSnowflake,
        },
        form::{
            form_checkbox,
            form_field,
            full_form,
        },
        http::{
            PageStyle,
            Script,
            Tab,
            asset,
            page,
        },
        time::{
            DateWithOptionalTime,
            format_date,
            format_date_naive,
        },
        wiki::render_markdown,
    },
};

#[derive(Debug, Clone)]
pub(crate) struct User {
    pub(crate) id: Id,
    pub(crate) data: Data,
    pub(crate) discorddata: Option<DiscordData>,
}

impl User {
    pub(crate) fn all<'a>(db_pool: impl PgExecutor<'a> + 'a) -> impl Stream<Item = sqlx::Result<Self>> {
        sqlx::query!(r#"SELECT wmbid, snowflake AS "snowflake: PgSnowflake<UserId>", data AS "data: Json<Data>", discorddata AS "discorddata: Json<DiscordData>" FROM people"#)
            .fetch(db_pool)
            .map_ok(|row| Self {
                id: match (row.wmbid, row.snowflake) {
                    (None, None) => unreachable!("person in database with no Wurstmineberg ID and no Discord snowflake"),
                    (None, Some(PgSnowflake(discord_id))) => Id::Discord(discord_id),
                    (Some(wmbid), None) => Id::Wmbid(wmbid),
                    (Some(wmbid), Some(PgSnowflake(discord_id))) => Id::Both { wmbid, discord_id },
                },
                data: row.data.map(|Json(data)| data).unwrap_or_default(),
                discorddata: row.discorddata.map(|Json(discorddata)| discorddata),
            })
    }

    pub(crate) async fn from_api_key(db_pool: impl PgExecutor<'_>, api_key: &str) -> sqlx::Result<Option<Self>> {
        Ok(
            sqlx::query!(r#"SELECT wmbid, snowflake AS "snowflake: PgSnowflake<UserId>", data AS "data: Json<Data>", discorddata AS "discorddata: Json<DiscordData>" FROM people WHERE apikey = $1"#, api_key).fetch_optional(db_pool).await?
            .map(|row| Self {
                id: match (row.wmbid, row.snowflake) {
                    (None, None) => unreachable!("person in database with no Wurstmineberg ID and no Discord snowflake"),
                    (None, Some(PgSnowflake(discord_id))) => Id::Discord(discord_id),
                    (Some(wmbid), None) => Id::Wmbid(wmbid),
                    (Some(wmbid), Some(PgSnowflake(discord_id))) => Id::Both { wmbid, discord_id },
                },
                data: row.data.map(|Json(data)| data).unwrap_or_default(),
                discorddata: row.discorddata.map(|Json(discorddata)| discorddata),
            })
        )
    }

    pub(crate) async fn from_wmbid(db_pool: impl PgExecutor<'_>, wmbid: impl Into<Cow<'_, str>>) -> sqlx::Result<Option<Self>> {
        let wmbid = wmbid.into();
        Ok(
            sqlx::query!(r#"SELECT snowflake AS "snowflake: PgSnowflake<UserId>", data AS "data: Json<Data>", discorddata AS "discorddata: Json<DiscordData>" FROM people WHERE wmbid = $1"#, &wmbid).fetch_optional(db_pool).await?
            .map(|row| Self {
                id: if let Some(PgSnowflake(discord_id)) = row.snowflake {
                    Id::Both { wmbid: wmbid.into_owned(), discord_id }
                } else {
                    Id::Wmbid(wmbid.into_owned())
                },
                data: row.data.map(|Json(data)| data).unwrap_or_default(),
                discorddata: row.discorddata.map(|Json(discorddata)| discorddata),
            })
        )
    }

    pub(crate) async fn from_discord(db_pool: impl PgExecutor<'_>, discord_id: UserId) -> sqlx::Result<Option<Self>> {
        Ok(
            sqlx::query!(r#"SELECT wmbid, data AS "data: Json<Data>", discorddata AS "discorddata: Json<DiscordData>" FROM people WHERE snowflake = $1"#, PgSnowflake(discord_id) as _).fetch_optional(db_pool).await?
            .map(|row| Self {
                id: if let Some(wmbid) = row.wmbid {
                    Id::Both { wmbid, discord_id }
                } else {
                    Id::Discord(discord_id)
                },
                data: row.data.map(|Json(data)| data).unwrap_or_default(),
                discorddata: row.discorddata.map(|Json(discorddata)| discorddata),
            })
        )
    }

    pub(crate) async fn from_id(db_pool: impl PgExecutor<'_>, id: Id) -> sqlx::Result<Self> {
        Ok(match id {
            Id::Discord(discord_id) | Id::Both { discord_id, .. } => Self::from_discord(db_pool, discord_id).await?,
            Id::Wmbid(wmbid) => Self::from_wmbid(db_pool, wmbid).await?,
        }.expect("invalid user ID"))
    }

    pub(crate) async fn from_id_request(db_pool: impl PgExecutor<'_>, id: UserIdRequest) -> sqlx::Result<Option<Self>> {
        match id {
            UserIdRequest::Discord(discord_id) => Self::from_discord(db_pool, discord_id).await,
            UserIdRequest::Wmbid(wmbid) => Self::from_wmbid(db_pool, wmbid).await,
        }
    }

    pub(crate) async fn from_discord_or_wmbid(db_pool: impl PgExecutor<'_>, id: impl Into<Cow<'_, str>>) -> sqlx::Result<Option<Self>> {
        let id = id.into();
        if let Ok(discord_id) = id.parse() {
            Self::from_discord(db_pool, discord_id).await
        } else {
            Self::from_wmbid(db_pool, id).await
        }
    }

    pub(crate) async fn from_tag(db_pool: impl PgExecutor<'_>, username: &str, discriminator: Option<NonZero<u16>>) -> sqlx::Result<Option<Self>> {
        Ok(if let Some(discriminator) = discriminator {
            sqlx::query!(r#"SELECT wmbid, snowflake AS "snowflake!: PgSnowflake<UserId>", data AS "data: Json<Data>", discorddata AS "discorddata!: Json<DiscordData>" FROM people WHERE discorddata -> 'username' = $1 AND discorddata -> 'discriminator' = $2"#, Json(username) as _, Json(discriminator) as _).fetch_optional(db_pool).await?
            .map(|row| Self {
                id: if let Some(wmbid) = row.wmbid {
                    Id::Both { wmbid, discord_id: row.snowflake.0 }
                } else {
                    Id::Discord(row.snowflake.0)
                },
                data: row.data.map(|Json(data)| data).unwrap_or_default(),
                discorddata: Some(row.discorddata.0),
            })
        } else {
            sqlx::query!(r#"SELECT wmbid, snowflake AS "snowflake!: PgSnowflake<UserId>", data AS "data: Json<Data>", discorddata AS "discorddata!: Json<DiscordData>" FROM people WHERE discorddata -> 'username' = $1 AND discorddata -> 'discriminator' = JSONB 'null'"#, Json(username) as _).fetch_optional(db_pool).await?
            .map(|row| Self {
                id: if let Some(wmbid) = row.wmbid {
                    Id::Both { wmbid, discord_id: row.snowflake.0 }
                } else {
                    Id::Discord(row.snowflake.0)
                },
                data: row.data.map(|Json(data)| data).unwrap_or_default(),
                discorddata: Some(row.discorddata.0),
            })
        })
    }

    pub(crate) async fn from_minecraft_uuid(db_pool: impl PgExecutor<'_>, uuid: Uuid) -> sqlx::Result<Option<Self>> {
        Ok(
            sqlx::query!(r#"SELECT wmbid, snowflake AS "snowflake: PgSnowflake<UserId>", data AS "data!: Json<Data>", discorddata AS "discorddata: Json<DiscordData>" FROM people WHERE data -> 'minecraft' -> 'uuid' = $1"#, Json(uuid) as _).fetch_optional(db_pool).await?
            .map(|row| Self {
                id: match (row.wmbid, row.snowflake) {
                    (None, None) => unreachable!("person in database with no Wurstmineberg ID and no Discord snowflake"),
                    (None, Some(PgSnowflake(discord_id))) => Id::Discord(discord_id),
                    (Some(wmbid), None) => Id::Wmbid(wmbid),
                    (Some(wmbid), Some(PgSnowflake(discord_id))) => Id::Both { wmbid, discord_id },
                },
                data: row.data.0,
                discorddata: row.discorddata.map(|Json(discorddata)| discorddata),
            })
        )
    }

    pub(crate) fn wmbid(&self) -> Option<&str> {
        self.id.wmbid()
    }

    pub(crate) fn discord_id(&self) -> Option<UserId> {
        self.id.discord_id()
    }

    pub(crate) fn profile_url(&self) -> String {
        format!("/people/{}", self.id.url_part())
    }

    pub(crate) fn html_avatar(&self, size: u16) -> RawHtml<String> {
        let (url, pixelate) = if let Some(avatar) = self.discorddata.as_ref().and_then(|discorddata| discorddata.avatar.as_ref()) {
            (avatar.to_string(), false)
        } else if !self.data.minecraft.nicks.is_empty() {
            (format!("/person/{}/skin/head.png", self.id.url_part()), true)
        } else {
            (asset("/img/grid-unknown.png"), true)
        };
        html! {
            img(class = if pixelate { "avatar nearest-neighbor" } else { "avatar" }, src = url, alt = "avatar", style = format!("width: {size}px; height: {size}px;"));
        }
    }

    pub(crate) fn minecraft_uuid(&self) -> Option<Uuid> {
        self.data.minecraft.uuid
    }

    pub(crate) async fn save_data(&self, db_pool: impl PgExecutor<'_>) -> sqlx::Result<()> {
        match self.id {
            Id::Both { discord_id, .. } | Id::Discord(discord_id) => sqlx::query!("UPDATE people SET data = $1 WHERE snowflake = $2", Json(&self.data) as _, PgSnowflake(discord_id) as _),
            Id::Wmbid(ref wmbid) => sqlx::query!("UPDATE people SET data = $1 WHERE wmbid = $2", Json(&self.data) as _, wmbid),
        }.execute(db_pool).await?;
        Ok(())
    }
}

impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for User {}

/// Workaround for `FromParam` not being `async`.
pub(crate) struct UserParam<'r>(&'r str);

impl<'r> UserParam<'r> {
    pub(crate) async fn parse(self, db_pool: impl PgExecutor<'_>) -> sqlx::Result<Option<User>> {
        User::from_discord_or_wmbid(db_pool, self.0).await
    }
}

#[derive(Debug, thiserror::Error)]
#[error("user parameter is neither a valid Discord snowflake nor a valid Wurstmineberg ID")]
pub(crate) struct UserParamError;

impl<'r> FromParam<'r> for UserParam<'r> {
    type Error = UserParamError;

    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        if param.parse::<UserId>().is_ok() || regex_is_match!("[a-z][0-9a-z]{1,15}", param) {
            Ok(Self(param))
        } else {
            Err(UserParamError)
        }
    }
}

impl<'a> FromUriParam<uri::fmt::Path, &'a Id> for UserParam<'_> {
    type Target = Cow<'a, str>;

    fn from_uri_param(param: &'a Id) -> Self::Target {
        param.url_part()
    }
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(discorddata) = &self.discorddata {
            if let Some(nick) = &discorddata.nick {
                nick.fmt(f)
            } else {
                discorddata.username.fmt(f)
            }
        } else if let Some(name) = &self.data.name {
            name.fmt(f)
        } else if let Some(wmbid) = self.wmbid() {
            wmbid.fmt(f)
        } else if let Some(nick) = self.data.minecraft.nicks.last() {
            nick.fmt(f)
        } else {
            //TODO get from Minecraft UUID
            panic!("{self:?} has no name")
        }
    }
}

impl ToHtml for User {
    fn to_html(&self) -> RawHtml<String> {
        html! {
            a(title = self.to_string(), href = self.profile_url().to_string()) {
                : "@";
                : self.to_string();
            }
        }
    }
}

pub(crate) type Id = wurstmineberg_web::websocket::UserIdResponse;

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Data {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) base: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) fav_color: Option<Color>,
    #[serde(default, skip_serializing_if = "DataMinecraft::is_default")]
    pub(crate) minecraft: DataMinecraft,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) mojira: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) options: BTreeMap<String, bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) slack: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) status_history: Vec<StatusHistoryItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) twitch: Option<DataTwitch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) timezone: Option<Tz>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) twitter: Option<DataTwitter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) website: Option<Url>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) wiki: Option<String>,
}

impl Data {
    fn join_date(&self) -> Option<DateWithOptionalTime> {
        self.status_history.iter()
            .filter(|hist| hist.status == Status::Later)
            .find_map(|hist| hist.date)
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Color {
    red: u8,
    green: u8,
    blue: u8,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DataMinecraft {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    nicks: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    uuid: Option<Uuid>,
}

impl DataMinecraft {
    pub(crate) fn is_default(&self) -> bool {
        let Self { nicks, uuid } = self;
        nicks.is_empty() && uuid.is_none()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct StatusHistoryItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    by: Option<Id>,
    #[serde(skip_serializing_if = "Option::is_none")]
    date: Option<DateWithOptionalTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    status: Status,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Status {
    Founding,
    Later,
    Former,
    Vetoed,
    Guest,
    Invited,
}

impl Status {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Founding => "founding",
            Self::Later => "later",
            Self::Former => "former",
            Self::Vetoed => "vetoed",
            Self::Guest => "guest",
            Self::Invited => "invited",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct DataTwitch {
    pub(crate) login: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct DataTwitter {
    username: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiscordData {
    pub(crate) avatar: Option<Url>,
    pub(crate) joined: DateTime<Utc>,
    pub(crate) nick: Option<String>,
    pub(crate) roles: Vec<RoleId>,
    pub(crate) username: String,
    pub(crate) discriminator: Option<NonZero<u16>>,
}

#[derive(Debug, thiserror::Error, rocket_util::Error)]
pub(crate) enum Error {
    #[error(transparent)] Serenity(#[from] serenity::Error),
    #[error(transparent)] Sql(#[from] sqlx::Error),
    #[error(transparent)] Wiki(#[from] crate::wiki::Error),
}

#[rocket::get("/people")]
pub(crate) async fn list(db_pool: &State<PgPool>, me: Option<User>, uri: Origin<'_>) -> Result<RawHtml<String>, Error> {
    async fn people_table(db_pool: &PgPool, status_groups: &HashMap<Status, Vec<User>>, id: Status, name: &str) -> Result<RawHtml<String>, Error> {
        Ok(html! {
            h2(id = id.as_str()) : name;
            @if let Some(group) = status_groups.get(&id) {
                table(class = "table table-responsive people-table") {
                    thead {
                        tr {
                            th : RawHtml("&nbsp;");
                            th : "Name";
                            th : "Info";
                        }
                    }
                    tbody {
                        @for person in group {
                            tr(id? = person.wmbid().map(|wmbid| format!("person-row-{wmbid}"))) {
                                td(class = "people-avatar") {
                                    : person.html_avatar(32);
                                }
                                td(class = "username") {
                                    : person;
                                    @if let Some(nick) = person.data.minecraft.nicks.last() {
                                        @if !nick.eq_ignore_ascii_case(&person.to_string()) {
                                            br;
                                            span(class = "muted") : nick;
                                        }
                                    } else {
                                        br;
                                        span(class = "muted") : "no Minecraft account";
                                    }
                                }
                                td(class = "description") {
                                    @if let Some(description) = &person.data.description {
                                        : render_markdown(db_pool, description).await?;
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                p : "(none currently)";
            }
        })
    }

    let mut status_groups = HashMap::<_, Vec<User>>::default();
    let mut all_people = User::all(&**db_pool);
    while let Some(person) = all_people.try_next().await? {
        if let Some(lasthistory) = person.data.status_history.last() {
            let status = match lasthistory.status {
                Status::Invited => Status::Guest,
                Status::Vetoed => Status::Former,
                status => status,
            };
            let group = status_groups.entry(status).or_default();
            let sort_date = person.data.status_history.iter().find_map(|history| history.date).map(|date| date.sort_key()).unwrap_or_else(|| Utc::now());
            let idx = group.partition_point(|iter_person| iter_person.data.status_history.iter().find_map(|history| history.date).map(|date| date.sort_key()).unwrap_or_else(|| Utc::now()) <= sort_date);
            group.insert(idx, person);
        }
    }
    Ok(page(&me, &uri, PageStyle::default(), "People — Wurstmineberg", Tab::People, html! {
        div(class = "panel panel-default") {
            div(class = "panel-heading") {
                h3(class = "panel-title") : "All the people";
            }
            div(class = "panel-body") {
                p(class = "lead") : "Here's a list of all the people who are or have been on the whitelist.";
                p : "Players are ranked chronologically by the date they were invited or whitelisted.";
                p {
                    : "To keep player info updated, we kind of rely on the players themselves, so this info may be incomplete or nonsensical. If you are on the server you can use ";
                    a(href = uri!(preferences_get(_))) : "the Preferences page";
                    : " to update some of your info.";
                }
            }
        }
        div {
            : people_table(db_pool, &status_groups, Status::Founding, "Founding members").await?;
            : people_table(db_pool, &status_groups, Status::Later, "Later members").await?;
            : people_table(db_pool, &status_groups, Status::Former, "Former members").await?;
            : people_table(db_pool, &status_groups, Status::Guest, "Invited people and guests").await?;
        }
    }))
}

#[rocket::get("/people/<user>")]
pub(crate) async fn profile(db_pool: &State<PgPool>, me: Option<User>, uri: Origin<'_>, user: UserParam<'_>) -> Result<Option<RawHtml<String>>, Error> {
    fn profile_stat_row(id: &str, title: impl ToHtml) -> RawHtml<String> {
        html! {
            tr(class = "profile-stat-row", id = format!("profile-stat-row-{id}")) {
                td : title;
                td(class = "value") : "(loading)";
            }
        }
    }

    fn profile_deathgames_stat_row(id: &str, title: impl ToHtml) -> RawHtml<String> {
        html! {
            tr(id = format!("minigames-stat-row-deathgames-{id}")) {
                td : title;
                td(class = "value") : "(loading)";
            }
        }
    }

    let Some(user) = user.parse(&**db_pool).await? else { return Ok(None) };
    Ok(Some(page(&me, &uri, PageStyle { extra_scripts: vec![
        Script::External(format!("https://raw.githubusercontent.com/alexei/sprintf.js/master/dist/sprintf.min.js")), //TODO this doesn't load properly, remove dependency or vendor
        Script::External(asset("/js/profile.js")),
    ], ..PageStyle::default() }, &format!("{user} on Wurstmineberg"), Tab::People, html! {
        div(class = "panel panel-default profile-panel") {
            div(class = "panel-heading") {
                : user.html_avatar(32);
                h3(id = "username", class = "panel-title panel-loading") {
                    : user.to_string();
                    @if let Some(nick) = user.data.minecraft.nicks.last() {
                        @if !nick.eq_ignore_ascii_case(&user.to_string()) {
                            br;
                            span(class = "muted") {
                                : "(Minecraft: ";
                                : nick;
                                : ")";
                            }
                        }
                    } else {
                        br;
                        span(class = "muted") : "(no Minecraft account)";
                    }
                    @if me.as_ref().is_some_and(|me| user == *me) {
                        span(style="float: right;") {
                            a(href = uri!(preferences_get(_))) : "Edit";
                        }
                    }
                }
            }
            div(class = "panel-body") {
                div(class = "lead") {
                    @if let Some(nick) = user.data.minecraft.nicks.last() {
                        div(id = "profile-skin") {
                            img(class = "nearest-neighbor drop-shadow", style? = user.wmbid().is_some_and(|wmbid| wmbid == "dinnerbone").then_some("transform: rotate(180deg);"), title = nick, alt = nick, src = uri!(api::player_skin_front(Version::default(), &user.id)));
                            img(class = "nearest-neighbor foreground-image", style? = user.wmbid().is_some_and(|wmbid| wmbid == "dinnerbone").then_some("transform: rotate(180deg);"), title = nick, alt = nick, src = uri!(api::player_skin_front(Version::default(), &user.id)));
                        }
                    }
                    div(id = "user-info") {
                        p(id = "user-description") {
                            @if let Some(description) = &user.data.description {
                                : render_markdown(db_pool, description).await?;
                            } else if me.as_ref().is_some_and(|me| user == *me) {
                                : "You can update your description in your ";
                                a(href = uri!(preferences_get(_))) : "preferences";
                                : ".";
                            }
                        }
                        p(id = "social-links") {
                            @if let Some(website) = &user.data.website {
                                a(class = "btn btn-link", href = website.to_string()) : "Website";
                            }
                            @if let Some(twitch) = &user.data.twitch {
                                a(class = "btn btn-link", href = format!("https://twitch.tv/{}", twitch.login)) : "Twitch";
                            } else if me.as_ref().is_some_and(|me| user == *me) {
                                a(class = "btn btn-success", href = uri!(crate::auth::twitch_login(Some(&uri)))) : "Connect Twitch Account";
                            }
                            @if let Some(twitter_username) = user.data.twitter.as_ref().map(|twitter| &twitter.username) {
                                a(class = "btn btn-link", href = format!("https://twitter.com/{twitter_username}")) : "Twitter";
                            }
                        }
                        div(class = "inventory-container") {
                            div(class = "inventory-opt-out pull-left") {
                                h2(id = "inventory") : "Inventory";
                                table(id = "main-inventory", class = "inventory-table") {
                                    tbody {
                                        tr(class = "loading") {
                                            td : "loading…";
                                        }
                                    }
                                }
                                div(style = "height: 29px;");
                                table(id = "hotbar-table", class = "inventory-table") {
                                    tbody {
                                        tr(class = "loading") {
                                            td : "loading…";
                                        }
                                    }
                                }
                            }
                            div(class = "inventory-opt-out") {
                                h2(id = "enderchest") : "Ender chest";
                                table(id = "ender-chest-table", class = "inventory-table") {
                                    tbody {
                                        tr(class = "loading") {
                                            td : "loading…";
                                        }
                                    }
                                }
                                div(style = "height: 29px;");
                                table(id = "offhand-slot-table", class = "inventory-table", style = "float: right;") {
                                    tr(class = "loading") {
                                        td : "loading…";
                                    }
                                }
                                table(id = "armor-table", class = "inventory-table") {
                                    tbody {
                                        tr(class = "loading") {
                                            td : "loading…";
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        h2 : "Statistics";
        ul(id = "pagination", class = "nav nav-tabs") {
            li {
                a(id = "tab-stats-profile", class = "tab-item", href = "#profile") : "Profile";
            }
            li {
                a(id = "tab-stats-general", class = "tab-item", href = "#general") : "General";
            }
            li {
                a(id = "tab-stats-blocks", class = "tab-item", href = "#blocks") : "Blocks";
            }
            li {
                a(id = "tab-stats-items", class = "tab-item", href = "#items") : "Items";
            }
            li {
                a(id = "tab-stats-mobs", class = "tab-item", href = "#mobs") : "Mobs";
            }
            li {
                a(id = "tab-stats-achievements", class = "tab-item", href = "#achievements") : "Achievements";
            }
            li {
                a(id = "tab-stats-minigames", class = "tab-item", href = "#minigames") : "Minigames";
            }
        }
        div(id = "stats-profile", class = "section") {
            table(id = "stats-profile-table", class = "table table-responsive stats-table") {
                thead {
                    tr {
                        th : "Info";
                        th : "Value";
                    }
                }
                tbody {
                    tr(class = "profile-stat-row") {
                        td : "Date of Whitelisting";
                        td {
                            @match user.data.join_date() {
                                Some(DateWithOptionalTime::DateTime(join_date)) => : format_date(join_date);
                                Some(DateWithOptionalTime::Date(join_date)) => : format_date_naive(join_date);
                                None => span(class = "muted") : "not yet";
                            }
                        }
                    }
                    : profile_stat_row("fav-color", "Favorite Color");
                    : profile_stat_row("fav-item", "Favorite Item");
                    : profile_stat_row("invited-by", "Invited By");
                    : profile_stat_row("last-death", "Last Death");
                    : profile_stat_row("last-seen", "Last Seen");
                    : profile_stat_row("people-invited-prefreeze", html! {
                        : "People “Invited” (pre-";
                        : crate::wiki::link(db_pool, "freeze", "wiki", "freeze").await?;
                        : ")";
                    });
                    : profile_stat_row("people-invited", "People Invited (post-freeze)");
                    : profile_stat_row("status", "Status");
                }
            }
        }
        div(id = "stats-general", class = "section hidden") {
            table(id = "stats-general-table", class = "table table-responsive stats-table") {
                thead {
                    tr {
                        th : "Stat";
                        th : "Value";
                    }
                }
                tbody {
                    tr(id = "loading-stat-general-table", class = "loading-stat") {
                        td(colspan = "2") : "Loading stat data…";
                    }
                }
            }
        }
        div(id = "stats-blocks", class = "section hidden") {
            table(id = "stats-blocks-table", class = "table table-responsive stats-table") {
                thead {
                    tr {
                        th : RawHtml("&nbsp;");
                        th : "Block";
                        th(style = "text-align: right;") : "Times Crafted";
                        th(style = "text-align: right;") : "Times Used";
                        th(style = "text-align: right;") : "Times Mined";
                        th(style = "text-align: right;") : "Times Dropped";
                        th(style = "text-align: right;") : "Times Picked Up";
                    }
                }
                tbody {
                    tr(id = "loading-stat-blocks-table", class = "loading-stat") {
                        td(colspan = "5") : "Loading stat data…";
                    }
                }
            }
        }
        div(id = "stats-items", class = "section hidden") {
            table(id = "stats-items-table", class = "table table-responsive stats-table") {
                thead {
                    tr {
                        th : RawHtml("&nbsp;");
                        th : "Item";
                        th(style = "text-align: right;") : "Times Crafted";
                        th(style = "text-align: right;") : "Times Used";
                        th(style = "text-align: right;") : "Times Depleted";
                        th(style = "text-align: right;") : "Times Dropped";
                        th(style = "text-align: right;") : "Times Picked Up";
                    }
                }
                tbody {
                    tr(id = "loading-stat-items-table", class = "loading-stat") {
                        td(colspan = "5") : "Loading stat data…";
                    }
                }
            }
        }
        div(id = "stats-mobs", class = "section hidden") {
            table(id = "stats-mobs-table", class = "table table-responsive stats-table") {
                thead {
                    tr {
                        th : "Mob";
                        th : "Killed";
                        th : "Killed By";
                    }
                }
                tbody {
                    tr(id = "loading-stat-mobs-table", class = "loading-stat") {
                        td(colspan = "3") : "Loading stat data…";
                    }
                }
            }
        }
        div(id = "stats-achievements", class = "section hidden") {
            table(id = "stats-achievements-table", class = "table table-responsive stats-table") {
                thead {
                    tr {
                        th : RawHtml("&nbsp;");
                        th : "Achievement";
                        th : "Value";
                    }
                }
                tbody {
                    tr(id = "loading-stat-achievements-table", class = "loading-stat") {
                        td(colspan = "3") : "Loading stat data…";
                    }
                }
            }
        }
        div(id = "stats-minigames", class = "section hidden") {
            h2 : "Achievement Run";
            table(id = "minigames-stats-table-achievementrun", class = "table table-responsive stats-table") {
                thead {
                    tr {
                        th : "Stat";
                        th : "Value";
                    }
                }
                tbody {
                    tr(id = "minigames-stat-row-achievementrun-place") {
                        td : "Rank";
                        td(class = "value") : "(loading)";
                    }
                }
            }
            h2 : "Death Games";
            table(id = "minigames-stats-table-deathgames", class = "table table-responsive stats-table") {
                thead {
                    tr {
                        th : "Stat";
                        th : "Value";
                    }
                }
                tbody {
                    : profile_deathgames_stat_row("kills", "Kills");
                    : profile_deathgames_stat_row("deaths", "Deaths");
                    : profile_deathgames_stat_row("diamonds", "Diamonds earned (kills minus deaths)");
                    : profile_deathgames_stat_row("attacks", "Attacks total");
                    : profile_deathgames_stat_row("attacks-success", "Successful attacks");
                    : profile_deathgames_stat_row("attacks-fail", "Failed attacks");
                    : profile_deathgames_stat_row("defense", "Defenses total");
                    : profile_deathgames_stat_row("defense-success", "Successful defenses");
                    : profile_deathgames_stat_row("defense-fail", "Failed defenses");
                }
            }
        }
    })))
}

enum PreferencesFormDefaults<'v> {
    Context(Context<'v>),
    Values(User),
}

impl<'v> PreferencesFormDefaults<'v> {
    fn errors(&self) -> Vec<&form::Error<'v>> {
        match self {
            Self::Context(ctx) => ctx.errors().collect(),
            Self::Values(_) => Vec::default(),
        }
    }

    fn name(&self) -> Option<&str> {
        match self {
            Self::Context(ctx) => ctx.field_value("name"),
            Self::Values(user) => user.data.name.as_deref().or_else(|| user.discorddata.as_ref().and_then(|discorddata| discorddata.nick.as_deref())),
        }
    }

    fn description(&self) -> Option<&str> {
        match self {
            Self::Context(ctx) => ctx.field_value("description"),
            Self::Values(user) => user.data.description.as_deref(),
        }
    }

    fn mojira(&self) -> Option<&str> {
        match self {
            Self::Context(ctx) => ctx.field_value("mojira"),
            Self::Values(user) => user.data.mojira.as_deref(),
        }
    }

    fn twitter(&self) -> Option<&str> {
        match self {
            Self::Context(ctx) => ctx.field_value("twitter"),
            Self::Values(user) => user.data.twitter.as_ref().map(|twitter| &*twitter.username),
        }
    }

    fn website(&self) -> Option<&str> {
        match self {
            Self::Context(ctx) => ctx.field_value("website"),
            Self::Values(user) => user.data.website.as_ref().map(|website| website.as_str()),
        }
    }

    fn favcolor(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::Context(ctx) => ctx.field_value("favcolor").map(Cow::Borrowed),
            Self::Values(user) => user.data.fav_color.map(|Color { red, green, blue }| Cow::Owned(format!("#{red:02x}{green:02x}{blue:02x}"))),
        }
    }

    fn option(&self, name: &str, default: bool) -> bool {
        match self {
            Self::Context(ctx) => ctx.field_value(name).map(|value| value == "yes").unwrap_or(false),
            Self::Values(user) => user.data.options.get(name).copied().unwrap_or(default),
        }
    }

    fn timezone_matches(&self, tz: Tz) -> bool {
        match self {
            Self::Context(ctx) => tz.name() == ctx.field_value("timezone").unwrap_or("Etc/UTC"),
            Self::Values(user) => tz == user.data.timezone.unwrap_or(Etc::UTC),
        }
    }
}

fn preferences_form(me: User, uri: Origin<'_>, csrf: Option<&CsrfToken>, saved: bool, tab: &str, defaults: PreferencesFormDefaults<'_>) -> RawHtml<String> {
    let (mut profile_errors, mut settings_errors) = match tab {
        "profile" => (defaults.errors(), Vec::default()),
        "settings" => (Vec::default(), defaults.errors()),
        _ => (Vec::default(), Vec::default()),
    };
    page(&Some(me.clone()), &uri, PageStyle::default(), "Preferences — Wurstmineberg", Tab::Login, html! {
        div(class = "panel panel-default") {
            div(class = "panel-heading") {
                h3(id = "heading", class = "panel-title panel-loading") : "Preferences";
            }
            div(class = "panel-body") {
                div(class = "lead") {
                    p : "Change your user preferences.";
                }
                div {
                    p : "These preferences control the display and behavior of this website, of the bot and of other Wurstmineberg sites and services.";
                }
            }
        }
        @if saved {
            div(class = "alert alert-info", role = "alert") {
                : "Successfully saved profile";
            }
        }
        div {
            div(class = "panel with-nav-tabs panel-default") {
                div(class = "panel-heading") {
                    ul(id = "pagination", class = "nav nav-tabs", rol = "tablist") {
                        li(class? = (tab == "profile").then_some("active")) {
                            a(id = "tab-profile", href = "#profile", data_toggle = "tab") : "Profile";
                        }
                        li(class? = (tab == "settings").then_some("active")) {
                            a(id = "tab-settings", href = "#settings", data_toggle = "tab") : "Settings";
                        }
                    }
                }
                div(id = "preferences-body", class = "section panel-body") {
                    div(class = "panel-content tab-content") {
                        div(class = if tab == "profile" { "tab-pane active" } else { "tab-pane" }, id = "profile") {
                            div(class = "container-fluid form-group") {
                                p(class = "col-sm-10 col-sm-offset-2 preferences-profile-lead") : "These preferences control how you are displayed to other people visiting the site.";
                            }
                            : full_form(uri!(profile_post), csrf, html! {
                                @if !me.discorddata.as_ref().is_some_and(|discorddata| discorddata.roles.contains(&discord::ADMIN)) {
                                    : form_field("name", &mut profile_errors, "Name", html! {
                                        input(class = "form-control", type = "text", name = "name", value = defaults.name());
                                    }, Some(html! {
                                        : "The name that will be used when addressing you and referring to you. If you're in our Discord server, this will be kept in sync with your display name there.";
                                    }));
                                }
                                : form_field("description", &mut profile_errors, "Description", html! {
                                    textarea(class = "form-control", name = "description", maxlength = "1000", placeholder = "A short text (up to 1000 characters) that describes you. May contain Markdown formatting.") : defaults.description();
                                    //TODO Markdown preview for description
                                }, Some(html! {
                                    : "1000 characters maximum.";
                                }));
                                : form_field("mojira", &mut profile_errors, "Mojira username", html! {
                                    input(class = "form-control", type = "text", name = "mojira", maxlength = "50", value = defaults.mojira());
                                }, Some(html! {
                                    : "Your username on the Mojira bug tracker";
                                }));
                                : form_field("twitter", &mut profile_errors, "Twitter username", html! {
                                    input(class = "form-control", type = "text", name = "twitter", maxlength = "15", value = defaults.twitter());
                                }, Some(html! {
                                    : "Your Twitter @username";
                                }));
                                : form_field("website", &mut profile_errors, "Website", html! {
                                    input(class = "form-control", type = "text", name = "website", placeholder = "https://www.example.com/", maxlength = "2000", value = defaults.website());
                                }, Some(html! {
                                    : "The URL of your website";
                                }));
                                : form_field("favcolor", &mut profile_errors, "Favorite Color", html! {
                                    span(class = "form-colorpicker input-group", data_format = "hex") {
                                        input(class = "form-control", type = "text", name = "favcolor", placeholder = "Enter a hex RGB color like #000000 or use the color picker on the right", value = defaults.favcolor());
                                        span(class = "input-group-addon") {
                                            i;
                                        }
                                    }
                                }, Some(html! {
                                    : "A color used to represent you along with your name, avatar, and Minecraft skin";
                                }));
                            }, profile_errors, "Save");
                        }
                        div(class = if tab == "settings" { "tab-pane active" } else { "tab-pane" }, id = "settings") {
                            p(class = "col-sm-10 col-sm-offset-2 preferences-profile-lead") : "These preferences change the behavior of several tools and functions like on this website, the bot etc.";
                            : full_form(uri!(settings_post), csrf, html! {
                                : form_checkbox("allow_online_notifications", &mut settings_errors, "Allow others to receive online notifications for you", defaults.option("allow_online_notifications", true), Some(html! {
                                    : "This website will soon™ have a feature where members can ask to receive notifications when players join/leave the main world. If you disable this setting, no one will receive these notifications when you join/leave.";
                                }));
                                /*
                                : form_checkbox("activity_tweets", &mut settings_errors, "Activity Tweets", defaults.option("activity_tweets", true), Some(html! {
                                    : "When this option is off, the bot will refrain from @mentioning you in achievement and death tweets (this feature is not yet implemented).";
                                }));
                                : form_checkbox("inactivity_tweets", &mut settings_errors, "Inactivity Tweets", defaults.option("inactivity_tweets", false), Some(html! {
                                    : "When this option is on, the bot will send you a tweet after a random time (between 1 and 6 months) of inactivity (this feature is not yet implemented, see here for the feature request) and on your whitelisting anniversary (not yet implemented either, see here for the feature request). When it's off, it will still tweet about your anniversary, but without @mentioning you.";
                                }));
                                */
                                : form_checkbox("public_info", &mut settings_errors, "User data is public", defaults.option("public_info", true), Some(html! {
                                    : "When this option is off, only server members logged in on the website can view your profile page and statistics. Note that your data is still publicly accessible via the API.";
                                }));
                                : form_checkbox("show_inventory", &mut settings_errors, "Show inventory", defaults.option("show_inventory", false), Some(html! {
                                    : "Whether or not your profile page should show your inventory and Ender chest content.";
                                }));
                                @let timezones = {
                                    let mut timezones = chrono_tz::TZ_VARIANTS;
                                    timezones.sort_by_key(|tz| Reverse(matches!(*tz, Etc::UTC | Europe::Berlin | Europe::Vienna)));
                                    timezones
                                };
                                : form_field("timezone", &mut settings_errors, "Time zone", html! {
                                    select(class = "form-control", name = "timezone") {
                                        @for tz in timezones {
                                            option(selected? = defaults.timezone_matches(tz), value = tz.name()) : tz.name();
                                        }
                                    }
                                }, None);
                            }, settings_errors, "Save");
                        }
                    }
                }
            }
        }
    })
}

#[rocket::get("/preferences?<tab>")]
pub(crate) fn preferences_get(me: User, uri: Origin<'_>, csrf: Option<CsrfToken>, tab: Option<&str>) -> RawHtml<String> {
    preferences_form(me.clone(), uri, csrf.as_ref(), false, tab.unwrap_or("profile"), PreferencesFormDefaults::Values(me))
}

#[derive(FromForm, CsrfForm)]
pub(crate) struct ProfileForm {
    #[field(default = String::new())]
    csrf: String,
    #[field(default = String::new())]
    name: String,
    description: String,
    mojira: String,
    twitter: String,
    website: String,
    favcolor: String,
}

#[rocket::post("/preferences?tab=profile", data = "<form>", rank = 0)]
pub(crate) async fn profile_post(db_pool: &State<PgPool>, discord_ctx: &State<RwFuture<DiscordCtx>>, mut me: User, uri: Origin<'_>, csrf: Option<CsrfToken>, form: Form<Contextual<'_, ProfileForm>>) -> Result<RawHtml<String>, Error> {
    let mut form = form.into_inner();
    form.verify(&csrf);
    Ok(if let Some(ref value) = form.value {
        if !me.discorddata.as_ref().is_some_and(|discorddata| discorddata.roles.contains(&discord::ADMIN)) && !regex_is_match!("^[^@#:]{2,32}$", &value.name) { //TODO better compliance with https://discord.com/developers/docs/resources/user
            form.context.push_error(form::Error::validation("Discord display names must be 2–32 characters and must not contain ^@#:").with_name("name"));
        }
        if value.description.len() > 1000 {
            form.context.push_error(form::Error::validation("Description must be at most 1000 characters").with_name("description"));
        }
        if value.mojira.len() > 50 {
            form.context.push_error(form::Error::validation("Not a valid Mojira username").with_name("mojira"));
        }
        if !regex_is_match!("^[A-Za-z0-9_]*$", &value.twitter) {
            form.context.push_error(form::Error::validation("Not a valid Twitter username").with_name("twitter"));
        }
        if value.website.len() > 2000 {
            form.context.push_error(form::Error::validation("Website must be at most 2000 characters").with_name("website"));
        } else if Url::parse(&value.website).is_err() {
            form.context.push_error(form::Error::validation("Not a valid URL").with_name("website"));
        }
        let fav_color = if value.favcolor.is_empty() {
            None
        } else if let Some((_, r, g, b)) = regex_captures!("^#([0-9A-Fa-f]{2})([0-9A-Fa-f]{2})([0-9A-Fa-f]{2})$", &value.favcolor) {
            Some(Color {
                red: u8::from_str_radix(r, 16).unwrap(),
                green: u8::from_str_radix(g, 16).unwrap(),
                blue: u8::from_str_radix(b, 16).unwrap(),
            })
        } else {
            form.context.push_error(form::Error::validation("Color must be in the #RRGGBB format").with_name("favcolor"));
            None
        };
        if form.context.errors().next().is_some() {
            preferences_form(me, uri, csrf.as_ref(), false, "profile", PreferencesFormDefaults::Context(form.context))
        } else {
            if let (Some(discord_id), Some(discorddata)) = (me.discord_id(), &me.discorddata) {
                if !discorddata.roles.contains(&discord::ADMIN) {
                    discord::GUILD.edit_member(&*discord_ctx.read().await, discord_id, EditMember::default().nickname(&value.name)).await?;
                }
            }
            if !me.discorddata.as_ref().is_some_and(|discorddata| discorddata.roles.contains(&discord::ADMIN)) {
                me.data.name = Some(value.name.clone());
            }
            me.data.description = (!value.description.is_empty()).then(|| value.description.clone());
            me.data.mojira = (!value.mojira.is_empty()).then(|| value.mojira.clone());
            me.data.twitter = (!value.twitter.is_empty()).then(|| DataTwitter { username: value.twitter.clone() });
            me.data.website = (!value.website.is_empty()).then(|| value.website.parse().expect("validated"));
            me.data.fav_color = fav_color;
            me.save_data(&**db_pool).await?;
            preferences_form(me.clone(), uri, csrf.as_ref(), true, "profile", PreferencesFormDefaults::Values(me))
        }
    } else {
        preferences_form(me, uri, csrf.as_ref(), false, "profile", PreferencesFormDefaults::Context(form.context))
    })
}

#[derive(FromForm, CsrfForm)]
pub(crate) struct SettingsForm {
    #[field(default = String::new())]
    csrf: String,
    allow_online_notifications: bool,
    //activity_tweets: bool,
    //inactivity_tweets: bool,
    public_info: bool,
    show_inventory: bool,
    timezone: String,
}

#[rocket::post("/preferences?tab=settings", data = "<form>", rank = 1)]
pub(crate) async fn settings_post(db_pool: &State<PgPool>, mut me: User, uri: Origin<'_>, csrf: Option<CsrfToken>, form: Form<Contextual<'_, SettingsForm>>) -> Result<RawHtml<String>, rocket_util::Error<sqlx::Error>> {
    let mut form = form.into_inner();
    form.verify(&csrf);
    Ok(if let Some(ref value) = form.value {
        if !value.timezone.is_empty() && value.timezone.parse::<Tz>().is_err() {
            form.context.push_error(form::Error::validation("Not a valid timezone").with_name("timezone"));
        }
        if form.context.errors().next().is_some() {
            preferences_form(me, uri, csrf.as_ref(), false, "settings", PreferencesFormDefaults::Context(form.context))
        } else {
            me.data.options.insert(format!("allow_online_notifications"), value.allow_online_notifications);
            //me.data.options.insert(format!("activity_tweets"), value.activity_tweets);
            //me.data.options.insert(format!("inactivity_tweets"), value.inactivity_tweets);
            me.data.options.insert(format!("public_info"), value.public_info);
            me.data.options.insert(format!("show_inventory"), value.show_inventory);
            me.data.timezone = (!value.timezone.is_empty()).then(|| value.timezone.parse().expect("validated"));
            match me.id {
                Id::Both { discord_id, .. } | Id::Discord(discord_id) => sqlx::query!("UPDATE people SET data = $1 WHERE snowflake = $2", Json(&me.data) as _, PgSnowflake(discord_id) as _),
                Id::Wmbid(ref wmbid) => sqlx::query!("UPDATE people SET data = $1 WHERE wmbid = $2", Json(&me.data) as _, wmbid),
            }.execute(&**db_pool).await?;
            preferences_form(me.clone(), uri, csrf.as_ref(), true, "settings", PreferencesFormDefaults::Values(me))
        }
    } else {
        preferences_form(me, uri, csrf.as_ref(), false, "settings", PreferencesFormDefaults::Context(form.context))
    })
}
