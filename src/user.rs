use {
    std::{
        borrow::Cow,
        cmp::Reverse,
        collections::BTreeMap,
        fmt,
        num::NonZero,
    },
    chrono_tz::{
        Etc,
        Europe,
        Tz,
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
    crate::{
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
            Tab,
            asset,
            page,
        },
    },
};

#[derive(Debug, Clone)]
pub(crate) struct User {
    pub(crate) id: Id,
    data: Data,
    pub(crate) discorddata: Option<DiscordData>,
}

impl User {
    pub(crate) async fn from_api_key(pool: impl PgExecutor<'_>, api_key: &str) -> sqlx::Result<Option<Self>> {
        Ok(
            sqlx::query!(r#"SELECT wmbid, snowflake AS "snowflake: PgSnowflake<UserId>", data AS "data: Json<Data>", discorddata AS "discorddata: Json<DiscordData>" FROM people WHERE apikey = $1"#, api_key).fetch_optional(pool).await?
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

    pub(crate) async fn from_wmbid(pool: impl PgExecutor<'_>, wmbid: impl Into<Cow<'_, str>>) -> sqlx::Result<Option<Self>> {
        let wmbid = wmbid.into();
        Ok(
            sqlx::query!(r#"SELECT snowflake AS "snowflake: PgSnowflake<UserId>", data AS "data: Json<Data>", discorddata AS "discorddata: Json<DiscordData>" FROM people WHERE wmbid = $1"#, &wmbid).fetch_optional(pool).await?
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

    pub(crate) async fn from_discord(pool: impl PgExecutor<'_>, discord_id: UserId) -> sqlx::Result<Option<Self>> {
        Ok(
            sqlx::query!(r#"SELECT wmbid, data AS "data: Json<Data>", discorddata AS "discorddata: Json<DiscordData>" FROM people WHERE snowflake = $1"#, PgSnowflake(discord_id) as _).fetch_optional(pool).await?
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

    pub(crate) async fn from_id(pool: impl PgExecutor<'_>, id: Id) -> sqlx::Result<Self> {
        Ok(match id {
            Id::Discord(discord_id) | Id::Both { discord_id, .. } => Self::from_discord(pool, discord_id).await?,
            Id::Wmbid(wmbid) => Self::from_wmbid(pool, wmbid).await?,
        }.expect("invalid user ID"))
    }

    pub(crate) async fn from_discord_or_wmbid(pool: impl PgExecutor<'_>, id: impl Into<Cow<'_, str>>) -> sqlx::Result<Option<Self>> {
        let id = id.into();
        if let Ok(discord_id) = id.parse() {
            Self::from_discord(pool, discord_id).await
        } else {
            Self::from_wmbid(pool, id).await
        }
    }

    pub(crate) async fn from_tag(pool: impl PgExecutor<'_>, username: &str, discriminator: Option<NonZero<u16>>) -> sqlx::Result<Option<Self>> {
        Ok(if let Some(discriminator) = discriminator {
            sqlx::query!(r#"SELECT wmbid, snowflake AS "snowflake!: PgSnowflake<UserId>", data AS "data: Json<Data>", discorddata AS "discorddata!: Json<DiscordData>" FROM people WHERE discorddata -> 'username' = $1 AND discorddata -> 'discriminator' = $2"#, Json(username) as _, Json(discriminator) as _).fetch_optional(pool).await?
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
            sqlx::query!(r#"SELECT wmbid, snowflake AS "snowflake!: PgSnowflake<UserId>", data AS "data: Json<Data>", discorddata AS "discorddata!: Json<DiscordData>" FROM people WHERE discorddata -> 'username' = $1 AND discorddata -> 'discriminator' = JSONB 'null'"#, Json(username) as _).fetch_optional(pool).await?
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

    pub(crate) async fn from_minecraft_uuid(pool: impl PgExecutor<'_>, uuid: Uuid) -> sqlx::Result<Option<Self>> {
        Ok(
            sqlx::query!(r#"SELECT wmbid, snowflake AS "snowflake: PgSnowflake<UserId>", data AS "data!: Json<Data>", discorddata AS "discorddata: Json<DiscordData>" FROM people WHERE data -> 'minecraft' -> 'uuid' = $1"#, Json(uuid) as _).fetch_optional(pool).await?
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
}

/// Workaround for `FromParam` not being `async`.
pub(crate) struct UserParam<'r>(&'r str);

impl<'r> UserParam<'r> {
    pub(crate) async fn parse(self, pool: impl PgExecutor<'_>) -> sqlx::Result<Option<User>> {
        User::from_discord_or_wmbid(pool, self.0).await
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
        } else if let Some(nick) = self.data.minecraft.nicks.first() {
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

#[derive(Serialize)]
#[serde(untagged)]
enum SerializeId {
    Wmbid(String),
    Discord(UserId),
}

impl From<Id> for SerializeId {
    fn from(value: Id) -> Self {
        match value {
            Id::Wmbid(wmbid) => Self::Wmbid(wmbid),
            Id::Discord(discord_id) | Id::Both { discord_id, .. } => Self::Discord(discord_id),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged, into = "SerializeId")]
pub(crate) enum Id {
    Wmbid(String),
    Discord(UserId),
    Both {
        wmbid: String,
        discord_id: UserId,
    },
}

impl Id {
    fn wmbid(&self) -> Option<&str> {
        match self {
            Self::Discord(_) => None,
            Self::Wmbid(wmbid) | Self::Both { wmbid, .. } => Some(&wmbid),
        }
    }

    fn discord_id(&self) -> Option<UserId> {
        match self {
            Self::Wmbid(_) => None,
            Self::Discord(discord_id) | Self::Both { discord_id, .. } => Some(*discord_id),
        }
    }

    pub(crate) fn url_part(&self) -> Cow<'_, str> {
        match self {
            Self::Wmbid(wmbid) => Cow::Borrowed(wmbid),
            Self::Discord(discord_id) | Self::Both { discord_id, .. } => Cow::Owned(discord_id.to_string()),
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Data {
    #[serde(skip_serializing_if = "Option::is_none")]
    base: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fav_color: Option<Color>,
    #[serde(default, skip_serializing_if = "DataMinecraft::is_default")]
    minecraft: DataMinecraft,
    #[serde(skip_serializing_if = "Option::is_none")]
    mojira: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    options: BTreeMap<String, bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    slack: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status_history: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    twitch: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timezone: Option<Tz>,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    twitter: serde_json::Map<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    website: Option<Url>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wiki: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Color {
    red: u8,
    green: u8,
    blue: u8,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DataMinecraft {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    nicks: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    uuid: Option<Uuid>,
}

impl DataMinecraft {
    fn is_default(&self) -> bool {
        let Self { nicks, uuid } = self;
        nicks.is_empty() && uuid.is_none()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct DiscordData {
    avatar: Option<Url>,
    nick: Option<String>,
    roles: Vec<RoleId>,
    pub(crate) username: String,
    pub(crate) discriminator: Option<NonZero<u16>>,
}

#[derive(Debug, thiserror::Error, rocket_util::Error)]
pub(crate) enum Error {
    #[error(transparent)] Serenity(#[from] serenity::Error),
    #[error(transparent)] Sql(#[from] sqlx::Error),
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
            Self::Values(user) => user.data.twitter.get("username").and_then(|username| username.as_str()),
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
            if value.twitter.is_empty() {
                me.data.twitter = serde_json::Map::default();
            } else {
                me.data.twitter.insert(format!("username"), serde_json::Value::String(value.twitter.clone()));
            }
            me.data.website = (!value.website.is_empty()).then(|| value.website.parse().expect("validated"));
            me.data.fav_color = fav_color;
            match me.id {
                Id::Both { discord_id, .. } | Id::Discord(discord_id) => sqlx::query!("UPDATE people SET data = $1 WHERE snowflake = $2", Json(&me.data) as _, PgSnowflake(discord_id) as _),
                Id::Wmbid(ref wmbid) => sqlx::query!("UPDATE people SET data = $1 WHERE wmbid = $2", Json(&me.data) as _, wmbid),
            }.execute(&**db_pool).await?;
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
