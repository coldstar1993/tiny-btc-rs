pub struct AppConfig {
    pub port: u32,
}

impl AppConfig {
    pub fn current_node(&self) -> String {
        format!("localhost:{}", self.port)
    }
}
