//! A stub of the [systemd-minecraft](https://github.com/wurstmineberg/systemd-minecraft/tree/riir) crate, for testing wurstmineberg-web on non-Linux systems.

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)] Wheel(#[from] wheel::Error),
}

    #[derive(Debug, Clone)]
pub(crate) struct World(#[allow(unused)] String);

impl World {
    pub(crate) async fn is_running(&self) -> Result<bool, Error> {
        Ok(true)
    }

    pub fn status(&self) -> Result<mcping::Response, mcping::Error> {
        Err(mcping::Error::DnsLookupFailed)
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
