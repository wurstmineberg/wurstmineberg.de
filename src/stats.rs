use {
    rocket::{
        State,
        response::content::RawHtml,
        uri,
    },
    rocket_util::{
        Origin,
        html,
    },
    sqlx::PgPool,
    crate::{
        http::{
            PageStyle,
            Script,
            Tab,
            asset,
            page,
        },
        user::{
            self,
            User,
        },
        wiki,
    },
};

#[rocket::get("/stats")]
pub(crate) async fn get(db_pool: &State<PgPool>, me: Option<User>, uri: Origin<'_>) -> Result<RawHtml<String>, rocket_util::Error<sqlx::Error>> {
    Ok(page(&me, &uri, PageStyle { extra_scripts: vec![
        Script::External(asset("/js/stats.js")),
    ], ..PageStyle::default() }, "Stats — Wurstmineberg", Tab::Stats, html! {
        div(class = "panel panel-default") {
            div(class = "panel-heading") {
                h3(class = "panel-title") : "Statistics";
            }
            div(class = "panel-body") {
                p(class = "lead") : "These statistics, while mostly useless, are here to make you happy. More or less.";
            }
        }
        ul(id = "pagination", class = "nav nav-tabs") {
            li {
                a(id = "tab-stats-leaderboard", class = "tab-item", href = "#leaderboards") : "Leaderboards";
            }
            li {
                a(id = "tab-stats-mobs", class = "tab-item", href = "#mobs") : "Mobs";
            }
            li {
                a(id = "tab-stats-advancements", class = "tab-item", href = "#advancements") : "Advancements";
            }
            li {
                a(id = "tab-stats-achievements", class = "tab-item", href = "#achievements") : "Achievements";
            }
        }
        div(id = "stats-leaderboard", class = "section") {
            table(id = "stats-leaderboard-table", class = "table table-responsive stats-table") {
                thead {
                    tr {
                        th : "Stat";
                        th : "Leading player(s)";
                        th : "Value";
                        th : "Second place";
                        th : "Value";
                    }
                }
                tbody {
                    tr(id = "loading-stat-leaderboard-table", class = "loading-stat") {
                        td(colspan = "5") : "Loading stat data…";
                    }
                }
            }
        }
        div(id = "stats-mobs", class="section hidden") {
            table(id = "mobs-stats-table-bymob", class = "table table-responsive stats-table") {
                thead {
                    tr {
                        th : "Mob";
                        th : "Most killed player(s)";
                        th : "Value";
                        th : "Player(s) most killed by";
                        th : "Value";
                    }
                }
                tbody {
                    tr(id = "loading-mobs-bymob", class = "loading-stat") {
                        td(colspan = "5") : "Loading stat data…";
                    }
                }
            }
        }
        div(id = "stats-advancements", class = "section hidden") {
            p {
                : "This tab shows ";
                a(href = "https://minecraft.wiki/w/Advancements") : "advancements";
                : ", a new feature in ";
                a(href = "https://minecraft.wiki/w/Java_Edition_1.12") : "Minecraft 1.12";
                : " replacing ";
                a(href = "#achievements") : "achievements";
                : ". Only players who have logged in since ";
                a(href = "https://minecraft.wiki/w/Java_Edition_17w13a") : "snapshot 17w13a";
                : " will be displayed here.";
            }
            p {
                : "More detailed stats can be found on the ";
                a(href = uri!(user::list)) : "profile pages";
                : ".";
            }
            div(class = "row") {
                div(class = "col-lg-6 col-sm-12") {
                    h2 : "Leaderboard";
                    table(id = "stats-advancements-table-leaderboard", class = "table table-responsive") {
                        thead {
                            tr {
                                th : "Advancements completed";
                                th : "Player(s)";
                            }
                        }
                        tbody {
                            tr(id = "advancements-leaderboard-row-loading", class = "loading-stat") {
                                td(colspan = "2") : "Loading advancements data…";
                            }
                        }
                    }
                }
                div(class = "col-lg-6 col-sm-12") {
                    h2 : "Leaderboard by tab";
                    p {
                        : "Coming ";
                        : wiki::link(db_pool, "soon-tm", "wiki", "soon™").await?;
                    }
                }
            }
            h2 : "Stats by advancement";
            p {
                : "Coming ";
                : wiki::link(db_pool, "soon-tm", "wiki", "soon™").await?;
            }
        }
        div(id = "stats-achievements", class = "section hidden") {
            p {
                : "This tab shows ";
                a(href = "https://minecraft.wiki/w/Achievement/Java_Edition") : "achievements";
                : ", an old feature removed in ";
                a(href = "https://minecraft.wiki/w/Java_Edition_1.12") : "Minecraft 1.12";
                : " and replaced by ";
                a(href = "#advancements") : "advancements";
                : ". Only players who have not logged in since ";
                a(href = "https://minecraft.wiki/w/Java_Edition_17w13a") : "snapshot 17w13a";
                : " will be displayed here.";
            }
            div(class = "row") {
                div(class = "col-lg-6 col-sm-12") {
                    h2 : "Achievement Progress";
                    table(id = "stats-achievements-table-main-track", class = "table table-responsive") {
                        thead {
                            tr {
                                th : "&nbsp;";
                                th : "Achievement";
                                th : "Player(s)";
                            }
                        }
                        tbody {
                            tr(id = "achievement-row-none") {
                                td {
                                    img(class = "achievement-image", src = asset("/img/grid/air.png"));
                                }
                                td(class = "muted") : "no achievements yet";
                                td(class = "achievement-players") : " ";
                            }
                            tr(id = "achievement-row-loading", class = "loading-stat") {
                                td(colspan = "3") : "Loading achievement data…";
                            }
                            tr(id = "achievement-row-noadventuringtime") {
                                td {
                                    img(class = "achievement-image", src = asset("/img/grid/diamondboots.png"));
                                }
                                td(class = "muted") : "all except Adventuring Time";
                                td(class = "achievement-players") : " ";
                            }
                            tr(id = "achievement-row-all") {
                                td {
                                    img(class = "achievement-image fancy", src = asset("/img/grid-wurstpick.png"));
                                }
                                td(class = "muted") : "all achievements";
                                td(class = "achievement-players") : " ";
                            }
                        }
                    }
                }
                div(class = "col-lg-6 col-sm-12") {
                    h2 : "Adventuring Time";
                    table(id = "stats-achievements-table-biome-track", class = "table table-responsive") {
                        thead {
                            tr {
                                th : "Biomes visited";
                                th : "Player(s)";
                            }
                        }
                        tbody {
                            tr(id = "loading-achievements-table-biome-track", class = "loading-stat") {
                                td(colspan = "2") : "Loading Adventuring Time data…";
                            }
                        }
                    }
                    p {
                        : "For players who don't have ";
                        a(href = "https://minecraft.wiki/w/Achievement/Java_Edition#exploreAllBiomes") : "Adventuring Time";
                        : ", only biomes required for the achievement are counted here.";
                    }
                }
            }
        }
    }))
}
