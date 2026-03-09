mod tpass;

use crate::{EnrollData, FRResult, Image, SearchBy};
use libtpass::types::TPassProfile;
use serde::{Deserialize, Serialize};
//some external api based system that holds information about the people that need recognizing.
#[allow(async_fn_in_trait)]
pub trait Remote: Send + Sync {
    async fn register_enrollment(&self, reg_pair: &RegistrationPair) -> FRResult<()>;
    async fn unregister_enrollment(&self) -> FRResult<()>;
    async fn search(&self, enroll_data: &EnrollData) -> FRResult<Vec<SearchResult>>;
    async fn search_one(
        &self,
        search: SearchBy,
        include_image: bool,
    ) -> FRResult<Option<SearchResult>>;
    async fn search_by_ids(
        &self,
        search: SearchBy,
        include_img: bool,
    ) -> FRResult<Vec<SearchResult>>;
    //async fn create_profile(&self, some_profile_info) -> FRResult;
}

//package up what is returned from a remote.

//#[derive(Debug, Serialize, Deserialize)]
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
