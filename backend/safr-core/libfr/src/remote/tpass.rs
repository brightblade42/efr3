use crate::{remote::Remote, EnrollData, EnrollDetails, FRError, FRResult, Image, SearchBy};

use libtpass::{api::TPassClient, api::TResult, errors::TPassError};
use serde_json::json;
use tracing::{error, info, warn};

//pub type TPassResult<T> = Result<T, TPassError>;

async fn handle_name_search(
    client: &TPassClient,
    first_name: &str,
    last_name: &str,
) -> TResult<Option<SearchResult>> {
    let full_name = format!("{},{}", last_name, first_name);

    // Grab the first item, return Ok(None) if empty
    let Some(item) = client.search_by_name(&full_name).await?.into_iter().next() else {
        return Ok(None);
    };

    // Enforce the img_url rule
    if item.img_url.is_none() {
        return Err(TPassError::MissingImageURL {
            last_name: item.l_name.unwrap_or("Unknown".to_string()),
            first_name: item.f_name.unwrap_or("Unknown".to_string()),
            ext_id: item.ccode.unwrap_or(0),
        });
    }

    Ok(Some(SearchResult {
        image: None,
        id: item.ccode.map(|id| id.to_string()),
        details: Some(item),
    }))
}

// Helper for ExtID Search
async fn handle_ext_id_search(client: &TPassClient, ext_id: &str) -> TResult<Option<SearchResult>> {
    let ccode = ext_id.trim().parse::<u64>().map_err(|_| {
        TPassError::Generic("search_one received wrong id type. Only u64 supported".to_string())
    })?;

    let Some(item) = client.get_clients_by_ccode(vec![ccode]).await?.into_iter().next() else {
        return Ok(None);
    };

    //is this correct?
    if item.img_url.is_none() {
        return Err(TPassError::ClientNotFound { ext_id: ccode });
    }

    Ok(Some(SearchResult {
        image: None,
        id: Some(ccode.to_string()),
        details: Some(item),
    }))
}

use super::{RegistrationPair, SearchResult};
impl Remote for TPassClient {
    async fn register_enrollment(&self, reg_pair: &RegistrationPair) -> FRResult<()> {
        //TODO: reconstruct enrollment from old TPass functions.
        let ccode = reg_pair.ext_id.parse::<u64>().map_err(|_| {
            TPassError::Generic(format!(
                "register_enrollment: couldn't parse ext_id to u64: {}",
                reg_pair.ext_id
            ))
        })?;

        let res = self.register_frid(ccode, reg_pair.fr_id.clone()).await?;
        if res["error"] == true {
            return Err(TPassError::RegisterEnrollment { ext_id: ccode, value: res }.into());
        }

        Ok(()) //ok is enough to indicate success.
    }

    async fn unregister_enrollment(&self) -> FRResult<()> {
        warn!("UN-Registering an identity! TBI");
        Ok(())
    }

    //NOTE: we only want single results. but a name search is squirelly.

    ///search a remote source for image and detailed information.
    ///The kind of EnrollData that is received will determine how we perform our search
    ///and how much data we need to get.
    async fn search(&self, enroll_data: &EnrollData) -> FRResult<Vec<SearchResult>> {
        match enroll_data {
            EnrollData { details: Some(det), .. } => {
                match det {
                    EnrollDetails::Min { first_name, last_name, .. } => {
                        //TODO: what about middle
                        info!(
                            "do a name search: received minimum:  first {} and last {}",
                            first_name, last_name
                        );

                        let searcher = SearchBy::Name {
                            first_name: first_name.clone(),
                            last_name: last_name.clone(),
                        };
                        let include_image = enroll_data.image.is_none(); //no image? get one.

                        let res = self.search_one(searcher, include_image).await?;

                        //we really want the whole SearchResult to be None
                        let sr =
                            res.unwrap_or(SearchResult { image: None, details: None, id: None });

                        Ok(vec![sr])

                        //TODO: if search returns nothing, then fail the enrollment, suggest creating
                        //a new profile first.
                    }
                    EnrollDetails::TPass(prof) => {
                        //TODO: this is weird logic to call search if we've already searched
                        //turns out that this means we've already searched. so pass back
                        //Search Results
                        info!("remote search skipped. data requirement satisfied");

                        //let ccode = prof.ccode; //tpd["ccode"].as_u64();

                        let image =
                            enroll_data.image.clone().ok_or_else(|| TPassError::MissingImage {
                                last_name: prof.l_name.clone().unwrap_or("unknown".to_string()),
                                first_name: prof.f_name.clone().unwrap_or("unknown".to_string()),
                                ext_id: prof.ccode.unwrap_or(0),
                                img_url: prof.img_url.clone().unwrap_or("unknown".to_string()),
                            })?;

                        let img_url = prof.img_url.as_ref().map(String::from);

                        let sr = SearchResult {
                            image: Some(Image { bytes: Some(image), url: img_url }),
                            details: Some(prof.clone()),
                            id: prof.ccode.map(|id| id.to_string()),
                        };

                        Ok(vec![sr])

                        //Err(FRError::with_code(1002, "Um, we don't need to search. error because I don't know what to do yet!"))

                        //we could attempt to parse it to a known TPass value....
                        //assume TPass is the full enchilada and we don't need do a search
                        //a search may have already been executed if we have a full profile
                        //do we have a ccode? search by that.
                    }
                }
            }
            _ => {
                //we won't actually return, temp
                Err(TPassError::Generic("search did not receive enough data.".to_string()).into())
            } //get an image if we don't have one, return err if we couldn't get the image
        }

        //based on what we've been given decide what we need to get, if anything.
        //if there's an image, we don't need another one.
        //if there's a  ccode, use the ccode,
        //if there's no ccode but there's a name, do a name search (we only want one, use full name)
    }

    async fn search_one(
        &self,
        search: SearchBy,
        include_img: bool,
    ) -> FRResult<Option<SearchResult>> {
        let mut sr = match search {
            SearchBy::Name { first_name, last_name } => {
                handle_name_search(self, &first_name, &last_name).await?
            }

            SearchBy::ExtID(ext_id) => handle_ext_id_search(self, &ext_id).await?,

            _ => {
                return Err(
                    TPassError::Generic("search_one unsupported search mode".to_string()).into()
                );
            }
        };

        //reaching in to a gooey center
        let url = sr
            .as_ref()
            .and_then(|res| res.details.as_ref())
            .and_then(|res| res.img_url.clone());
        //
        // If we got an image URL and we want the image, download it
        let p_img = match (include_img, url) {
            (true, Some(url_own)) => {
                let image_bytes = self.download_tpass_image(&url_own).await?;
                Some(Image { bytes: Some(image_bytes), url: Some(url_own) })
            }
            _ => None,
        };

        // Apply the downloaded image to the search result if we found one
        if let Some(ref mut result) = sr {
            result.image = p_img;
        }

        Ok(sr)
    }
    ///
    /// search tpass for multiple clients with a batch of ccodes, obviously.
    async fn search_by_ids(
        &self,
        search: SearchBy,
        include_img: bool,
    ) -> FRResult<Vec<SearchResult>> {
        let SearchBy::ExtIDS(ext_ids) = search else {
            return Err(TPassError::Generic("unsupported name search, only id".to_string()).into());
        };

        //NOTE: Deprecate
        // Map the successes and filter out the errors inline
        // This probably not really needed , it's likely that we'll always have a number repre as string, but hey shit happens
        let ccodes: Vec<u64> = ext_ids
            .into_iter()
            .filter_map(|ext_id| match ext_id.trim().parse::<u64>() {
                Ok(code) => Some(code),
                Err(_) => {
                    warn!("search_many skipping non-numeric ext_id '{}' for TPass", ext_id);
                    None
                }
            })
            .collect();

        //TODO: abort and return empty search results, or should this error since no valid ids were passed. leaning worars error
        if ccodes.is_empty() {
            warn!("search_by_ids: ccodes was emtpy. can't search");
            return Ok(vec![]);
        }

        let res = self.get_clients_by_ccode(ccodes).await?;
        let mut out = Vec::with_capacity(res.len());

        for details in res {
            let ccode = details.ccode.as_ref().map(|f| f.to_string());
            // Tuple match: Flattens the nested if/else logic
            let image = match (include_img, details.img_url.clone()) {
                (true, Some(url)) => match self.download_tpass_image(&url).await {
                    Ok(image_bytes) => {
                        Some(Image { bytes: Some(image_bytes), url: Some(url.to_string()) })
                    }
                    Err(err) => {
                        warn!("search_many image download failed for '{}': {}", url, err);
                        None
                    }
                },
                _ => None,
            };

            // details is moved, no cloning required!
            out.push(SearchResult { image, id: ccode, details: Some(details) });
        }

        Ok(out)
    }
}
