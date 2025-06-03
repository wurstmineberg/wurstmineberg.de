#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use {
    std::{
        iter,
        time::Duration,
    },
    itertools::Itertools as _,
    rocket::Rocket,
    wheel::traits::ReqwestResponseExt as _,
    crate::config::Config,

};

mod about;
mod api;
mod auth;
mod config;
mod discord;
mod http;
#[cfg(not(target_os = "linux"))] mod systemd_minecraft;
mod user;
mod wiki;

include!(concat!(env!("OUT_DIR"), "/build_output.rs"));

async fn night_report(config: &Config, http_client: &reqwest::Client, path: &str, extra: Option<&str>) -> Result<(), Error> {
    http_client
        .post("https://night.fenhl.net/dev/gharch/report")
        .bearer_auth(&config.night.password)
        .form(&iter::once(("path", path)).chain(extra.map(|extra| ("extra", extra))).collect_vec())
        .send().await?
        .detailed_error_for_status().await?;
    Ok(())
}

fn night_report_sync(config: &Config, path: &str, extra: Option<&str>) -> Result<(), Error> {
    reqwest::blocking::Client::new()
        .post("https://night.fenhl.net/dev/gharch/report")
        .bearer_auth(&config.night.password)
        .form(&iter::once(("path", path)).chain(extra.map(|extra| ("extra", extra))).collect_vec())
        .send()?
        .error_for_status()?;
    Ok(())
}


#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)] Base64(#[from] base64::DecodeError),
    #[error(transparent)] Config(#[from] config::Error),
    #[error(transparent)] Reqwest(#[from] reqwest::Error),
    #[error(transparent)] Rocket(#[from] rocket::Error),
    #[error(transparent)] Sql(#[from] sqlx::Error),
    #[error(transparent)] Wheel(#[from] wheel::Error),
}

#[wheel::main(rocket)]
async fn main() -> Result<(), Error> {
    let config = Config::load().await?;
    let panic_config = config.clone();
    let default_panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = night_report_sync(&panic_config, &format!("/dev/gharch/webError"), Some("thread panic"));
        default_panic_hook(info)
    }));
    let http_client = reqwest::Client::builder()
        .user_agent(concat!("WurstminebergWeb/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .use_rustls_tls()
        .hickory_dns(true)
        .https_only(true)
        .build()?;
    let proxy_http_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("WurstminebergWeb/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(90))
        .build()?;
    let Rocket { .. } = http::rocket(config, http_client, proxy_http_client).await?.launch().await?;
    Ok(())
}
