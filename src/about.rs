use {
    std::marker::PhantomData,
    chrono::prelude::*,
    doubloon::{
        Currency,
        Money,
        iso_currencies::{
            EUR,
            USD,
        },
    },
    icu_locale::locale,
    linode_rs::*,
    rocket::{
        Request,
        State,
        http::Status,
        response::content::RawHtml,
        uri,
    },
    rocket_util::{
        Origin,
        html,
    },
    rust_decimal::Decimal,
    serde::{
        Deserialize,
        de::{
            Deserializer,
            Error as _,
            Unexpected,
        },
    },
    serde_with::{
        DeserializeAs,
        serde_as,
    },
    sqlx::PgPool,
    wheel::traits::{
        IsNetworkError,
        ReqwestResponseExt as _,
    },
    crate::{
        api,
        auth,
        http::{
            PageStyle,
            Tab,
            page,
        },
        config::Config,
        user::User,
        wiki,
    },
};
#[cfg(not(target_os = "linux"))] use crate::systemd_minecraft;

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)] Minecraft(#[from] systemd_minecraft::Error),
    #[error(transparent)] Reqwest(#[from] reqwest::Error),
    #[error(transparent)] Sql(#[from] sqlx::Error),
    #[error(transparent)] Wheel(#[from] wheel::Error),
}

impl IsNetworkError for Error {
    fn is_network_error(&self) -> bool {
        match self {
            Self::Minecraft(_) => false,
            Self::Reqwest(e) => e.is_network_error(),
            Self::Sql(_) => false,
            Self::Wheel(e) => e.is_network_error(),
        }
    }
}

impl<'r> rocket::response::Responder<'r, 'static> for Error {
    fn respond_to(self, request: &'r Request<'_>) -> rocket::response::Result<'static> {
        let status = if self.is_network_error() {
            Status::BadGateway //TODO different status codes (e.g. GatewayTimeout for timeout errors)?
        } else {
            Status::InternalServerError
        };
        eprintln!("responded with {status} to request to {}", request.uri());
        eprintln!("display: {self}");
        eprintln!("debug: {self:?}");
        Err(status)
    }
}

trait StaticCurrency: Currency + Copy + Sized {
    const INSTANCE: Self;
}

impl StaticCurrency for EUR {
    const INSTANCE: Self = Self;
}

impl StaticCurrency for USD {
    const INSTANCE: Self = Self;
}

struct DeserializeMoney<C: StaticCurrency> {
    _phantom: PhantomData<C>,
}

impl<'de, C: StaticCurrency> DeserializeAs<'de, Money<C>> for DeserializeMoney<C> {
    fn deserialize_as<D: Deserializer<'de>>(deserializer: D) -> Result<Money<C>, D::Error> {
        #[derive(Deserialize)]
        struct DeserializedMoney {
            #[serde(with = "rust_decimal::serde::str")]
            amount: Decimal,
            currency: String,
        }

        let DeserializedMoney { amount, currency } = DeserializedMoney::deserialize(deserializer)?;
        if currency != C::INSTANCE.code() { return Err(D::Error::invalid_value(Unexpected::Str(&currency), &C::INSTANCE.code())) }
        Ok(Money::new(amount, C::INSTANCE))
    }
}

/*
fn format_eur(money: Money<EUR>) -> String {
    money.format(&doubloon::formatter::Formatter {
        digit_group_separator: " ", // thin space
        positive_template: "{a}€",
        negative_template: "−{a}€",
        ..doubloon::formatter::Formatter::default()
    }).expect("invalid money formatter")
}
*/

fn format_usd(money: Money<USD>) -> String {
    money.format(&locale!("en-DE"))
}

#[allow(unused)]
#[serde_as]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoneyOverview {
    /// The amount of money currently available to Wurstmineberg.
    #[serde_as(as = "DeserializeMoney<EUR>")]
    balance: Money<EUR>,
    /// The amount of money already available towards the current goal.
    #[serde_as(as = "DeserializeMoney<USD>")]
    amount: Money<USD>,
    /// The amount of money required for the current goal.
    #[serde_as(as = "Option<DeserializeMoney<USD>>")]
    goal: Option<Money<USD>>,
    /// The deadline to meet the current goal.
    due: Option<DateTime<Utc>>,
    /// A human-readable description of the current goal.
    text: String,
    /// A machine-readable description of the current goal.
    goal_info: GoalInfo,
    /// The linode tier and backup volume size currently in use.
    current_tier: LinodeTier,
    /// The linode tier and backup volume size that will be used next month assuming current funding.
    next_tier: LinodeTierWithPrice,
}

#[allow(unused)]
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum GoalInfo {
    Overdue,
    Base {
        year: i32,
        month: u32,
    },
    #[serde(rename_all = "camelCase")]
    ReduceDowngrade {
        year: i32,
        month: u32,
        is_next_month: bool,
        tier: LinodeTierWithPrice,
    },
    #[serde(rename_all = "camelCase")]
    Upgrade {
        year: i32,
        month: u32,
        is_next_month: bool,
        tier: LinodeTierWithPrice,
    },
    Buffer,
}

#[allow(unused)]
#[serde_as]
#[derive(Deserialize)]
struct LinodeTierWithPrice {
    #[serde(flatten)]
    base: LinodeTier,
    #[serde_as(as = "DeserializeMoney<USD>")]
    price: Money<USD>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinodeTier {
    linode_type: LinodeType,
    #[serde(rename = "backupGB")]
    backup_gb: u16,
}

#[rocket::get("/about")]
pub(crate) async fn get(db_pool: &State<PgPool>, me: Option<User>, uri: Origin<'_>, config: &State<Config>, http_client: &State<reqwest::Client>) -> Result<RawHtml<String>, Error> {
    let money_overview = http_client
        .get("https://night.fenhl.net/wurstmineberg/money/overview.json")
        .bearer_auth(&config.night.password)
        .send().await?
        .detailed_error_for_status().await?
        .json_with_text_in_error::<MoneyOverview>().await?;
    Ok(page(&me, &uri, PageStyle::default(), "About — Wurstmineberg", Tab::About, html! {
        div(class = "panel panel-default") {
            div(class = "panel-heading") {
                h3(class = "panel-title") : "About Wurstmineberg";
            }
            div(class = "panel-body") {
                p(class = "lead") : "Wurstmineberg is a whitelisted Minecraft server with a small number of active people. Our main world is a vanilla world running on the latest release. We sometimes also have a second modded world that is reset regularly. We do a lot of background and infrastructure work to make the most out of the vanilla Minecraft experience.";
            }
            div(class = "panel-footer") {
                @let running_worlds = systemd_minecraft::World::all_running().await?;
                @if running_worlds.is_empty() {
                    p : "None of our worlds are currently running. Check Discord for details.";
                } else {
                    @for world in running_worlds {
                        @if world.is_running().await? {
                            p {
                                //TODO restore world wiki links
                                @if world == systemd_minecraft::World::default() {
                                    : "Main world";
                                } else {
                                    : world.to_string();
                                }
                                : " address: ";
                                code {
                                    @if world != systemd_minecraft::World::default() {
                                        : world.to_string();
                                        : ".";
                                    }
                                    : "wurstmineberg.de";
                                }
                            }
                        }
                    }
                }
            }
        }
        h2(id = "hosting") {
            : "Hosting ";
            small : "Because Minecraft doesn't run on potatoes";
        }
        p : "Our hosting situation has changed many times now. We started out on a spare MacBook in a living room, tried commercial Minecraft server hosters, set everything up ourselves on a VPS, then a physical server, and are back to a VPS now.";
        p {
            : "Our current VPS is a ";
            a(href = "https://linode.com/") : "Linode";
            : " ";
            : money_overview.current_tier.linode_type.label;
            : " with ";
            : money_overview.current_tier.backup_gb;
            : "GB of backup space.";
        }
        p {
            : "For more details and a brief history of the server, read ";
            : wiki::link(db_pool, "hosting", "wiki", "the Hosting article on our wiki").await?;
            : ".";
        }
        h2(id = "joining") {
            : "Joining ";
            small : "Want to play with us?";
        }
        p : "If you're thinking about joining the server or wondering how you can, read on. First, though, there are a few things we would like to let you know about what playing on the server is like.";
        ul {
            li {
                : "We're pretty much end-game on the main world, as far as ";
                a(href = "/stats#achievements") : "the achievement progress";
                : " is concerned. The original dragon has been dead for ages and the ";
                : wiki::link(db_pool, "end", "wiki", "End").await?;
                : " has been turned into ";
                : wiki::link(db_pool, "ender-ender", "wiki", "an XP farm").await?;
                : ". The lunchbox which will be part of your ";
                : wiki::link(db_pool, "tour", "wiki", "server tour").await?;
                : " has things like an Ender chest in it. We also have beacons pretty much everywhere. If you want to start from scratch, either expect to be playing singleplayer style for a while, or consider joining when we start our next ";
                : wiki::link(db_pool, "renascence", "wiki", "Renascence").await?;
                : ".";
            }
            li {
                : "We think of a lot of stuff as public which you might expect to be treated differently. For example, we have a tradition of “leaking” parts of the chatlog to social media, and all sorts of in-game data like your inventory and statistics are publicly available through ";
                a(href = uri!(api::index)) : "our API";
                : ".";
            }
            li {
                : "We have a ";
                : wiki::link(db_pool, "coc", "wiki", "code of conduct").await?;
                : ".";
            }
        }
        p {
            : "Now for the actual invitation process, just ask one of our members to invite you. See ";
            a(href = uri!(crate::http::index)) : "the main page";
            : " for ways to contact us.";
        }
        h2(id = "starter") : "Getting started";
        p {
            : "If you're new and don't know where to go and what to do, chances are you didn't get a ";
            : wiki::link(db_pool, "tour", "wiki", "server tour").await?;
            : " by one of our more senior members. Sorry about that. The first thing you'll see is our ";
            : wiki::link(db_pool, "new-spawn", "wiki", "spawn area").await?;
            : ", which is rather obviously still being built, like so many things on the server. “Beneath” the spawn area is our ";
            : wiki::link(db_pool, "nether-hub-system", "wiki", "Nether hub").await?;
            : ", which serves as a central community trafficking area and connects various tunnels to other peoples's Nether portals. Both the Nether hub and spawn area are community oriented builds. All the supplies there are either donated or specifically collected for specific builds or purposes, so… well, the general rule is “Don't be a dick”. If something has a sign with “free to take” on it, you can probably take it. Taking ";
            em : "everything";
            : " is generally frowned upon for reasons of obviousness. Also, even if there are usually some community supplies floating around (like steak), please don't rely on them as your only means of, you know, not starving to death.";
        }
        p {
            : "If you'd like to work on community builds like spawners, ";
            : wiki::link(db_pool, "farms", "wiki", "farms").await?;
            : " or whatever, please try to ask around some of the other people who may or may not have already started preparations for such builds. It's not about ";
            em : "owning";
            : " a community project, it's just about coordination and not stepping on people's toes. However, experience shows that you probably won't be murdered in your sleep if you build at spawn without discussing every minor detail with everyone. Hanging around on Discord and/or checking out the ";
            a(href = uri!(wiki::index)) : "wiki";
            : " should give a start as to what's currently going on. And sometimes it doesn't.";
        }
        p : "As I was saying, it's about community. As long as you're being reasonable and remember you're not alone on this server (both for your and everybody else's benefit), everything's fine.";
        h2(id = "finance") : "Financial stuff";
        p : "The Wurstmineberg infrastructure runs on a VPS with monthly costs. Members may contribute to paying for these expenses on a voluntary basis. Depending on how much money is available, we will upgrade or downgrade the hardware on which the server runs. We may also occasionally use this money for other purposes, such as temporarily upgrading the server for events like Renascence or USC.";
        p {
            strong : "Note:";
            : " The server is billed in US dollars but contributions are typically in euros. Therefore, the status below is only an approximation and may change based on conversion rates at billing time.";
        }
        div {
            p {
                : "Current goal: ";
                : money_overview.text;
            }
            @if let GoalInfo::ReduceDowngrade { is_next_month: true, tier, .. } | GoalInfo::Upgrade { is_next_month: true, tier, .. } = money_overview.goal_info {
                p {
                    : "This month: ";
                    : money_overview.current_tier.linode_type.label;
                    : " with ";
                    : money_overview.current_tier.backup_gb;
                    : "GB backup space";
                }
                p {
                    : "Next month with current funding: ";
                    : money_overview.next_tier.base.linode_type.label;
                    : " with ";
                    : money_overview.next_tier.base.backup_gb;
                    : "GB backup space";
                }
                p {
                    : "Next month if goal is met: ";
                    : tier.base.linode_type.label;
                    : " with ";
                    : tier.base.backup_gb;
                    : "GB backup space";
                }
            }
            @if let Some(goal) = money_overview.goal {
                div(class = "progress") {
                    @let percent = if goal.is_zero() {
                        Decimal::from(if money_overview.amount.is_zero() { 100 } else { 0 })
                    } else {
                        Decimal::from(100) * (money_overview.amount.amount() / goal.amount()) //TODO contribute typesafe `div` to `doubloon`
                    };
                    div(class = "progress-bar progress-bar-success", style = format!("min-width: 5em; max-width: calc(100% - 5em); width: {percent}%; text-align: right; padding-right: 0.5em;")) : format_usd(money_overview.amount);
                    div(class = "progress-right") : format_usd(goal);
                }
            } else {
                p {
                    : "Current progress: ";
                    : format_usd(money_overview.amount);
                    : " (no target amount)";
                }
            }
        }
        @if me.is_some() {
            h3 : "Contribute";
            p : "If you would like to contribute money, please use one of the methods of transfer below, or contact Fenhl on Discord if none of them work for you. The amount and time of receipt will be published anonymously. Thank you.";
            div(class = "row") {
                div(class = "col-md-6") {
                    h4 : "bank transfer";
                    p {
                        : "Name: ";
                        : config.money.name;
                        br;
                        : "IBAN: ";
                        : config.money.iban;
                        br;
                        : "BIC: ";
                        : config.money.bic;
                        br;
                        : "Reference: Wurstmineberg";
                    }
                }
                div(class = "col-md-6") {
                    h4 : "PayPal";
                    p {
                        a(href = format!("https://www.paypal.me/{}", config.money.paypal)) {
                            : "paypal.me/";
                            : config.money.paypal;
                        }
                    }
                    p : "Please include the word Wurstmineberg in the note.";
                }
            }
        } else {
            p {
                a(href = uri!(auth::discord_login(Some(&uri)))) : "Log in";
                : " to view ways to contribute.";
            }
        }
    }))
}
