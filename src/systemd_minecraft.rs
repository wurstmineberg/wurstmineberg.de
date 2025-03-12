//! A stub of the [systemd-minecraft](https://github.com/wurstmineberg/systemd-minecraft/tree/riir) crate, for testing wurstmineberg-web on non-Linux systems.

use {
    std::fmt,
    tokio::net::TcpStream,
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)] Wheel(#[from] wheel::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
