use {
    chrono::prelude::*,
    lazy_regex::regex_captures,
    rocket::{
        FromForm,
        State,
        form::{
            self,
            Context,
            Contextual,
            Form,
        },
        http::Status,
        response::{
            Redirect,
            content::RawHtml,
        },
        uri,
    },
    rocket_csrf::CsrfToken,
    rocket_util::{
        ContextualExt as _,
        CsrfForm,
        Origin,
        ToHtml,
        html,
    },
    serenity::{
        all::{
            Context as DiscordCtx,
            CreateAllowedMentions,
            CreateMessage,
            MessageBuilder,
        },
        model::prelude::*,
    },
    serenity_utils::RwFuture,
    sqlx::PgPool,
    url::Url,
    crate::{
        discord::{
            MessageBuilderExt as _,
            PgSnowflake,
        },
        form::{
            form_field,
            full_form,
        },
        http::{
            PageStyle,
            RedirectOrContent,
            StatusOrError,
            Tab,
            base_uri,
            page,
        },
        time::{
            DateTimeFormat,
            format_datetime,
        },
        user::User,
    },
};

const CHANNEL: ChannelId = ChannelId::new(681458815543148547);

#[derive(Debug, thiserror::Error, rocket_util::Error)]
pub(crate) enum Error {
    #[error(transparent)] Io(#[from] std::io::Error),
    #[error(transparent)] Serenity(#[from] serenity::Error),
    #[error(transparent)] Sql(#[from] sqlx::Error),
    #[error(transparent)] Url(#[from] url::ParseError),
}

impl<E: Into<Error>> From<E> for StatusOrError<Error> {
    fn from(e: E) -> Self {
        Self::Err(e.into())
    }
}

async fn mentions_to_tags(db_pool: &PgPool, mut text: String) -> sqlx::Result<String> {
    while let Some((_, prefix, bang, id, suffix)) = regex_captures!("^(.*?)<@(!?)([a-z][0-9a-z]{1,15}|[0-9]+)>(.*)$", &text) {
        if let Some(user) = User::from_discord_or_wmbid(db_pool, id).await? {
            let tag = if let Some(discord) = user.discorddata {
                if let Some(discriminator) = discord.discriminator {
                    format!("@{}#{discriminator:04}", discord.username)
                } else {
                    format!("@{}#", discord.username)
                }
            } else {
                format!("@{}#", user.wmbid().expect("user with no Discord data and no Wurstmineberg ID"))
            };
            text = format!("{prefix}{tag}{suffix}");
        } else {
            // skip this mention but convert the remaining text recursively
            return Ok(format!("{prefix}<@{bang}{id}>{}", Box::pin(mentions_to_tags(db_pool, suffix.to_owned())).await?))
        }
    }
    Ok(text)
}

async fn tags_to_mentions(db_pool: &PgPool, mut text: String) -> sqlx::Result<String> {
    while let Some((_, prefix, username, discriminator, suffix)) = regex_captures!("^(.*?)@([^@#:\n]{2,32})#((?:[0-9]{4})?)(.*)$", &text) { // see https://discord.com/developers/docs/resources/user
        if let Some(user) = User::from_tag(db_pool, username, discriminator.parse().ok()).await? {
            text = format!("{prefix}<@{}>{suffix}", user.id.url_part());
        } else {
            // skip this tag but convert the remaining text recursively
            return Ok(format!("{prefix}@{username}#{discriminator}{}", Box::pin(tags_to_mentions(db_pool, suffix.to_owned())).await?))
        }
    }
    Ok(text)
}

async fn link_open_tag(db_pool: &PgPool, article: &str, namespace: &str) -> sqlx::Result<RawHtml<String>> {
    let exists = sqlx::query_scalar!(r#"SELECT EXISTS (SELECT 1 FROM wiki WHERE title = $1 AND namespace = $2) AS "exists!""#, article, namespace).fetch_one(db_pool).await?;
    Ok(RawHtml(format!("<a{} href=\"{}\">", if exists { "" } else { " class=\"redlink\"" }, if namespace == "wiki" { uri!(main_article(article)) } else { uri!(namespaced_article(article, namespace)) })))
}

pub(crate) async fn link(db_pool: &PgPool, article: &str, namespace: &str, content: impl ToHtml) -> sqlx::Result<RawHtml<String>> {
    Ok(html! {
        : link_open_tag(db_pool, article, namespace).await?;
        : content;
        : RawHtml("</a>");
    })
}

#[rocket::get("/wiki")]
pub(crate) async fn index(db_pool: &State<PgPool>, me: Option<User>, uri: Origin<'_>) -> Result<RawHtml<String>, Error> {
    Ok(page(&me, &uri, PageStyle::default(), "Wurstmineberg Wiki", Tab::Wiki, html! {
        h1 : "Wurstmineberg Wiki";
        @let namespaces = sqlx::query_scalar!("SELECT name FROM wiki_namespaces ORDER BY name ASC").fetch_all(&**db_pool).await?;
        @if namespaces.is_empty() {
            p : "There are no articles in this wiki.";
        } else {
            @for namespace in namespaces {
                h2 : namespace;
                ul {
                    @let articles = sqlx::query_scalar!("SELECT DISTINCT title FROM wiki WHERE namespace = $1 ORDER BY title ASC", namespace).fetch_all(&**db_pool).await?;
                    @if articles.is_empty() {
                        li : "(This namespace is empty.)";
                    } else {
                        @for article in articles {
                            li {
                                a(href = if namespace == "wiki" { uri!(main_article(&article)) } else { uri!(namespaced_article(&article, &namespace)) }) : article;
                            }
                        }
                    }
                }
            }
        }
    }))
}

struct Markdown<'a>(Vec<pulldown_cmark::Event<'a>>);

impl<'a> ToHtml for Markdown<'a> {
    fn to_html(&self) -> RawHtml<String> {
        let mut rendered = RawHtml(String::default());
        pulldown_cmark::html::push_html(&mut rendered.0, self.0.iter().cloned());
        rendered
    }

    fn push_html(&self, buf: &mut RawHtml<String>) {
        pulldown_cmark::html::push_html(&mut buf.0, self.0.iter().cloned());
    }
}

async fn render_wiki_page<'a>(db_pool: &PgPool, source: &'a str) -> Result<Markdown<'a>, Error> {
    let mut events = Vec::default();
    let mut parser = pulldown_cmark::Parser::new_ext(
        &source,
        pulldown_cmark::Options::ENABLE_TABLES | pulldown_cmark::Options::ENABLE_FOOTNOTES | pulldown_cmark::Options::ENABLE_STRIKETHROUGH | pulldown_cmark::Options::ENABLE_MATH | pulldown_cmark::Options::ENABLE_SUPERSCRIPT | pulldown_cmark::Options::ENABLE_SUBSCRIPT,
    ).peekable();
    while let Some(event) = parser.next() {
        events.push(match event {
            pulldown_cmark::Event::UserMention(mention) => if let Some(user) = User::from_discord_or_wmbid(&*db_pool, &*mention).await? {
                pulldown_cmark::Event::Html(user.to_html().0.into())
            } else {
                pulldown_cmark::Event::Text(format!("<@{mention}>").into())
            },
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::Heading { level, mut id, classes, attrs }) => {
                if let Some(pulldown_cmark::Event::Text(text)) = parser.peek() {
                    id.get_or_insert(pulldown_cmark::CowStr::Boxed(text.chars().filter_map(|c| if c == ' ' { Some('-') } else if c.is_ascii_alphanumeric() { Some(c.to_ascii_lowercase()) } else { None }).collect::<Box<str>>()));
                }
                pulldown_cmark::Event::Start(pulldown_cmark::Tag::Heading { level, id, classes, attrs })
            }
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link { link_type, dest_url, title, id }) => {
                let dest_url = Url::options().base_url(Some(&"https://wurstmineberg.de/wiki/".parse()?)).parse(&dest_url)?;
                if let Some(relative) = Url::parse("https://wurstmineberg.de/wiki/")?.make_relative(&dest_url) {
                    if let Some((_, title)) = regex_captures!("^([0-9a-z_-]+)$", &relative) {
                        pulldown_cmark::Event::Html(link_open_tag(db_pool, title, "wiki").await?.0.into())
                    } else if let Some((_, title, namespace)) = regex_captures!("^([0-9a-z_-]+)/([0-9a-z_-]+)$", &relative) {
                        pulldown_cmark::Event::Html(link_open_tag(db_pool, title, namespace).await?.0.into())
                    } else {
                        pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link { link_type, dest_url: dest_url.to_string().into(), title, id })
                    }
                } else {
                    pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link { link_type, dest_url: dest_url.to_string().into(), title, id })
                }
            }
            _ => event,
        });
    }
    Ok(Markdown(events))
}

#[rocket::get("/wiki/<title>")]
pub(crate) async fn main_article(db_pool: &State<PgPool>, me: Option<User>, uri: Origin<'_>, title: &str) -> Result<RawHtml<String>, StatusOrError<Error>> {
    let source = sqlx::query_scalar!("SELECT text FROM wiki WHERE title = $1 AND namespace = 'wiki' ORDER BY timestamp DESC LIMIT 1", title).fetch_optional(&**db_pool).await?.ok_or_else(|| StatusOrError::Status(Status::NotFound))?;
    Ok(page(&me, &uri, PageStyle::default(), &format!("{title} — Wurstmineberg Wiki"), Tab::Wiki, html! {
        h1 {
            : title;
            : " — Wurstmineberg Wiki ";
            a(href = uri!(edit_get(title, "wiki")), class = "btn btn-primary") : "Edit";
            a(href = uri!(history(title, "wiki")), class = "btn btn-link") : "History";
        }
        : render_wiki_page(db_pool, &source).await?;
    }))
}

#[rocket::get("/wiki/<title>/<namespace>")]
pub(crate) async fn namespaced_article(db_pool: &State<PgPool>, me: Option<User>, uri: Origin<'_>, title: &str, namespace: &str) -> Result<RawHtml<String>, StatusOrError<Error>> {
    let source = sqlx::query_scalar!("SELECT text FROM wiki WHERE title = $1 AND namespace = $2 ORDER BY timestamp DESC LIMIT 1", title, namespace).fetch_optional(&**db_pool).await?.ok_or_else(|| StatusOrError::Status(Status::NotFound))?;
    Ok(page(&me, &uri, PageStyle::default(), &format!("{title} ({namespace}) — Wurstmineberg Wiki"), Tab::Wiki, html! {
        h1 {
            : title;
            : " (";
            : namespace;
            : ") — Wurstmineberg Wiki ";
            a(href = uri!(edit_get(title, namespace)), class = "btn btn-primary") : "Edit";
            a(href = uri!(history(title, namespace)), class = "btn btn-link") : "History";
        }
        : render_wiki_page(db_pool, &source).await?;
    }))
}

enum EditFormDefaults<'v> {
    Context(Context<'v>),
    Values {
        source: Option<String>,
    },
}

impl<'v> EditFormDefaults<'v> {
    fn errors(&self) -> Vec<&form::Error<'v>> {
        match self {
            Self::Context(ctx) => ctx.errors().collect(),
            Self::Values { .. } => Vec::default(),
        }
    }

    fn field_value(&self, field_name: &str) -> Option<&str> {
        match self {
            Self::Context(ctx) => ctx.field_value(field_name),
            Self::Values { .. } => None,
        }
    }

    fn source(&self) -> Option<&str> {
        match self {
            Self::Context(ctx) => ctx.field_value("source"),
            Self::Values { source } => source.as_deref(),
        }
    }
}

fn edit_form(me: User, uri: Origin<'_>, csrf: Option<&CsrfToken>, title: &str, namespace: &str, defaults: EditFormDefaults<'_>) -> RawHtml<String> {
    let mut errors = defaults.errors();
    page(&Some(me), &uri, PageStyle::default() /*TODO enable full_width and use column layout for edit/preview on wide screens?*/, &format!("edit — {title}{} — Wurstmineberg Wiki", if namespace == "wiki" { String::default() } else { format!(" ({namespace})") }), Tab::Wiki, html! {
        h1 {
            @if defaults.source().is_some() {
                : "Edit ";
            } else {
                : "Create ";
            }
            : title;
            : " (";
            : namespace;
            : ") — Wurstmineberg Wiki ";
            a(href = if namespace == "wiki" { uri!(main_article(title)) } else { uri!(namespaced_article(title, namespace)) }, class = "btn btn-danger") : "Cancel";
        }
        : full_form(uri!(edit_post(title, namespace)), csrf, html! {
            : form_field("source", &mut errors, "Text", html! {
                textarea(class = "form-control", name = "source") : defaults.source().unwrap_or_default();
            }, None);
            //TODO live preview
            : form_field("summary", &mut errors, "Edit Summary", html! {
                input(class = "form-control", type = "text", name = "summary", placeholder = "optional", value = defaults.field_value("summary"));
            }, None);
        }, errors, "Save");
    })
}

#[rocket::get("/wiki/<title>/<namespace>/edit")]
pub(crate) async fn edit_get(db_pool: &State<PgPool>, me: User, uri: Origin<'_>, csrf: Option<CsrfToken>, title: &str, namespace: &str) -> Result<RawHtml<String>, Error> {
    let source = if let Some(source) = sqlx::query_scalar!("SELECT text FROM wiki WHERE title = $1 AND namespace = $2 ORDER BY timestamp DESC LIMIT 1", title, namespace).fetch_optional(&**db_pool).await? {
        Some(mentions_to_tags(db_pool, source).await?)
    } else {
        None
    };
    Ok(edit_form(me, uri, csrf.as_ref(), title, namespace, EditFormDefaults::Values { source }))
}

#[derive(FromForm, CsrfForm)]
pub(crate) struct EditForm {
    #[field(default = String::new())]
    csrf: String,
    source: String,
    summary: String,
}

#[rocket::post("/wiki/<title>/<namespace>/edit", data = "<form>")]
pub(crate) async fn edit_post(discord_ctx: &State<RwFuture<DiscordCtx>>, db_pool: &State<PgPool>, me: User, uri: Origin<'_>, csrf: Option<CsrfToken>, title: &str, namespace: &str, form: Form<Contextual<'_, EditForm>>) -> Result<RedirectOrContent, Error> {
    let mut form = form.into_inner();
    form.verify(&csrf);
    Ok(if let Some(ref value) = form.value {
        if form.context.errors().next().is_some() {
            RedirectOrContent::Content(edit_form(me, uri, csrf.as_ref(), title, namespace, EditFormDefaults::Context(form.context)))
        } else {
            let mut transaction = db_pool.begin().await?;
            let exists = sqlx::query_scalar!(r#"SELECT EXISTS (SELECT 1 FROM wiki WHERE title = $1 AND namespace = $2) AS "exists!""#, title, namespace).fetch_one(&mut *transaction).await?;
            sqlx::query!("INSERT INTO wiki (title, namespace, text, author, timestamp, summary) VALUES ($1, $2, $3, $4, NOW(), $5)", title, namespace, tags_to_mentions(db_pool, value.source.clone()).await?, me.discord_id().map(PgSnowflake) as _, value.summary).execute(&mut *transaction).await?;
            transaction.commit().await?;
            let url = if namespace == "wiki" { uri!(base_uri(), main_article(title)) } else { uri!(base_uri(), namespaced_article(title, namespace)) };
            let mut content = MessageBuilder::default();
            content.push('<');
            content.push(url.to_string());
            content.push("> has been ");
            content.push(if exists { "edited" } else { "created" });
            content.push(" by ");
            content.mention_user(&me);
            if !value.summary.is_empty() {
                content.push_line(':');
                content.push_quote_safe(&value.summary);
            }
            CHANNEL.send_message(&*discord_ctx.read().await, CreateMessage::default().content(content.build()).allowed_mentions(CreateAllowedMentions::default())).await?;
            RedirectOrContent::Redirect(Redirect::to(if namespace == "wiki" { uri!(main_article(title)) } else { uri!(namespaced_article(title, namespace)) }))
        }
    } else {
        RedirectOrContent::Content(edit_form(me, uri, csrf.as_ref(), title, namespace, EditFormDefaults::Context(form.context)))
    })
}

#[rocket::get("/wiki/<title>/<namespace>/history")]
pub(crate) async fn history(db_pool: &State<PgPool>, me: Option<User>, uri: Origin<'_>, title: &str, namespace: &str) -> Result<RawHtml<String>, StatusOrError<Error>> {
    let revisions = sqlx::query!(r#"SELECT id, timestamp AS "timestamp: DateTime<Utc>", author AS "author: PgSnowflake<UserId>", summary FROM wiki WHERE title = $1 AND namespace = $2 ORDER BY timestamp DESC"#, title, namespace).fetch_all(&**db_pool).await?;
    if revisions.is_empty() { return Err(StatusOrError::Status(Status::NotFound)) }
    Ok(page(&me, &uri, PageStyle::default(), &format!("history of {title}{} — Wurstmineberg Wiki", if namespace == "wiki" { String::default() } else { format!(" ({namespace})") }), Tab::Wiki, html! {
        h1 {
            : "History of ";
            : title;
            @if namespace != "wiki" {
                : " (";
                : namespace;
                : ")";
            }
            : " — Wurstmineberg Wiki";
        }
        table(class = "table table-responsive") {
            thead {
                tr {
                    th : "Time";
                    th : "Author";
                    th : "Summary";
                }
            }
            tbody {
                @for revision in revisions {
                    tr {
                        td {
                            a(href = uri!(revision(title, namespace, revision.id))) {
                                : format_datetime(revision.timestamp, DateTimeFormat { long: false, running_text: false });
                            }
                        }
                        td {
                            @if let Some(PgSnowflake(author)) = revision.author {
                                @if let Some(author) = User::from_discord(&**db_pool, author).await? {
                                    : author;
                                } else {
                                    : "not found";
                                }
                            } else {
                                : "unknown";
                            }
                        }
                        td : revision.summary;
                    }
                }
            }
        }
    }))
}

#[rocket::get("/wiki/<title>/<namespace>/history/<rev>")]
pub(crate) async fn revision(db_pool: &State<PgPool>, me: Option<User>, uri: Origin<'_>, title: &str, namespace: &str, rev: Option<i32>) -> Result<RawHtml<String>, StatusOrError<Error>> {
    let Some(rev) = rev else { return Err(StatusOrError::Status(Status::NotFound)) }; // don't forward to Flask on wrong revision format, prevents an internal server error
    let source = sqlx::query_scalar!("SELECT text FROM wiki WHERE title = $1 AND namespace = $2 AND id = $3", title, namespace, rev).fetch_optional(&**db_pool).await?.ok_or_else(|| StatusOrError::Status(Status::NotFound))?;
    Ok(page(&me, &uri, PageStyle::default(), &format!("revision of {title}{} — Wurstmineberg Wiki", if namespace == "wiki" { String::default() } else { format!(" ({namespace})") }), Tab::Wiki, html! {
        h1 {
            : "Revision of ";
            : title;
            @if namespace != "wiki" {
                : " (";
                : namespace;
                : ")";
            }
            : " — Wurstmineberg Wiki ";
            a(href = if namespace == "wiki" { uri!(main_article(title)) } else { uri!(namespaced_article(title, namespace)) }, class = "btn btn-primary") : "View latest revision";
            a(href = uri!(history(title, namespace)), class = "btn btn-link") : "History";
        }
        : render_wiki_page(db_pool, &source).await?;
    }))
}
