//! A stub of the [systemd-minecraft](https://github.com/wurstmineberg/systemd-minecraft/tree/riir) crate, for testing wurstmineberg-web on non-Linux systems.

use {
    std::{
        fmt,
        path::{
            Path,
            PathBuf,
        },
    },
    lazy_regex::regex_is_match,
    minecraft::chat::Chat,
    rocket::request::FromParam,
    serde::Deserialize,
    tokio::net::TcpStream,
};

const WORLDS_DIR: &str = "/opt/wurstmineberg/world";

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)] Rcon(#[from] rcon::Error),
    #[error(transparent)] Wheel(#[from] wheel::Error),
}

pub(crate) enum VersionSpec {
    Exact(#[allow(unused)] String),
    LatestRelease,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub(crate) struct World(#[allow(unused)] String);

impl World {
    pub async fn all() -> Result<Vec<Self>, Error> {
        Ok(vec![Self::default()])
    }

    pub async fn all_running() -> Result<Vec<Self>, Error> {
        let mut running = Vec::default();
        for world in Self::all().await? {
            if world.is_running().await? {
                running.push(world);
            }
        }
        Ok(running)
    }

    fn new(name: impl ToString) -> Self {
        Self(name.to_string())
    }

    pub async fn command(&self, _: &str) -> Result<String, Error> {
        Ok(String::default())
    }

    pub(crate) fn dir(&self) -> PathBuf {
        Path::new(WORLDS_DIR).join(&self.0)
    }

    pub(crate) async fn is_running(&self) -> Result<bool, Error> {
        Ok(true)
    }

    pub(crate) async fn ping(&self) -> craftping::Result<craftping::Response> {
        let (hostname, port) = match &*self.0 {
            "creative" => (format!("wurstmineberg.de"), 25562),
            "testworld" => (format!("wurstmineberg.de"), 25580),
            "usc" => (format!("wurstmineberg.de"), 25569),
            "wurstmineberg" => (format!("wurstmineberg.de"), 25568),
            _ => (format!("{self}.wurstmineberg.de"), 25565),
        };
        let mut stream = TcpStream::connect((&*hostname, port)).await?;
        craftping::tokio::ping(&mut stream, &hostname, port).await
    }

    pub(crate) async fn tellraw(&self, rcpt: &str, msg: &Chat) -> Result<String, Error> {
        Ok(self.command(&format!("tellraw {} {}", rcpt, msg)).await?)
    }

    pub(crate) async fn update(&self, _: VersionSpec) -> Result<(), Error> {
        Ok(())
    }

    pub(crate) async fn version(&self) -> Result<Option<String>, Error> {
        Ok(Some(format!("1.21.4")))
    }
}

impl Default for World {
    fn default() -> Self {
        Self(format!("wurstmineberg"))
    }
}

impl fmt::Display for World {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum WorldFromParamError {
    #[error(transparent)] Wheel(#[from] wheel::Error),
    #[error("world name must only consist of ASCII letters/numbers/underscores")]
    Name,
}

impl<'r> FromParam<'r> for World {
    type Error = WorldFromParamError;

    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        if regex_is_match!("^[0-9A-Za-z_]+$", param) {
            Ok(Self::new(param))
        } else {
            Err(WorldFromParamError::Name)
        }
    }
}
