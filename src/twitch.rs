use {
    std::{
        collections::HashMap,
        convert::Infallible as Never,
        io,
    },
    futures::stream::TryStreamExt as _,
    minecraft::chat::Chat,
    //serde::Deserialize,
    serenity::prelude::*,
    serenity_utils::RwFuture,
    sqlx::types::Json,
    twitch_irc::{
        ClientConfig,
        SecureTCPTransport as SecureTcpTransport,
        TwitchIRCClient as TwitchIrcClient,
        message::ServerMessage,
    },
    crate::discord::DbPool,
};
#[cfg(not(target_os = "linux"))] use crate::systemd_minecraft;

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)] Minecraft(#[from] systemd_minecraft::Error),
    #[error(transparent)] Sql(#[from] sqlx::Error),
    #[error(transparent)] Validate(#[from] twitch_irc::validate::Error),
}

/*
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Config {
    #[serde(default = "make_wurstminebot")]
    bot_username: String,
    #[serde(rename = "clientID")]
    client_id: String,
    client_secret: String,
}
*/ //TODO

/*
impl Config {
    async fn user_config(&self) -> Result<UserConfig, Error> {
        let api_client = twitch_helix::Client::new(
            concat!("wurstminebot/", env!("CARGO_PKG_VERSION")),
            self.client_id.clone(),
            twitch_helix::Credentials::from_client_secret(&self.client_secret, iter::empty::<String>()),
        )?;
        let cfg = UserConfig::builder()
            .name(&self.bot_username)
            .token(format!("oauth:{}", api_client.get_oauth_token(None).await?))
            .enable_all_capabilities()
            .build()?;
        Ok(cfg)
    }
}
*/ //TODO twitch-irc equivalent

pub(crate) async fn listen_chat(ctx_fut: RwFuture<Context>) -> Result<Never, Error> {
    loop {
        let mut nick_map = HashMap::<String, String>::default();
        {
            let ctx = ctx_fut.read().await;
            let data = (*ctx).data.read().await;
            let pool = data.get::<DbPool>().expect("missing database connection");
            let mut query = sqlx::query!(r#"SELECT data->'minecraft'->'nicks'->-1 as "minecraft_nick!: Json<String>", data->'twitch'->'login' as "twitch_nick!: Json<String>" FROM people WHERE data->'minecraft'->'nicks'->-1 IS NOT NULL AND data->'twitch'->'login' IS NOT NULL"#)
                .fetch(pool);
            while let Some(person_data) = query.try_next().await? {
                nick_map.insert(person_data.twitch_nick.0, person_data.minecraft_nick.0);
            }
        }
        let client_config = ClientConfig::default(); //TODO use wurstminebot credentials
        let (mut incoming_messages, client) = TwitchIrcClient::<SecureTcpTransport, _>::new(client_config);
        for twitch_nick in nick_map.keys() {
            client.join(twitch_nick.clone())?; //TODO dynamically join/leave channels as nick map is updated
        }
        while let Some(msg) = incoming_messages.recv().await { //TODO move to a separate task, start before initial joins
            match msg {
                ServerMessage::Join(join) => if let Some(minecraft_nick) = nick_map.get(&join.channel_login) {
                    for world in systemd_minecraft::World::all_running().await? {
                        match world.tellraw(minecraft_nick, Chat::from(format!("[Twitch] reconnected")).color(minecraft::chat::Color::Aqua)).await {
                            Ok(_) => {}
                            Err(systemd_minecraft::Error::Rcon(rcon::Error::Io(e))) if e.kind() == io::ErrorKind::ConnectionRefused => {} // Minecraft world not fully running yet, skip “reconnected” message
                            Err(e) => return Err(e.into()),
                        }
                    }
                },
                ServerMessage::Part(part) => if let Some(minecraft_nick) = nick_map.get(&part.channel_login) {
                    for world in systemd_minecraft::World::all_running().await? {
                        world.tellraw(minecraft_nick, Chat::from(format!("[Twitch] disconnected")).color(minecraft::chat::Color::Aqua)).await?;
                    }
                },
                ServerMessage::Privmsg(pm) => if let Some(minecraft_nick) = nick_map.get(&pm.channel_login) {
                    for world in systemd_minecraft::World::all_running().await? {
                        let mut chat = Chat::from("[Twitch] ");
                        chat.color(minecraft::chat::Color::Aqua);
                        chat.add_extra({
                            let mut extra = Chat::from(if pm.is_action { format!("* {}", pm.sender.name) } else { format!("<{}>", pm.sender.name) });
                            extra.on_hover(minecraft::chat::HoverEvent::ShowText(Box::new(Chat::from(&*pm.sender.login))));
                            extra
                        });
                        chat.add_extra(" ");
                        chat.add_extra(&*pm.message_text);
                        world.tellraw(minecraft_nick, &chat).await?;
                    }
                },
                ServerMessage::Reconnect(_) => for minecraft_nick in nick_map.values() {
                    for world in systemd_minecraft::World::all_running().await? {
                        world.tellraw(minecraft_nick, Chat::from(format!("[Twitch] reconnected")).color(minecraft::chat::Color::Aqua)).await?;
                    }
                },
                //ServerMessage::UserNotice(notice) => unimplemented!(), //TODO display user notices (sub, raid, etc.)
                _ => {}
            }
        }
    }
}

//fn make_wurstminebot() -> String { format!("wurstminebot") } //TODO
