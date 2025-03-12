use {
    serde::Deserialize,
    serenity::model::prelude::*,
};
#[cfg(unix)] use wheel::fs;
#[cfg(windows)] use {
    tokio::process::Command,
    wheel::traits::IoResultExt as _,
};

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
        #[cfg(unix)] { Ok(fs::read_json("/opt/wurstmineberg/config.json").await?) }
        #[cfg(windows)] { // allow testing without having rust-analyzer slow down the server
            Ok(serde_json::from_slice(&Command::new("ssh").arg("wurstmineberg.de").arg("cat").arg("/opt/wurstmineberg/config.json").output().await.at_command("ssh")?.stdout)?)
        }
    }
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
    #[serde(rename = "clientID")]
    pub(crate) client_id: ApplicationId,
    pub(crate) client_secret: String,
}
