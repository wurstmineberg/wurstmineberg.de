#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use {
    std::{
        iter,
        time::Duration,
    },
    base64::engine::{
        Engine as _,
        general_purpose::STANDARD as BASE64,
    },
    itertools::Itertools as _,
    rocket::{
        Responder,
        Rocket,
        State,
        config::SecretKey,
        data::{
            Limits,
            ToByteUnit as _,
        },
        fs::FileServer,
        http::{
            Status,
            uri::{
                Segments,
                fmt::Path,
            },
        },
        request::{
            self,
            FromRequest,
            Request,
        },
        response::content::RawHtml,
        uri,
    },
    rocket_oauth2::{
        OAuth2,
        OAuthConfig,
    },
    rocket_util::{
        Doctype,
        Origin,
        Response,
        ToHtml,
        html,
    },
    sqlx::{
        PgPool,
        postgres::PgConnectOptions,
    },
    systemd_minecraft::World,
    url::Url,
    wheel::traits::ReqwestResponseExt as _,
    crate::{
        auth::DiscordUser,
        config::Config,
        user::User,
    },
};

mod about;
mod api;
mod auth;
mod config;
mod discord;
#[cfg(not(target_os = "linux"))] mod systemd_minecraft;
mod user;
mod wiki;

include!(concat!(env!("OUT_DIR"), "/build_output.rs"));

const HOST: &str = "wurstmineberg.de";

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

#[derive(Responder)]
enum StatusOrError<E> {
    Status(Status),
    Err(E),
}

fn base_uri() -> rocket::http::uri::Absolute<'static> {
    uri!("https://wurstmineberg.de")
}

fn asset(path: &str) -> String {
    format!("https://assets.{HOST}{path}") //TODO allow testing with local assetserver, like flask.g.assetserver
}

#[derive(Default)]
struct PageStyle {
    full_width: bool,
}

#[allow(unused)] //TODO port more pages to Rust
enum Tab {
    None,
    Home,
    About,
    People,
    Stats,
    Wiki,
    More,
    Login,
}

fn page(me: &Option<User>, uri: &Origin<'_>, style: PageStyle, title: &str, tab: Tab, content: impl ToHtml) -> RawHtml<String> {
    let PageStyle { full_width } = style;
    html! {
        : Doctype;
        html {
            head {
                meta(charset = "utf-8");
                title : title;
                meta(name = "viewport", content = "width=device-width, initial-scale=1, shrink-to-fit=no");
                meta(name = "description", content = "Info site for a minecraft server");
                meta(name = "author", content = "Wurstmineberg");
                link(href = "https://cdnjs.cloudflare.com/ajax/libs/twitter-bootstrap/3.3.7/css/bootstrap.min.css", rel = "stylesheet");
                link(rel = "icon", type = "image/png", href = asset("/img/logo/wurstpick_16.png"), sizes = "16x16");
                link(rel = "icon", type = "image/png", href = asset("/img/logo/wurstpick_32.png"), sizes = "32x32");
                link(rel = "icon", type = "image/png", href = asset("/img/logo/wurstpick_64.png"), sizes = "64x64");
                link(rel = "icon", type = "image/png", href = asset("/img/logo/wurstpick_128.png"), sizes = "128x128");
                link(rel = "icon", type = "image/png", href = asset("/img/logo/wurstpick_256.png"), sizes = "256x256");
                link(rel = "stylesheet", href = "https://netdna.bootstrapcdn.com/font-awesome/4.1.0/css/font-awesome.min.css");
                link(rel = "stylesheet", href = "https://fonts.googleapis.com/css?family=Lato&amp;subset=latin,latin-ext");
                link(rel = "stylesheet", href = asset("/css/common.css"));
                link(rel = "stylesheet", href = asset("/css/responsive.css"));
                link(rel = "stylesheet", href = asset("/css/dark.css"));
            }
            body {
                nav(class = "navbar navbar-inverse navbar-fixed-top") {
                    // Brand and toggle get grouped for better mobile display
                    div(class = "navbar-header") {
                        button(type = "button", class = "navbar-toggle", data_toggle = "collapse", data_target = ".navbar-ex1-collapse") {
                            span(class = "sr-only") : "Toggle navigation";
                            span(class = "icon-bar");
                            span(class = "icon-bar");
                            span(class = "icon-bar");
                        }
                        a(class = "navbar-brand", href = uri!(index).to_string()) : "Wurstmineberg";
                    }
                    // Collect the nav links, forms, and other content for toggling
                    div(class = "collapse navbar-collapse navbar-ex1-collapse") {
                        ul(id = "navbar-list", class = "nav navbar-nav") {
                            li(class? = matches!(tab, Tab::Home).then_some("active")) {
                                a(href = "/") {
                                    span(class = "fa fa-home");
                                    : "Home";
                                }
                            }
                            li(class? = matches!(tab, Tab::About).then_some("active")) {
                                a(href = uri!(about::get).to_string()) {
                                    span(class = "fa fa-info-circle");
                                    : "About";
                                }
                            }
                            li(class? = matches!(tab, Tab::People).then_some("active")) {
                                a(href = "/people") {
                                    span(class = "fa fa-users");
                                    : "People";
                                }
                            }
                            li(class? = matches!(tab, Tab::Stats).then_some("active")) {
                                a(href = "/stats") {
                                    span(class = "fa fa-table");
                                    : "Statistics";
                                }
                            }
                            li(class? = matches!(tab, Tab::Wiki).then_some("active")) {
                                a(href = uri!(wiki::index).to_string()) {
                                    span(class = "fa fa-book");
                                    : "Wiki";
                                }
                            }
                            li(class = if let Tab::More = tab { "dropdown active" } else { "dropdown" }) {
                                a(href = "#", class = "dropdown-toggle", data_toggle = "dropdown", aria_expanded = "true") {
                                    span(class = "fa fa-ellipsis-h");
                                    : "More";
                                    b(class = "caret");
                                }
                                ul(class = "dropdown-menu") {
                                    li {
                                        a(href = "/api") : "API";
                                    }
                                    li {
                                        a(href = format!("https://alltheitems.{HOST}/")) : "All The Items";
                                    }
                                    li {
                                        a(href = uri!(map).to_string()) : "Map";
                                    }
                                }
                            }
                        }
                        ul(class = "navbar-personaltools navbar-userloggedin nav navbar-nav navbar-right") {
                            @if let Some(me) = me {
                                li(class = if let Tab::Login = tab { "navbar-personaltools-tools active" } else { "navbar-personaltools-tools" }) {
                                    a(class = "dropdown-toggle", href = "#", data_toggle = "dropdown", title = format!("You are logged in as {me}."), aria_expanded = "true") : me.html_avatar(32);
                                    ul(class = "p-personal-tools dropdown-menu dropdown-menu-right") {
                                        li(id = "pt-userpage") {
                                            a(href = me.profile_url(), dir = "auto", title = "Your user page [ctrl-alt-.]", accesskey = ".") : me.to_string();
                                        }
                                        li(id = "pt-preferences") {
                                            a(href = "/preferences", title = "Your preferences") : "Preferences";
                                        }
                                        li(id = "pt-logout") {
                                            a(href = "/logout", title = "Your preferences") : "Log out";
                                        }
                                    }
                                }
                            } else {
                                li {
                                    li(class = "navbar-user-notloggedin") {
                                        a(href = uri!(auth::discord_login(Some(uri))), title = "You are not logged in.") {
                                            span(class = "glyphicon glyphicon-log-in", aria_hidden = "true");
                                            : "Log in";
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div(class = if full_width { "container fullwidth" } else { "container" }) : content;
                hr;
                p(class = "muted text-center") {
                    : "The People of wurstmineberg.de 2012–";
                    : YEAR_OF_LAST_COMMIT.to_string();
                }
                p(class = "muted text-center") : "Wurstmineberg is not created by, affiliated with, or supported by Discord Inc or Twitch Interactive.";
                script(src = "https://cdnjs.cloudflare.com/ajax/libs/jquery/1.12.4/jquery.min.js");
                script(src = "https://cdnjs.cloudflare.com/ajax/libs/twitter-bootstrap/3.3.7/js/bootstrap.min.js");
                script(src = "https://cdnjs.cloudflare.com/ajax/libs/underscore.js/1.8.3/underscore-min.js");
                script(src = "https://raw.githubusercontent.com/alexei/sprintf.js/master/dist/sprintf.min.js");
                script(src = asset("/js/common.js"));
                script(type = "text/javascript") : RawHtml(format!("
                    // expose current user to js, if logged in
                    var currentUser = {};
                    // run by default
                    linkifyHeaders();
                    configureNavigation();
                    setAnchorHeight();
                    displayFundingData();
                    initializeTooltips();
                    // mark as dev.wurstmineberg.de
                    if (isDev) {{
                        $('.navbar-brand').after($('<span>').css({{
                            color: 'red',
                            left: 100,
                            position: 'absolute',
                            top: 30,
                            transform: 'rotate(-10deg) scale(2)',
                            'z-index': 10
                        }}).text('[DEV]'));
                    }}
                ", if let Some(wmbid) = me.as_ref().and_then(|me| me.wmbid()) { format!("'{wmbid}'") } else { format!("null") })); //TODO support Discord-only users
            }
        }
    }
}

#[derive(Debug, thiserror::Error, rocket_util::Error)]
enum IndexError {
    #[error(transparent)] Minecraft(#[from] systemd_minecraft::Error),
    #[error(transparent)] Ping(#[from] craftping::Error),
    #[error(transparent)] Sql(#[from] sqlx::Error),
}

#[rocket::get("/")]
async fn index(db_pool: &State<PgPool>, me: Option<User>, uri: Origin<'_>) -> Result<RawHtml<String>, IndexError> {
    Ok(page(&me, &uri, PageStyle::default(), "Wurstmineberg", Tab::Home, html! {
        div(class = "panel panel-default") {
            div(class = "panel-heading") {
                h3(class = "panel-title") : "The Wurstmineberg Minecraft Server";
            }
            div(class = "panel-body") {
                p(class = "lead") {
                    : "Wurstmineberg is a whitelisted vanilla ";
                    a(href = "https://www.minecraft.net/") : "Minecraft";
                    : " server.";
                }
                p {
                    : "The main world usually runs the latest stable version of Minecraft. We don't run any serverside mods due to the work that would require. But we do have some awesome selfmade tools that really replace lots of that functionality. Sometimes a member also runs a ";
                    : wiki::link(db_pool, "modded-world", "wiki", "modded world").await?;
                    : ".";
                }
                p {
                    : "If you're interested in playing with us, we have some ";
                    a(href = uri!(_, about::get, "#joining").to_string()) : "requirements";
                    : ". In the meantime, you can check out the Discord section below.";
                }
            }
            div(class = "panel-footer", id = "serverinfo") {
                p {
                    //TODO show status of other worlds when running?
                    @let main_world = World::default();
                    @if main_world.is_running().await? {
                        : "The main world is currently ";
                        strong : "online";
                        @if let Some(version) = main_world.version().await? {
                            : " and running on version ";
                            a(href = format!("https://minecraft.wiki/w/Java_Edition_{}", version), style = "font-weight: bold;") : version;
                        }
                        : ", and ";
                        @let num_online = main_world.ping().await?.online_players;
                        span(id = "peopleCount") {
                            @if num_online == 0 {
                                : "none";
                            } else {
                                : num_online.to_string();
                            }
                            : " of the ";
                            span(id = "whitelistCount") : "(loading)"; //TODO
                            : " whitelisted players are";
                        }
                        : " currently active";
                        span(id = "punctuation") : ".";
                        br;
                        span(id = "peopleList"); //TODO show who is online
                    } else {
                        : "The main world is ";
                        strong : "offline";
                        : " right now. For more information, check ";
                        a(href = "https://discord.com/channels/88318761228054528/388412978677940226") : "#wurstmineberg";
                        : ".";
                    }
                }
                p {
                    : "See when someone's online in your ";
                    a(href = "https://github.com/wurstmineberg/systray#readme") : "Windows taskbar";
                    : " or ";
                    a(href = "https://github.com/wurstmineberg/bitbar-server-status#readme") : "macOS menu bar";
                }
            }
        }
        div(class = "row") {
            div(class = "col-md-4") {
                h2(id = "overview") {
                    img(src = asset("/img/grid/map.png"), class = "heading-icon nearest-neighbor", alt = "");
                    : " Server map";
                }
                p {
                    : "We set up ";
                    del : "Overviewer";
                    : " ";
                    del : "Mapcrafter";
                    : " ";
                    del : "Overviewer";
                    : " ";
                    a(href = "https://github.com/wurstmineberg/wurstmapberg") : "our own tool";
                    : " to generate a daily overview map of our main world. It works with current Minecraft versions, but it's still kind of experimental.";
                }
                a(class = "btn btn-default", href = uri!(map).to_string()) {
                    : "View map ";
                    i(class = "fa fa-chevron-right");
                }
            }
            div(class = "col-md-4") {
                h2(id = "irc") {
                    img(src = asset("/img/grid/bookandquill.png"), class = "heading-icon nearest-neighbor", alt = "");
                    : " Discord";
                }
                p {
                    : "We use ";
                    del : "IRC";
                    : " ";
                    del : "Slack";
                    : " ";
                    a(href = "https://discord.com/") : "Discord";
                    : " for communication internally, since it has both text and voice chat. Because sometimes it's just nice to talk to people.";
                }
                p {
                    : "If you're a server member or would like to be, you can ask the more active members for an invite link."; //TODO display Discord username on profile, display Discord logo in people list
                }
                a(class = "btn btn-default", href = "https://discord.com/download") {
                    : "Download app ";
                    i(class = "fa fa-chevron-right");
                }
            }
            div(class = "col-md-4") {
                h2(id = "twitter") {
                    img(src = asset("/img/grid/egg.png"), class = "heading-icon nearest-neighbor", alt = "");
                    : " Twitter";
                }
                p {
                    : "Long before the platform was renamed, we integrated Twitter into our server/IRC bot for ";
                    abbr(title = "It tended to get kinda spammy when new users arrived but people got used to it") : "automated tweets";
                    : " like death messages, lists of currently online players, or snippets from IRC shenanigans. ";
                    abbr(title = "At least we tried!") : "It was emotional";
                    : ".";
                }
                p : "The account has been inactive for a long time now, and we probably won't respond to DMs or mentions, but the old tweets are still there if you want to look at them.";
                a(class = "btn btn-default", href = "https://twitter.com/wurstmineberg") {
                    : "Go to account ";
                    i(class = "fa fa-chevron-right");
                }
            }
        }
        div(class = "row") {
            div(class = "col-md-4") {
                h2(id = "costs") {
                    img(src = asset("/img/grid/emerald.png"), class = "heading-icon nearest-neighbor", alt = "");
                    : " Costs";
                }
                p : "Since we're running on an actual server that requires actual monies to run, we decided to try a “donate if you will, or don't but it would be pretty dandy if you would” model.";
                //TODO explain auto resizing, display summary of current status
                a(class = "btn btn-default", href = uri!(_, about::get, "#finance").to_string()) {
                    : "More info ";
                    i(class = "fa fa-chevron-right");
                }
            }
            div(class = "col-md-4") {
                h2(id = "customtools") {
                    img(src = asset("/img/grid/goldpickaxe.png"), class = "heading-icon nearest-neighbor", alt = "");
                    : " Custom Tools";
                }
                p {
                    : "We have a GitHub organization where we keep most of our stuff, like ";
                    a(href = "https://github.com/wurstmineberg/wurstminebot-discord") : "the Discord bot";
                    : ", ";
                    a(href = "https://github.com/wurstmineberg/systemd-minecraft") : "the systemd service we use to manage our worlds";
                    : ", and even ";
                    a(href = "https://github.com/wurstmineberg/wurstmineberg.de") : "this website";
                    : ". We also have three single-serving sites for the current ";
                    a(href = format!("https://time.{HOST}/")) : "server time";
                    : ", ";
                    a(href = format!("https://weather.{HOST}/")) : "weather conditions";
                    : ", and ";
                    a(href = format!("https://accidents.{HOST}/")) : "time since the last death on the server";
                    : " (";
                    abbr(title = "Seriously. We need Javascript for nearly everything. We're working on reducing that though.") : "Javascript required";
                    : "). There's also the ";
                    a(href = "/api") : "Wurstmineberg API";
                    : ", which makes a lot of cool stuff possible, like our ";
                    a(href = "/stats") : "statistics page";
                    : ".";
                }
                a(class = "btn btn-default", href = "https://github.com/wurstmineberg") {
                    : "GitHub Organization ";
                    i(class = "fa fa-chevron-right");
                }
            }
            div(class = "col-md-4") {
                h2(id = "wiki") {
                    img(src = asset("/img/grid/enchantedbook.png"), class = "heading-icon nearest-neighbor", alt = "");
                    : " Wiki";
                }
                p {
                    : "When we made backups of everything on the old server, we sort of didn't check to make sure that the wiki backup was up to date. So now we have a wiki backup from 2014 as well as some downloads from the ";
                    a(href = "https://web.archive.org/") : "Wayback Machine";
                    : ". Since then, we wrote our own wiki software for increased actually-able-to-login-ness, but it's still missing ";
                    abbr(title = "like redlinks, templates, revision diffs, and a working Markdown preview") : "some important features";
                    : " so it might take a while for things to be back to normal.";
                }
                a(class = "btn btn-default", href = uri!(wiki::index).to_string()) {
                    : "Visit Wiki ";
                    i(class = "fa fa-chevron-right");
                }
            }
        }
    }))
}

#[rocket::get("/map")]
fn map(me: Option<User>, uri: Origin<'_>) -> RawHtml<String> {
    page(&me, &uri, PageStyle { full_width: true, ..PageStyle::default() }, "Map — Wurstmineberg", Tab::More, html! {
        div(id = "map", style = "height: calc(100vh - 91px);");
        link(rel = "stylesheet", href = "https://unpkg.com/leaflet@1.9.4/dist/leaflet.css", integrity = "sha256-p4NxAoJBhIIN+hmNHrzRCf9tD/miZyoHS5obTRR9BMY=", crossorigin = "");
        script(src = "https://unpkg.com/leaflet@1.9.4/dist/leaflet.js", integrity = "sha256-20nQCchB9co0qIjJZRGuk2/Z9VM+kNiyxNV1lvTlZBo=", crossorigin = "");
        script(src = static_url!("map.js").to_string());
    })
}

struct ProxyHttpClient(reqwest::Client);

struct Headers(reqwest::header::HeaderMap);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Headers {
    type Error = FlaskProxyError;

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let mut reqwest_headers = reqwest::header::HeaderMap::default();
        for header in req.headers().iter() {
            reqwest_headers.append(
                match header.name.as_str().parse::<reqwest::header::HeaderName>() {
                    Ok(name) => name,
                    Err(e) => return request::Outcome::Error((Status::InternalServerError, e.into())),
                },
                match header.value.parse::<reqwest::header::HeaderValue>() {
                    Ok(value) => value,
                    Err(e) => return request::Outcome::Error((Status::InternalServerError, e.into())),
                },
            );
        }
        request::Outcome::Success(Self(reqwest_headers))
    }
}

#[derive(Debug, thiserror::Error, rocket_util::Error)]
enum FlaskProxyError {
    #[error(transparent)] InvalidHeaderName(#[from] reqwest::header::InvalidHeaderName),
    #[error(transparent)] InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),
    #[error(transparent)] Reqwest(#[from] reqwest::Error),
    #[error(transparent)] Url(#[from] url::ParseError),
    #[error("internal server error in proxied Flask application:\n{0}")]
    InternalServerError(String),
}

fn proxy_headers(headers: Headers, discord_user: Option<DiscordUser>) -> Result<reqwest::header::HeaderMap, FlaskProxyError> {
    let mut headers = headers.0;
    headers.insert(reqwest::header::HOST, reqwest::header::HeaderValue::from_static("gefolge.org"));
    headers.insert(reqwest::header::HeaderName::from_static("x-forwarded-proto"), reqwest::header::HeaderValue::from_static("https"));
    if let Some(discord_user) = discord_user {
        headers.insert(reqwest::header::HeaderName::from_static("x-gefolge-authorized-discord-id"), discord_user.id.to_string().parse()?);
    } else {
        headers.remove(reqwest::header::HeaderName::from_static("x-gefolge-authorized-discord-id"));
    }
    Ok(headers)
}

#[derive(Responder)]
enum FlaskProxyResponse {
    Proxied(Response<reqwest::Response>),
    Status(Status),
}

#[rocket::get("/<path..>")]
async fn flask_proxy_get(proxy_http_client: &State<ProxyHttpClient>, me: Option<DiscordUser>, origin: Origin<'_>, headers: Headers, path: Segments<'_, Path>) -> Result<FlaskProxyResponse, FlaskProxyError> {
    if Segments::<Path>::get(&path, 0).map_or(true, |prefix| !matches!(prefix, "api" | "people" | "preferences" | "profile" | "stats" | "wiki")) {
        // only forward the directories that are actually served by the proxy to prevent internal server errors on malformed requests from spambots
        return Ok(FlaskProxyResponse::Status(Status::NotFound))
    }
    let mut url = Url::parse("http://127.0.0.1:24823/")?;
    url.path_segments_mut().expect("proxy URL is cannot-be-a-base").extend(path);
    url.set_query(origin.0.query().map(|query| query.as_str()));
    let response = proxy_http_client.0.get(url).headers(proxy_headers(headers, me)?).send().await?;
    if response.status() == reqwest::StatusCode::INTERNAL_SERVER_ERROR {
        return Err(FlaskProxyError::InternalServerError(response.text().await?))
    }
    Ok(FlaskProxyResponse::Proxied(Response(response)))
}

#[rocket::post("/<path..>", data = "<data>")]
async fn flask_proxy_post(proxy_http_client: &State<ProxyHttpClient>, me: Option<DiscordUser>, origin: Origin<'_>, headers: Headers, path: Segments<'_, Path>, data: Vec<u8>) -> Result<FlaskProxyResponse, FlaskProxyError> {
    if Segments::<Path>::get(&path, 0).map_or(true, |prefix| !matches!(prefix, "api" | "people" | "preferences" | "profile" | "stats" | "wiki")) {
        // only forward the directories that are actually served by the proxy to prevent internal server errors on malformed requests from spambots
        return Ok(FlaskProxyResponse::Status(Status::NotFound))
    }
    let mut url = Url::parse("http://127.0.0.1:24823/")?;
    url.path_segments_mut().expect("proxy URL is cannot-be-a-base").extend(path);
    url.set_query(origin.0.query().map(|query| query.as_str()));
    let response = proxy_http_client.0.post(url).headers(proxy_headers(headers, me)?).body(data).send().await?;
    if response.status() == reqwest::StatusCode::INTERNAL_SERVER_ERROR {
        return Err(FlaskProxyError::InternalServerError(response.text().await?))
    }
    Ok(FlaskProxyResponse::Proxied(Response(response)))
}

#[rocket::catch(400)]
async fn bad_request(request: &Request<'_>) -> RawHtml<String> {
    let me = request.guard::<User>().await.succeeded();
    let uri = request.guard::<Origin<'_>>().await.succeeded().unwrap_or_else(|| Origin(uri!(index)));
    page(&me, &uri, PageStyle::default(), "Bad Request — Wurstmineberg", Tab::None, html! {
        h1 : "Error 400: Bad Request";
        p : "Login failed. If you need help, please ask in #dev on Discord.";
    })
}

#[rocket::catch(401)]
async fn unauthorized(request: &Request<'_>) -> RawHtml<String> {
    let me = request.guard::<User>().await.succeeded();
    let uri = request.guard::<Origin<'_>>().await.succeeded().unwrap_or_else(|| Origin(uri!(index)));
    page(&me, &uri, PageStyle::default(), "Unauthorized — Wurstmineberg", Tab::None, html! {
        h1 : "Error 401: Unauthorized";
        @if me.is_some() {
            p : "You don't have access to this page. If you think this is a bug, please report it in #dev on Discord.";
        } else {
            p : "You don't have access to this page. Try logging in first!";
        }
    })
}

#[rocket::catch(404)]
async fn not_found(request: &Request<'_>) -> RawHtml<String> {
    let me = request.guard::<User>().await.succeeded();
    let uri = request.guard::<Origin<'_>>().await.succeeded().unwrap_or_else(|| Origin(uri!(index)));
    page(&me, &uri, PageStyle::default(), "Not Found — Wurstmineberg", Tab::None, html! {
        h1 : "Error 404: Not Found";
        p : "This page does not exist.";
    })
}

#[rocket::catch(500)]
async fn internal_server_error(request: &Request<'_>) -> RawHtml<String> {
    let config = request.guard::<&State<Config>>().await.expect("missing config");
    let http_client = request.guard::<&State<reqwest::Client>>().await.expect("missing HTTP client");
    let me = request.guard::<User>().await.succeeded();
    let uri = request.guard::<Origin<'_>>().await.succeeded().unwrap_or_else(|| Origin(uri!(index)));
    let is_reported = night_report(config, http_client, "/dev/gharch/webError", Some("internal server error")).await.is_ok();
    page(&me, &uri, PageStyle::default(), "Internal Server Error — Wurstmineberg", Tab::None, html! {
        h1 : "Error 500: Internal Server Error";
        p : "This is a sad time. An error occured.";
        @if is_reported {
            p : "This error has been reported to the Wurstmineberg site admins. We'll try to fix it soon™.";
        } else {
            p : "Please report this to the Wurstmineberg site admins.";
        }
    })
}

#[rocket::catch(default)]
async fn fallback_catcher(status: Status, request: &Request<'_>) -> RawHtml<String> {
    let config = request.guard::<&State<Config>>().await.expect("missing config");
    let http_client = request.guard::<&State<reqwest::Client>>().await.expect("missing HTTP client");
    let me = request.guard::<User>().await.succeeded();
    let uri = request.guard::<Origin<'_>>().await.succeeded().unwrap_or_else(|| Origin(uri!(index)));
    let is_reported = night_report(config, http_client, "/dev/gharch/webError", Some("responding with unexpected HTTP status code")).await.is_ok();
    page(&me, &uri, PageStyle::default(), &format!("{} — Wurstmineberg", status.reason_lossy()), Tab::None, html! {
        h1 {
            : "Error ";
            : status.code;
            : ": ";
            : status.reason_lossy();
        }
        p : "This is a sad time. An error occured.";
        @if is_reported {
            p : "This error has been reported to the Wurstmineberg site admins. We'll try to fix it soon™.";
        } else {
            p : "Please report this to the Wurstmineberg site admins.";
        }
    })
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
    let Rocket { .. } = rocket::custom(rocket::Config {
        secret_key: SecretKey::from(&BASE64.decode(&config.web.secret_key)?),
        log_level: rocket::config::LogLevel::Critical,
        port: 24822,
        limits: Limits::default()
            .limit("bytes", 2.mebibytes()), // for proxied wiki edits
        ..rocket::Config::default()
    })
    .mount("/", rocket::routes![
        index,
        map,
        flask_proxy_get,
        flask_proxy_post,
        about::get,
        api::discord_voice_state,
        auth::discord_callback,
        auth::discord_login,
        auth::logout,
        wiki::index,
        wiki::main_article,
        wiki::namespaced_article,
        wiki::revision,
    ])
    .mount("/static", FileServer::new({
        #[cfg(windows)] { rocket::fs::relative!("assets/static") }
        #[cfg(not(windows))] { "/opt/git/github.com/wurstmineberg/wurstmineberg.de/main/assets/static" }
    }, rocket::fs::Options::None))
    .register("/", rocket::catchers![
        bad_request,
        unauthorized,
        not_found,
        internal_server_error,
        fallback_catcher,
    ])
    .attach(OAuth2::<auth::Discord>::custom(rocket_oauth2::HyperRustlsAdapter::default(), OAuthConfig::new(
        rocket_oauth2::StaticProvider::Discord,
        config.wurstminebot.client_id.to_string(),
        config.wurstminebot.client_secret.to_string(),
        Some(uri!(base_uri(), auth::discord_callback).to_string()),
    )))
    .manage(config)
    .manage(PgPool::connect_with(PgConnectOptions::default().username("wurstmineberg").database("wurstmineberg").application_name("wurstmineberg-web")).await?)
    .manage(http_client)
    .manage(ProxyHttpClient(proxy_http_client))
    .launch().await?;
    Ok(())
}
