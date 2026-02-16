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
    //you should really pass these in
    //env vars are required
    pub fn from_env() -> Self {
        debug!("loading env vars for TPASS");
        let user = env::var("TPASS_USER").expect("TPASS_USER env var");
        let url = env::var("TPASS_URL").expect("TPASS_URL env var");
        let pwd = env::var("TPASS_PWD").expect("TPASS_PWD env var");

        Self { url, user, pwd }
    }
}
