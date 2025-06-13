use {
    std::borrow::Cow,
    chrono::{
        Duration,
        prelude::*,
    },
    serde::Deserialize,
    serenity::all::{
        Colour,
        Context,
        CreateEmbed,
        CreateMessage,
        MessageBuilder,
    },
    serenity_utils::RwFuture,
    sqlx::{
        PgPool,
        types::Json,
    },
    tokio::time::sleep,
    crate::{
        discord::{
            self,
            GENERAL,
            MessageBuilderExt as _,
        },
        Error,
        lang::join_opt,
        user::{
            self,
            User,
        },
    },
};

#[derive(Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(crate) enum EventKind {
    Minigame {
        minigame: String,
    },
    #[serde(rename_all = "camelCase")]
    Renascence {
        settlement: String,
        hub_coords: [i16; 2],
    },
    RenascenceDragonFight {
        settlement: String,
    },
    Tour {
        area: Option<String>,
        guests: Vec<user::Id>,
    },
    Usc {
        season: usize,
    },
    Other {
        title: String,
        location: Option<String>,
    },
}

#[derive(Clone)]
pub(crate) struct Event {
    pub(crate) id: i32,
    pub(crate) start_time: DateTime<Utc>,
    pub(crate) end_time: DateTime<Utc>,
    pub(crate) kind: Json<EventKind>,
}

impl Event {
    pub(crate) async fn title(&self, pool: &PgPool) -> sqlx::Result<String> {
        Ok(match self.kind.0 {
            EventKind::Minigame { ref minigame } => format!("Minigame Night: {}", minigame),
            EventKind::Renascence { ref settlement, .. } => format!("Renascence: {}", settlement),
            EventKind::RenascenceDragonFight { ref settlement } => format!("{} dragon fight", settlement),
            EventKind::Tour { ref guests, ref area } => {
                let mut guest_names = Vec::default();
                for guest in guests {
                    guest_names.push(User::from_id(pool, guest.clone()).await?);
                }
                if let Some(area) = area {
                    format!("tour of {} for {}", area, join_opt(guest_names).unwrap_or_else(|| format!("no one")))
                } else {
                    format!("server tour for {}", join_opt(guest_names).unwrap_or_else(|| format!("no one")))
                }
            }
            EventKind::Usc { season } => format!("Ultra Softcore season {}", season),
            EventKind::Other { ref title, .. } => title.to_owned(),
        })
    }

    pub(crate) async fn title_discord(&self, pool: &PgPool, builder: &mut MessageBuilder) -> sqlx::Result<()> {
        match self.kind.0 {
            EventKind::Minigame { ref minigame } => {
                builder.push("Minigame Night: ");
                builder.push_safe(minigame);
            }
            EventKind::Renascence { ref settlement, .. } => {
                builder.push("Renascence: ");
                builder.push_safe(settlement);
            }
            EventKind::RenascenceDragonFight { ref settlement } => {
                builder.push_safe(settlement);
                builder.push(" dragon fight");
            }
            EventKind::Tour { ref guests, ref area } => {
                if let Some(area) = area {
                    builder.push("tour of ");
                    builder.push_safe(area);
                } else {
                    builder.push("server tour");
                }
                builder.push(" for");
                if guests.is_empty() {
                    builder.push(" no one");
                } else {
                    for guest in guests {
                        builder.push(' ');
                        builder.mention_user(&User::from_id(pool, guest.clone()).await?);
                    }
                }
            }
            EventKind::Usc { season } => {
                builder.push("Ultra Softcore season ");
                builder.push(season.to_string());
            }
            EventKind::Other { ref title, .. } => {
                builder.push(title);
            }
        }
        Ok(())
    }

    pub(crate) fn ics_location(&self) -> Option<Cow<'static, str>> {
        match self.kind.0 {
            EventKind::Minigame { .. } => Some(Cow::Borrowed("minigame.wurstmineberg.de")),
            EventKind::Renascence { hub_coords: [x, z], .. } => Some(Cow::Owned(format!("Hub {}, {}\nThe Nether\nWurstmineberg", x, z))),
            EventKind::RenascenceDragonFight { ref settlement } => Some(Cow::Owned(format!("{}\nWurstmineberg", settlement))),
            EventKind::Tour { area: Some(ref area), .. } => Some(Cow::Owned(format!("{}\nWurstmineberg", area))),
            EventKind::Tour { area: None, .. } => Some(Cow::Borrowed(if self.start_time >= Utc.with_ymd_and_hms(2019, 4, 7, 0, 0, 0).single().expect("invalid UTC datetime") {
                "spawn platform\nZucchini\nWurstmineberg"
            } else {
                "Platz des Ursprungs\nWurstmineberg"
            })),
            EventKind::Usc { .. } => Some(Cow::Borrowed("usc.wurstmineberg.de")),
            EventKind::Other { location: Some(ref loc), .. } => Some(Cow::Owned(loc.to_owned())),
            EventKind::Other { location: None, .. } => None,
        }
    }

    fn discord_location(&self) -> Option<Cow<'static, str>> {
        match self.kind.0 {
            EventKind::Minigame { .. } => Some(Cow::Borrowed("minigame.wurstmineberg.de")),
            EventKind::Renascence { hub_coords: [x, z], .. } => Some(Cow::Owned(format!("[Hub](https://wurstmineberg.de/wiki/nether-hub-system) {}, {}\nThe Nether\nWurstmineberg", x, z))),
            EventKind::RenascenceDragonFight { ref settlement } => Some(Cow::Owned(format!("[{}](https://wurstmineberg.de/renascence#{})\nWurstmineberg", settlement, settlement.to_lowercase()))),
            EventKind::Tour { area: Some(ref area), .. } => Some(Cow::Owned(format!("{}\nWurstmineberg", area))),
            EventKind::Tour { area: None, .. } => Some(Cow::Borrowed(if self.start_time >= Utc.with_ymd_and_hms(2019, 4, 7, 0, 0, 0).single().expect("invalid UTC datetime") {
                "spawn platform\n[Zucchini](https://wurstmineberg.de/wiki/renascence#zucchini)\nWurstmineberg"
            } else {
                "[Platz des Ursprungs](https://wurstmineberg.de/wiki/old-spawn#platz-des-ursprungs)\nWurstmineberg"
            })),
            EventKind::Usc { .. } => Some(Cow::Borrowed("usc.wurstmineberg.de")), //TODO linkify via menu bar/systray app?
            EventKind::Other { location: Some(ref loc), .. } => Some(Cow::Owned(loc.to_owned())),
            EventKind::Other { location: None, .. } => None,
        }
    }
}

pub(crate) async fn notifications(ctx_fut: RwFuture<Context>) -> Result<(), Error> {
    let ctx = ctx_fut.read().await;
    let mut unnotified = {
        let data = (*ctx).data.read().await;
        let pool = data.get::<discord::DbPool>().expect("missing database connection");
        let now = Utc::now();
        sqlx::query_as!(Event, r#"SELECT id, start_time AS "start_time: DateTime<Utc>", end_time AS "end_time: DateTime<Utc>", kind as "kind: Json<EventKind>" FROM calendar WHERE start_time > $1 ORDER BY start_time"#, (now + Duration::minutes(30)) as _).fetch_all(pool).await?
    };
    while !unnotified.is_empty() {
        let event = unnotified.remove(0);
        if let Ok(duration) = (event.start_time - Duration::minutes(30) - Utc::now()).to_std() {
            sleep(duration).await;
        }
        let title = {
            let mut builder = MessageBuilder::default();
            let data = (*ctx).data.read().await;
            let pool = data.get::<discord::DbPool>().expect("missing database connection");
            event.title_discord(pool, &mut builder).await?;
            builder.build()
        };
        GENERAL.send_message(&*ctx, CreateMessage::new()
            .content(format!("event starting <t:{}:R>", event.start_time.timestamp()))
            .add_embed({
                let mut e = CreateEmbed::new()
                    .colour(Colour(8794372))
                    .title(title);
                if let Some(loc) = event.discord_location() {
                    e = e.description(loc);
                }
                e.field("starts", format!("<t:{}:F>", event.start_time.timestamp()), false)
                    .field("ends", format!("<t:{}:F>", event.end_time.timestamp()), false)
            })
        ).await?;
    }
    Ok(())
}
