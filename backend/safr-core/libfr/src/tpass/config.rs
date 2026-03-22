use serde::{Deserialize, Serialize};
use std::env;
use tracing::debug;

#[derive(Serialize, Deserialize, Debug)]
pub struct TPassConf {
    pub url: String,
    pub user: String,
    pub pwd: String,
}

impl TPassConf {
    pub fn new(url: &str, user: &str, pwd: &str) -> Self {
        Self { url: url.to_string(), user: user.to_string(), pwd: pwd.to_string() }
    }
    //you should really pass these in
    //env vars are required
    pub fn from_env() -> Self {
        debug!("loading env vars for TPASS");
        let user = env::var("EFR_REMOTE_USER").expect("TPASS_USER env var");
        let url = env::var("EFR_REMOTE_URL").expect("TPASS_URL env var");
        let pwd = env::var("EFR_REMOTE_PWD").expect("TPASS_PWD env var");

        Self { url, user, pwd }
    }
}
