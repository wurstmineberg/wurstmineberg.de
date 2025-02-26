use {
    lazy_regex::regex_captures,
    rocket::{
        State,
        http::Status,
        response::content::RawHtml,
        uri,
    },
    rocket_util::{
        ToHtml,
        html,
    },
    sqlx::PgPool,
    url::Url,
    crate::{
        StatusOrError,
        Tab,
        page,
        user::User,
    },
};

#[derive(Debug, thiserror::Error, rocket_util::Error)]
pub(crate) enum Error {
    #[error(transparent)] Io(#[from] std::io::Error),
    #[error(transparent)] Sql(#[from] sqlx::Error),
    #[error(transparent)] Url(#[from] url::ParseError),
}

impl<E: Into<Error>> From<E> for StatusOrError<Error> {
    fn from(e: E) -> Self {
        Self::Err(e.into())
    }
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
pub(crate) async fn index(db_pool: &State<PgPool>, me: Option<User>) -> Result<RawHtml<String>, Error> {
    Ok(page(&me, "Wurstmineberg Wiki", Tab::Wiki, html! {
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
                                a(href = if namespace == "wiki" { uri!(main_article(&article)).to_string() } else { format!("/wiki/{article}/{namespace}") }) : article;
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
            pulldown_cmark::Event::UserMention(mention) => {
                let user = if let Ok(discord_id) = mention.parse() {
                    User::from_discord(&*db_pool, discord_id).await?
                } else {
                    User::from_wmbid(&*db_pool, mention.to_string()).await?
                };
                if let Some(user) = user {
                    pulldown_cmark::Event::Html(user.to_html().0.into())
                } else {
                    pulldown_cmark::Event::Text(format!("<@{mention}>").into())
                }
            }
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
pub(crate) async fn main_article(db_pool: &State<PgPool>, me: Option<User>, title: &str) -> Result<RawHtml<String>, StatusOrError<Error>> {
    let source = sqlx::query_scalar!("SELECT text FROM wiki WHERE title = $1 AND namespace = 'wiki' ORDER BY timestamp DESC LIMIT 1", title).fetch_optional(&**db_pool).await?.ok_or_else(|| StatusOrError::Status(Status::NotFound))?;
    Ok(page(&me, &format!("{title} — Wurstmineberg Wiki"), Tab::Wiki, html! {
        h1 {
            : title;
            : " — Wurstmineberg Wiki ";
            a(href = format!("/wiki/{title}/wiki/edit"), class = "btn btn-primary") : "Edit";
            a(href = format!("/wiki/{title}/wiki/history"), class = "btn btn-link") : "History";
        }
        : render_wiki_page(db_pool, &source).await?;
    }))
}

#[rocket::get("/wiki/<title>/<namespace>")]
pub(crate) async fn namespaced_article(db_pool: &State<PgPool>, me: Option<User>, title: &str, namespace: &str) -> Result<RawHtml<String>, StatusOrError<Error>> {
    let source = sqlx::query_scalar!("SELECT text FROM wiki WHERE title = $1 AND namespace = $2 ORDER BY timestamp DESC LIMIT 1", title, namespace).fetch_optional(&**db_pool).await?.ok_or_else(|| StatusOrError::Status(Status::NotFound))?;
    Ok(page(&me, &format!("{title} ({namespace}) — Wurstmineberg Wiki"), Tab::Wiki, html! {
        h1 {
            : title;
            : " (";
            : namespace;
            : ") — Wurstmineberg Wiki ";
            a(href = format!("/wiki/{title}/{namespace}/edit"), class = "btn btn-primary") : "Edit";
            a(href = format!("/wiki/{title}/{namespace}/history"), class = "btn btn-link") : "History";
        }
        : render_wiki_page(db_pool, &source).await?;
    }))
}

#[rocket::get("/wiki/<title>/<namespace>/history/<rev>")]
pub(crate) async fn revision(db_pool: &State<PgPool>, me: Option<User>, title: &str, namespace: &str, rev: Option<i32>) -> Result<RawHtml<String>, StatusOrError<Error>> {
    let Some(rev) = rev else { return Err(StatusOrError::Status(Status::NotFound)) }; // don't forward to Flask on wrong revision format, prevents an internal server error
    let source = sqlx::query_scalar!("SELECT text FROM wiki WHERE title = $1 AND namespace = $2 AND id = $3", title, namespace, rev).fetch_optional(&**db_pool).await?.ok_or_else(|| StatusOrError::Status(Status::NotFound))?;
    Ok(page(&me, &format!("revision of {title}{} — Wurstmineberg Wiki", if namespace == "wiki" { String::default() } else { format!(" ({namespace})") }), Tab::Wiki, html! {
        h1 {
            : "revision of ";
            : title;
            @if namespace != "wiki" {
                : " (";
                : namespace;
                : ")";
            }
            : " — Wurstmineberg Wiki ";
            a(href = if namespace == "wiki" { uri!(main_article(title)) } else { uri!(namespaced_article(title, namespace)) }.to_string(), class = "btn btn-primary") : "View latest revision";
            a(href = format!("/wiki/{title}/{namespace}/history"), class = "btn btn-link") : "History";
        }
        : render_wiki_page(db_pool, &source).await?;
    }))
}
