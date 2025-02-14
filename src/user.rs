use {
    std::{
        borrow::Cow,
        fmt,
    },
    rocket::response::content::RawHtml,
    rocket_util::{
        ToHtml,
        html,
    },
    serde::Deserialize,
    serenity::model::prelude::*,
    sqlx::{
        PgExecutor,
        types::Json,
    },
    url::Url,
    crate::{
        asset,
        discord::PgSnowflake,
    },
};

#[derive(Debug)]
pub(crate) struct User {
    id: Id,
    data: Data,
    discorddata: Option<DiscordData>,
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
                : self;
            }
        }
    }
}

#[derive(Debug)]
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

    fn url_part(&self) -> Cow<'_, str> {
        match self {
            Self::Wmbid(wmbid) => Cow::Borrowed(wmbid),
            Self::Discord(discord_id) | Self::Both { discord_id, .. } => Cow::Owned(discord_id.to_string()),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct Data {
    name: Option<String>,
    #[serde(default)]
    minecraft: DataMinecraft,
}

#[derive(Debug, Default, Deserialize)]
struct DataMinecraft {
    #[serde(default)]
    nicks: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct DiscordData {
    avatar: Option<Url>,
    nick: Option<String>,
    username: String,
}
