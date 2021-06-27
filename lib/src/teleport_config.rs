use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
struct Teleport {
    app_service: Vec<AppService>,

    #[serde(flatten)]
    ignored: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AppService {
    enabled: bool,
    services: Vec<Service>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Service {
    name: String,
    uri: String,
    public_addr: String,
    labels: HashMap<String, String>,
}
