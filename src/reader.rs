use tracing::info;

pub fn get_content(url: &str) -> String {
    info!("Fetching content");
    "Some content".into()
}
