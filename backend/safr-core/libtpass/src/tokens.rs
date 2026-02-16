use chrono::Utc;
use jsonwebtoken::dangerous::insecure_decode;
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Serialize, Deserialize, Debug)]
pub struct JWT {
    pub token: String,
}
//TPass requires a JWT to use the api calls.
#[derive(Serialize, Deserialize, Debug)]
pub struct TPassToken {
    pub token: Option<String>,
    pub claims: Option<Claims>,
}

impl TPassToken {
    pub fn is_valid(&self) -> bool {
        if self.token.is_none() {
            return false; //early return
        }

        !self.is_expired()
    }

    fn time_remaining(&self) -> Option<u64> {
        match &self.claims {
            Some(claims) => {
                let now = Utc::now().timestamp() as u64;
                //1200 seconds is 20 min.
                match claims.exp.checked_sub(now) {
                    Some(t) if t >= 1200 => Some(t),
                    _ => None,
                }
            }
            None => None,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.time_remaining().is_none()
    }
}

impl From<JWT> for TPassToken {
    fn from(jwt: JWT) -> Self {
        let token = jwt.token;
        //tpass tokens don't have good signatures. We overlook that.
        let tk_data = insecure_decode::<Claims>(&token);

        let claims = match tk_data {
            Ok(t) => Some(t.claims),
            Err(e) => {
                warn!("couldn't parse JWT claims: {}", e);
                None
            }
        };

        Self {
            token: Some(token),
            claims,
        }
    }
}

//Claims are decoded from JWT and provide us with capabilities info provided by the server.
//The most important part right now is the expiration which we check as part of our verify
//process so we know when we have ask the server for a shiny new Token.
#[derive(Serialize, Deserialize, Debug)]
pub struct Claims {
    Name: String,
    Role: String,
    CCode: String,
    exp: u64,
}
