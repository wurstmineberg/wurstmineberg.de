use {
    std::{
        num::NonZero,
        path::Path,
        pin::Pin,
        time::Duration,
    },
    discord_message_parser::{
        MessagePart,
        TimestampStyle,
        serenity::MessageExt as _,
    },
    itertools::Itertools as _,
    minecraft::chat::Chat,
    rand::{
        prelude::*,
        rng,
    },
    serde_json::json,
    serenity::{
        all::{
            CreateCommand,
            CreateCommandOption,
            CreateInteractionResponse,
            CreateInteractionResponseMessage,
            MessageBuilder,
        },
        model::prelude::*,
        prelude::*,
    },
    serenity_utils::{
        builder::ErrorNotifier,
        handler::{
            HandlerMethods as _,
            voice_state::VoiceStates,
        },
    },
    sqlx::{
        Database,
        Decode,
        Encode,
        PgPool,
        postgres::PgConnectOptions,
    },
    tokio::{
        process::Command,
        time::{
            Instant,
            sleep,
        },
    },
    wheel::{
        fs,
        traits::AsyncCommandOutputExt as _,
    },
    crate::{
        BASE_PATH,
        cal,
        config::Config,
        user::User,
    },
};
#[cfg(not(target_os = "linux"))] use crate::systemd_minecraft;

pub(crate) const GENERAL: ChannelId = ChannelId::new(88318761228054528);
const DEV: ChannelId = ChannelId::new(506905544901001228);

pub(crate) const GUILD: GuildId = GuildId::new(88318761228054528);

pub(crate) const ADMIN: RoleId = RoleId::new(88329417788502016);

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("encountered user without join date")]
    MissingJoinDate,
}

/// A wrapper around serenity's Discord snowflake types that can be stored in a PostgreSQL database as a BIGINT.
#[derive(Debug)]
pub(crate) struct PgSnowflake<T>(pub(crate) T);

impl<'r, T: From<NonZero<u64>>, DB: Database> Decode<'r, DB> for PgSnowflake<T>
where i64: Decode<'r, DB> {
    fn decode(value: <DB as Database>::ValueRef<'r>) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let id = i64::decode(value)?;
        let id = NonZero::try_from(id as u64)?;
        Ok(Self(id.into()))
    }
}

impl<'q, T: Copy + Into<i64>, DB: Database> Encode<'q, DB> for PgSnowflake<T>
where i64: Encode<'q, DB> {
    fn encode_by_ref(&self, buf: &mut <DB as Database>::ArgumentBuffer<'q>) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        self.0.into().encode(buf)
    }

    fn encode(self, buf: &mut <DB as Database>::ArgumentBuffer<'q>) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        self.0.into().encode(buf)
    }

    fn produces(&self) -> Option<<DB as Database>::TypeInfo> {
        self.0.into().produces()
    }

    fn size_hint(&self) -> usize {
        Encode::size_hint(&self.0.into())
    }
}

impl<T, DB: Database> sqlx::Type<DB> for PgSnowflake<T>
where i64: sqlx::Type<DB> {
    fn type_info() -> <DB as Database>::TypeInfo {
        i64::type_info()
    }

    fn compatible(ty: &<DB as Database>::TypeInfo) -> bool {
        i64::compatible(ty)
    }
}

enum UserListExporter {}

impl serenity_utils::handler::user_list::ExporterMethods for UserListExporter {
    fn upsert<'a>(ctx: &'a Context, member: &'a Member) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'a>> {
        Box::pin(async move {
            let data = ctx.data.read().await;
            let pool = data.get::<DbPool>().expect("missing database connection");
            //TODO update display name in data column
            sqlx::query!("UPDATE people SET discorddata = $1 WHERE snowflake = $2", json!({
                "avatar": member.user.avatar_url(),
                "discriminator": member.user.discriminator,
                "joined": if let Some(ref join_date) = member.joined_at { join_date } else { return Err(Box::new(Error::MissingJoinDate) as Box<dyn std::error::Error + Send + Sync>) },
                "nick": &member.nick,
                "roles": &member.roles,
                "username": member.user.name,
            }), i64::from(member.user.id))
                .execute(pool).await?;
            Ok(())
        })
    }

    fn replace_all<'a>(ctx: &'a Context, members: Vec<&'a Member>) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'a>> {
        Box::pin(async move {
            for member in members {
                Self::upsert(ctx, member).await?;
            }
            Ok(())
        })
    }

    fn remove<'a>(ctx: &'a Context, user_id: UserId, _: GuildId) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'a>> {
        Box::pin(async move {
            let data = ctx.data.read().await;
            let pool = data.get::<DbPool>().expect("missing database connection");
            sqlx::query!("UPDATE people SET discorddata = NULL WHERE snowflake = $1", i64::from(user_id))
                .execute(pool).await?;
            Ok(())
        })
    }
}

enum VoiceStateExporter {}

impl serenity_utils::handler::voice_state::ExporterMethods for VoiceStateExporter {
    fn dump_info<'a>(_: &'a Context, _: GuildId, VoiceStates(voice_states): &'a VoiceStates) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'a>> {
        Box::pin(async move {
            let buf = serde_json::to_vec_pretty(&json!({
                "channels": voice_states.into_iter()
                    .map(|(channel_id, (channel_name, members))| json!({
                        "members": members.into_iter()
                            .map(|user| json!({
                                "discriminator": user.discriminator,
                                "snowflake": user.id,
                                "username": user.name,
                            }))
                            .collect_vec(),
                        "name": channel_name,
                        "snowflake": channel_id,
                    }))
                    .collect_vec()
            }))?;
            fs::write(Path::new(BASE_PATH).join("discord").join("voice-state.json"), buf).await?;
            Ok(())
        })
    }
}

fn discord_to_minecraft<'a>(ctx: &'a Context, msg: &'a Message, chat: &'a mut Chat, part: MessagePart<'a>) -> Pin<Box<dyn Future<Output = serenity::Result<()>> + Send + 'a>> {
    Box::pin(async move {
        match part {
            MessagePart::Empty => {}
            MessagePart::Nested(parts) => for part in parts {
                discord_to_minecraft(ctx, msg, chat, part).await?;
            },
            MessagePart::PlainText(text) => { chat.add_extra(text); }
            MessagePart::UserMention { user, nickname_mention: _ } => {
                let (tag, nickname) = if let Some(guild_id) = msg.guild_id {
                    let member = guild_id.member(ctx, user).await?;
                    (Some(member.user.tag()), member.nick)
                } else {
                    (None, None)
                };
                let (tag, nickname) = match (tag, nickname) {
                    (Some(tag), Some(nickname)) => (tag, nickname),
                    (tag, nickname) => {
                        let user = user.to_user(ctx).await?;
                        (tag.unwrap_or_else(|| user.tag()), nickname.unwrap_or(user.name))
                    }
                };
                let mut extra = Chat::from(format!("@{}", nickname));
                //TODO add mention to chat input on click? (blue + underline)
                extra.on_hover(minecraft::chat::HoverEvent::ShowText(Box::new(Chat::from(tag))));
                chat.add_extra(extra);
            }
            MessagePart::ChannelMention(channel) => {
                let extra = Chat::from(match channel.to_channel(ctx).await? {
                    Channel::Guild(channel) => format!("#{}", channel.name),
                    Channel::Private(dm) => dm.name(),
                    _ => panic!("unexpected channel type"),
                });
                //TODO open channel in browser on click? (blue + underline)
                chat.add_extra(extra);
            }
            MessagePart::RoleMention(role) => {
                let mut extra = Chat::from(format!("<@&{}>", role));
                if let Some(guild_id) = msg.guild_id {
                    if let Some(role) = guild_id.roles(ctx).await?.get(&role) {
                        extra = Chat::from(format!("@{}", role.name));
                        //TODO add mention to chat input on click? (blue + underline)
                    }
                }
                chat.add_extra(extra);
            }
            MessagePart::UnicodeEmoji(text) => { chat.add_extra(text); } //TODO special handling for emoji where possible
            MessagePart::CustomEmoji(emoji) => {
                chat.add_extra(format!(":{}:", emoji.name));
            }
            MessagePart::Timestamp { timestamp, style } => {
                let mut extra = Chat::from(style.unwrap_or_default().fmt(timestamp)); //TODO convert to user timezone? (Would require replacing @a with individual commands)
                extra.underlined();
                if let Some(TimestampStyle::RelativeTime) = style {
                    extra.on_hover(minecraft::chat::HoverEvent::ShowText(Box::new(Chat::from(timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string())))); //TODO show user timezone if converted
                } else {
                    extra.on_hover(minecraft::chat::HoverEvent::ShowText(Box::new(Chat::from("UTC")))); //TODO show user timzeone if converted
                }
                chat.add_extra(extra);
            }
        }
        Ok(())
    })
}

pub(crate) trait MessageBuilderExt {
    fn mention_user(&mut self, user: &User) -> &mut Self;
}

impl MessageBuilderExt for MessageBuilder {
    fn mention_user(&mut self, user: &User) -> &mut Self {
        if let Some(discord_id) = user.discord_id() {
            self.mention(&discord_id)
        } else {
            self.push_safe(user.to_string())
        }
    }
}

pub(crate) enum DbPool {}

impl TypeMapKey for DbPool {
    type Value = PgPool;
}

#[derive(Clone, Copy)]
struct CommandIds {
    //TODO `/event` any-admin command to add or edit calendar events
    iam: CommandId,
    iamn: CommandId,
    ping: CommandId,
    update: CommandId,
    veto: CommandId,
}

impl TypeMapKey for CommandIds {
    type Value = Self;
}

pub(crate) async fn configure_builder(discord_builder: serenity_utils::Builder, config: Config, shutdown: rocket::Shutdown) -> Result<serenity_utils::Builder, crate::Error> {
    discord_builder
        .error_notifier(ErrorNotifier::Channel(DEV))
        .on_ready(|ctx, ready| Box::pin(async move {
            if ready.guilds.len() > 1 {
                println!("error: multiple guilds found (wurstminebot's code currently assumes that it's only in the Wurstmineberg guild)"); //TODO return as Err?
                serenity_utils::shut_down(&ctx).await;
            }
            Ok(())
        }))
        .on_message(true, |ctx, msg| Box::pin(async move {
            if msg.author.bot { return Ok(()) } // ignore bots to prevent message loops
            if let Some((world, _)) = ctx.data.read().await.get::<Config>().expect("missing config").wurstminebot.world_channels.iter().find(|(_, chan_id)| **chan_id == msg.channel_id) {
                if world.is_running().await? {
                    let mut chat = Chat::from(format!(
                        "[Discord:#{}",
                        if let Channel::Guild(chan) = msg.channel(&ctx).await? { chan.name.clone() } else { format!("?") },
                    ));
                    chat.color(minecraft::chat::Color::Aqua);
                    if let Some(ref in_reply_to) = msg.referenced_message {
                        chat.add_extra(", replying to ");
                        chat.add_extra({
                            let mut extra = Chat::from(in_reply_to.member.as_ref().and_then(|member| member.nick.as_deref()).unwrap_or(&in_reply_to.author.name));
                            extra.on_hover(minecraft::chat::HoverEvent::ShowText(Box::new(Chat::from(in_reply_to.author.tag()))));
                            extra
                        });
                    }
                    chat.add_extra("] ");
                    chat.add_extra({
                        let mut extra = Chat::from(format!("<{}>", msg.member.as_ref().and_then(|member| member.nick.as_ref()).unwrap_or(&msg.author.name)));
                        extra.on_hover(minecraft::chat::HoverEvent::ShowText(Box::new(Chat::from(msg.author.tag()))));
                        extra
                    });
                    chat.add_extra(" ");
                    discord_to_minecraft(&ctx, &msg, &mut chat, msg.parse()).await?;
                    for attachment in &msg.attachments {
                        chat.add_extra(" ");
                        chat.add_extra({
                            let mut extra = Chat::from(format!("[{}]", attachment.filename));
                            extra.color(minecraft::chat::Color::Blue);
                            extra.underlined();
                            extra.on_click(minecraft::chat::ClickEvent::OpenUrl(attachment.url.clone()));
                            extra.on_hover(minecraft::chat::HoverEvent::ShowText(Box::new(Chat::from(&*attachment.url))));
                            extra
                        });
                    }
                    match world.tellraw("@a", &chat).await {
                        Ok(_) => {}
                        Err(systemd_minecraft::Error::Rcon(rcon::Error::CommandTooLong)) => {
                            let mut chat = Chat::from(format!(
                                "[Discord:#{}] long message from ",
                                if let Channel::Guild(chan) = msg.channel(&ctx).await? { chan.name.clone() } else { format!("?") },
                            ));
                            chat.color(minecraft::chat::Color::Aqua);
                            chat.add_extra({
                                let mut extra = Chat::from(msg.member.as_ref().and_then(|member| member.nick.as_deref()).unwrap_or(&msg.author.name));
                                extra.on_hover(minecraft::chat::HoverEvent::ShowText(Box::new(Chat::from(msg.author.tag()))));
                                extra
                            });
                            world.tellraw("@a", &chat).await?;
                        }
                        Err(e) => return Err(e.into()),
                    }
                }
            }
            Ok(())
        }))
        .on_guild_create(false, |ctx, guild, _| Box::pin(async move {
            let mut commands = Vec::default();
            let iam = {
                let idx = commands.len();
                commands.push(CreateCommand::new("iam")
                    .kind(CommandType::ChatInput)
                    .add_context(InteractionContext::Guild)
                    .description("Give yourself a self-assignable role")
                    .add_option(CreateCommandOption::new(
                        CommandOptionType::Role,
                        "role",
                        "the role to add",
                    ).required(true))
                );
                idx
            };
            let iamn = {
                let idx = commands.len();
                commands.push(CreateCommand::new("iamn")
                    .kind(CommandType::ChatInput)
                    .add_context(InteractionContext::Guild)
                    .description("Remove a self-assignable role from yourself")
                    .add_option(CreateCommandOption::new(
                        CommandOptionType::Role,
                        "role",
                        "the role to remove",
                    ).required(true))
                );
                idx
            };
            let ping = {
                let idx = commands.len();
                commands.push(CreateCommand::new("ping")
                    .kind(CommandType::ChatInput)
                    .add_context(InteractionContext::Guild)
                    .description("Test if wurstminebot is online")
                );
                idx
            };
            let update = {
                let idx = commands.len();
                commands.push(CreateCommand::new("update")
                    .kind(CommandType::ChatInput)
                    .add_context(InteractionContext::Guild)
                    .description("Update this Minecraft world")
                    .add_option(CreateCommandOption::new(
                        CommandOptionType::String,
                        "version",
                        "the version to update to, defaults to latest release",
                    ).required(false))
                );
                idx
            };
            let veto = {
                let idx = commands.len();
                commands.push(CreateCommand::new("veto")
                    .kind(CommandType::ChatInput)
                    .add_context(InteractionContext::Guild)
                    .description("Anonymously veto a Wurstmineberg invite")
                    .add_option(CreateCommandOption::new(
                        CommandOptionType::User,
                        "invitee",
                        "the invited person to veto",
                    ).required(true))
                );
                idx
            };
            let commands = guild.set_commands(ctx, commands).await?;
            ctx.data.write().await.insert::<CommandIds>(CommandIds {
                iam: commands[iam].id,
                iamn: commands[iamn].id,
                ping: commands[ping].id,
                update: commands[update].id,
                veto: commands[veto].id,
            });
            Ok(())
        }))
        .on_interaction_create(|ctx, interaction| Box::pin(async move {
            match interaction {
                Interaction::Command(interaction) => {
                    if let Some(&command_ids) = ctx.data.read().await.get::<CommandIds>() {
                        if interaction.data.id == command_ids.iam {
                            let member = interaction.member.clone().expect("/iam called outside of a guild");
                            let role_id = match interaction.data.options[0].value {
                                CommandDataOptionValue::Role(role) => role,
                                _ => panic!("unexpected slash command option type"),
                            };
                            let response = if !ctx.data.read().await.get::<Config>().expect("missing self-assignable roles list").wurstminebot.self_assignable_roles.contains(&role_id) {
                                "this role is not self-assignable" //TODO (Discord feature request) list only self-assignable roles in autocomplete
                            } else if member.roles.contains(&role_id) {
                                "you already have this role"
                            } else {
                                member.add_role(&ctx, role_id).await?;
                                "role added"
                            };
                            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new()
                                .ephemeral(true)
                                .content(response)
                            )).await?;
                        } else if interaction.data.id == command_ids.iamn {
                            let member = interaction.member.clone().expect("/iamn called outside of a guild");
                            let role_id = match interaction.data.options[0].value {
                                CommandDataOptionValue::Role(role) => role,
                                _ => panic!("unexpected slash command option type"),
                            };
                            let response = if !ctx.data.read().await.get::<Config>().expect("missing self-assignable roles list").wurstminebot.self_assignable_roles.contains(&role_id) {
                                "this role is not self-assignable" //TODO (Discord feature request) list only self-assignable roles in autocomplete
                            } else if member.roles.contains(&role_id) {
                                "you already don't have this role"
                            } else {
                                member.remove_role(&ctx, role_id).await?;
                                "role removed"
                            };
                            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new()
                                .ephemeral(true)
                                .content(response)
                            )).await?;
                        } else if interaction.data.id == command_ids.ping {
                            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new()
                                .ephemeral(true)
                                .content({
                                    let mut rng = rng();
                                    if rng.random_bool(0.01) {
                                        format!("BWO{}{}G", "R".repeat(rng.random_range(3..20)), "N".repeat(rng.random_range(1..5)))
                                    } else {
                                        format!("pong")
                                    }
                                })
                            )).await?;
                        } else if interaction.data.id == command_ids.update {
                            if let Some((world, _)) = ctx.data.read().await.get::<Config>().expect("missing config").wurstminebot.world_channels.iter().find(|(_, chan_id)| **chan_id == interaction.channel_id) {
                                let version_spec = if let Some(option) = interaction.data.options.get(0) {
                                    match &option.value {
                                        CommandDataOptionValue::String(version) => systemd_minecraft::VersionSpec::Exact(version.clone()),
                                        _ => panic!("unexpected slash command option type"),
                                    }
                                } else {
                                    systemd_minecraft::VersionSpec::LatestRelease
                                };
                                if *world == systemd_minecraft::World::default() {
                                    interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new()
                                        .ephemeral(false)
                                        .content(MessageBuilder::default().push("Backing up ").push_safe(world.to_string()).push(" world…").build())
                                    )).await?;
                                    Command::new(Path::new(BASE_PATH).join("bin").join("wurstminebackup")).check("wurstminebackup").await?;
                                    interaction.channel_id.say(ctx, MessageBuilder::default().push("Updating ").push_safe(world.to_string()).push(" world…").build()).await?;
                                } else {
                                    interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new()
                                        .ephemeral(false)
                                        .content(MessageBuilder::default().push("Updating ").push_safe(world.to_string()).push(" world…").build())
                                    )).await?;
                                }
                                let reply = match world.update(version_spec).await {
                                    Ok(()) => format!("Done!"),
                                    Err(e) => MessageBuilder::default().push("World update error: ").push_safe(e.to_string()).push(" (").push_mono_safe(format!("{:?}", e)).push(")").build(),
                                };
                                interaction.channel_id.say(ctx, reply).await?;
                            } else {
                                interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new()
                                    .ephemeral(true)
                                    .content("This channel has no associated Minecraft world.")
                                )).await?;
                            }
                        } else if interaction.data.id == command_ids.veto {
                            //TODO only allow current members to use this command
                            let user_id = match interaction.data.options[0].value {
                                CommandDataOptionValue::User(user) => user,
                                _ => panic!("unexpected slash command option type"),
                            };
                            //TODO validate veto period, kick person from guild and remove from whitelist
                            GENERAL.say(ctx, MessageBuilder::default()
                                .push("invite for ")
                                .mention(&user_id)
                                .push(" has been vetoed")
                                .build()
                            ).await?;
                            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new()
                                .ephemeral(true)
                                .content(MessageBuilder::new().push("message posted in ").mention(&GENERAL).build())
                            )).await?;
                        } else {
                            panic!("unexpected slash command")
                        }
                    }
                }
                Interaction::Component(_) => panic!("received message component interaction even though no message components are registered"),
                _ => {}
            }
            Ok(())
        }))
        .event_handler(serenity_utils::handler::user_list_exporter::<UserListExporter>())
        .event_handler(serenity_utils::handler::voice_state_exporter::<VoiceStateExporter>())
        .data::<Config>(config)
        .data::<DbPool>(PgPool::connect_with(PgConnectOptions::default().database("wurstmineberg").application_name("wurstminebot")).await?)
        .task(|ctx_fut, notify_thread_crash| async move {
            if let Err(e) = cal::notifications(ctx_fut).await {
                eprintln!("{}", e);
                notify_thread_crash(format!("calendar notifications"), Box::new(e), None).await;
            }
        })
        .task(|ctx_fut, notify_thread_crash| async move {
            // follow the Minecraft log
            let Err(e) = crate::log::handle(ctx_fut).await;
            eprintln!("{}", e);
            notify_thread_crash(format!("log"), Box::new(e), None).await;
        })
        .task(|ctx_fut, notify_thread_crash| async move {
            // listen for Twitch chat messages
            let mut last_crash = Instant::now();
            let mut wait_time = Duration::from_secs(1);
            loop {
                let e = match crate::twitch::listen_chat(ctx_fut.clone()).await {
                    Ok(never) => match never {},
                    Err(e) => e,
                };
                if last_crash.elapsed() >= Duration::from_secs(60 * 60 * 24) {
                    wait_time = Duration::from_secs(1); // reset wait time after no crash for a day
                } else {
                    wait_time *= 2; // exponential backoff
                }
                eprintln!("{} ({:?})", e, e);
                if wait_time >= Duration::from_secs(2) { // only notify on multiple consecutive errors
                    notify_thread_crash(format!("Twitch"), Box::new(e), Some(wait_time)).await;
                }
                sleep(wait_time).await; // wait before attempting to reconnect
                last_crash = Instant::now();
            }
        })
        .task(|ctx_fut, _| async move {
            shutdown.await;
            serenity_utils::shut_down(&*ctx_fut.read().await).await;
        })
        .ok()
}
