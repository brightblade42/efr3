use crate::types::Image;
use libtpass::types::TPassProfile;
use serde::{Deserialize, Serialize};
//TODO: this will be a problem with other Remotes.

#[derive(Debug)]
pub struct SearchResult {
    pub image: Option<Image>,
    pub id: Option<String>,
    //pub details: Option<Value>, //json, let it be what it be.
    pub details: Option<TPassProfile>,
}

///A registration pair is the combination of our local fr_id and a client's external id.
///This combination is what binds our local fr info to a person.
#[derive(Debug, Serialize, Deserialize)]
pub struct RegistrationPair {
    pub ext_id: String,
    pub fr_id: String,
}

impl RegistrationPair {
    pub fn new(fr_id: String, ext_id: String) -> Self {
        RegistrationPair { ext_id, fr_id }
    }
}
