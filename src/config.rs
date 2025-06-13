use {
    std::collections::{
        HashMap,
        HashSet,
    },
    serde::Deserialize,
    serenity::{
        model::prelude::*,
        prelude::TypeMapKey,
    },
};
#[cfg(unix)] use {
    std::path::Path,
    wheel::fs,
    crate::BASE_PATH,
};
#[cfg(windows)] use {
    tokio::process::Command,
    wheel::traits::IoResultExt as _,
};
#[cfg(not(target_os = "linux"))] use crate::systemd_minecraft;

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[cfg(windows)] #[error(transparent)] Json(#[from] serde_json::Error),
    #[error(transparent)] Wheel(#[from] wheel::Error),
}

#[derive(Clone, Deserialize)]
pub(crate) struct Config {
    pub(crate) money: Money,
    pub(crate) night: Night,
    pub(crate) web: Web,
    pub(crate) wurstminebot: Wurstminebot,
}

impl Config {
    pub(crate) async fn load() -> Result<Self, Error> {
        #[cfg(unix)] { Ok(fs::read_json(Path::new(BASE_PATH).join("config.json")).await?) }
        #[cfg(windows)] { // allow testing without having rust-analyzer slow down the server
            Ok(serde_json::from_slice(&Command::new("ssh").arg("wurstmineberg.de").arg("cat").arg("/opt/wurstmineberg/config.json").output().await.at_command("ssh")?.stdout)?)
        }
    }
}

impl TypeMapKey for Config {
    type Value = Self;
}

#[derive(Clone, Deserialize)]
pub(crate) struct Money {
    pub(crate) bic: String,
    pub(crate) iban: String,
    pub(crate) name: String,
    #[serde(rename = "payPal")]
    pub(crate) paypal: String,
}

#[derive(Clone, Deserialize)]
pub(crate) struct Night {
    pub(crate) password: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Web {
    pub(crate) secret_key: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Wurstminebot {
    pub(crate) bot_token: String,
    #[serde(rename = "clientID")]
    pub(crate) client_id: ApplicationId,
    pub(crate) client_secret: String,
    #[serde(default)]
    pub(crate) self_assignable_roles: HashSet<RoleId>,
    #[serde(default)]
    pub(crate) world_channels: HashMap<systemd_minecraft::World, ChannelId>,
    #[serde(default)]
    pub(crate) world_channel_topics: HashMap<systemd_minecraft::World, String>,
}
