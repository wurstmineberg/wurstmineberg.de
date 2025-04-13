use {
    std::io,
    rocket::fs::NamedFile,
    crate::user::User,
};

#[rocket::get("/api/v3/discord/voice-state.json")]
pub(crate) async fn discord_voice_state(me: User) -> io::Result<NamedFile> {
    let _ = me; // only required for authorization
    NamedFile::open("/opt/wurstmineberg/discord/voice-state.json").await
}
