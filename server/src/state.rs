pub struct AppState;

pub fn load_state(_dir: &std::path::Path) -> anyhow::Result<AppState> {
    Ok(AppState)
}
