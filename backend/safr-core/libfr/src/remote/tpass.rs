use crate::{
    remote::Remote, EnrollData, EnrollDetails, FRError, FRResult, IDKind, Image, SearchBy,
};

use libtpass::api::TPassClient;
use serde_json::{json, Value};
use tracing::{error, info, warn};

use super::{RegistrationPair, SearchResult};
impl Remote for TPassClient {
    async fn register_enrollment(&self, reg_pair: &RegistrationPair) -> FRResult<()> {
        //TODO: reconstruct enrollment from old TPass functions.
        let ccode = reg_pair.ext_id.parse::<u64>().map_err(|e| {
            FRError::with_details(
                1081,
                "Could not parse ext_id for TPass registration",
                json!({
                    "ext_id": reg_pair.ext_id,
                    "error": e.to_string(),
                }),
            )
        })?;

        let res = self.register_frid(ccode, reg_pair.fr_id.clone()).await?;
        if res["error"] == true {
            error!("TPASS returned a Registration Error!");
            return Err(FRError::with_details(
                1080,
                "Couldn't register enrollment with Remote",
                res,
            ));
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
                        info!("received minimum:  first {} and last {}", first_name, last_name);
                        info!("do a name search to get tpass details, ccode especially");

                        let searcher = SearchBy::Name {
                            first_name: first_name.clone(),
                            last_name: last_name.clone(),
                        };
                        let include_image = enroll_data.image.is_none(); //no image? get one.

                        let res = self.search_one(searcher, include_image).await?;

                        //we really want the whole SearchResult to be None
                        let sr =
                            res.unwrap_or(SearchResult { image: None, details: None, id: None });

                        info!("searh results here");
                        info!("{:?}", sr);

                        Ok(vec![sr])

                        //TODO: if search returns nothing, then fail the enrollment, suggest creating
                        //a new profile first.
                    }
                    EnrollDetails::TPass(tpd) => {
                        //turns out that this means we've already searched. so pass back
                        //Search Results
                        info!("we have enough detail already, no search needed. this would be new anyway");

                        let ccode = tpd["ccode"].as_u64();

                        let image = enroll_data.image.clone().ok_or_else(|| {
                            FRError::with_code(
                                1003,
                                "TPass enrollment details were provided without an image",
                            )
                        })?;

                        let sr = SearchResult {
                            image: Some(Image::Binary(image)),
                            details: Some(tpd.clone()),
                            id: ccode.map(|id| id.to_string()),
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
                Err(FRError::with_code(
                    1001,
                    "remote.search did not have enough data to perform a search",
                ))
            } //get an image if we don't have one, return err if we couldn't get the image
        }

        //based on what we've been given decide what we need to get, if anything.
        //if there's an image, we don't need another one.
        //if there's a  ccode, use the ccode,
        //if there's no ccode but there's a name, do a name search (we only want one, use full name)
    }

    ///For when we really only want a single result.,
    async fn search_one(
        &self,
        search: SearchBy,
        include_img: bool,
    ) -> FRResult<Option<SearchResult>> {
        let mut url: Option<String> = None;

        let sr = match search {
            SearchBy::Name { first_name, last_name } => {
                //do a name search
                let full_name = format!("{},{}", last_name, first_name);
                //TODO: what are the conditions under which we would need to do a name search rather than an ext_id (ccode)
                //search.
                let sr = match self.search_by_name(&full_name).await?.first() {
                    Some(item) => {
                        let x = item["imgUrl"].as_str().ok_or_else(|| {
                            FRError::with_code(1002, "name search returned profile without imgUrl")
                        })?;
                        url = Some(x.to_string());

                        Some(SearchResult {
                            image: None,
                            id: item["ccode"].as_u64().map(|id| id.to_string()),
                            details: Some(item.clone()), //FIXME: avoid cloning if possible
                        })
                    }
                    None => None,
                };

                info!("searh results here");
                info!("{:?}", sr);
                sr
            }
            SearchBy::ExtID(IDKind::Num(ccode)) => {
                let sr = match self.get_clients_by_ccode(vec![ccode]).await?.first() {
                    Some(item) => {
                        let x = item["imgUrl"].as_str().ok_or_else(|| {
                            FRError::with_code(1002, &format!("client with ccode {} doesn't exist or has no imgUrl. can't enroll without an available image", ccode))
                        })?;

                        url = Some(x.to_string());

                        Some(SearchResult {
                            image: None,
                            id: Some(ccode.to_string()),
                            details: Some(item.clone()), //FIXME: avoid cloning if possible
                        })
                    }
                    None => None,
                };

                sr
            }
            _ => {
                return Err(FRError::with_code(
                    1002,
                    "search_one doesn't support provided ID type. ID must be u64",
                ));
            }
        };

        let p_img = if include_img {
            if let Some(url_ref) = url.as_ref() {
                let image_bytes = self.download_tpass_image(url_ref).await?;
                Some(Image::Binary(image_bytes))
            } else {
                None
            }
        } else {
            None
        };

        let nsr = sr.map(|mut r| {
            r.image = p_img;
            r
        });

        Ok(nsr)
    }

    ///search tpass for multiple clients with a batch of ccodes, obviously.
    async fn search_many(
        &self,
        search: SearchBy,
        include_img: bool,
    ) -> FRResult<Vec<SearchResult>> {
        if let SearchBy::ExtIDS(ccodes) = search {
            let mut ncode = vec![];
            for idk in ccodes {
                if let IDKind::Num(code) = idk {
                    ncode.push(code)
                }
            }

            let res = self.get_clients_by_ccode(ncode).await?;

            let mut out = Vec::with_capacity(res.len());
            for details in res {
                let ccode =
                    details.get("ccode").and_then(Value::as_u64).map(|code| code.to_string());

                let image = if include_img {
                    if let Some(url) = details.get("imgUrl").and_then(Value::as_str) {
                        match self.download_tpass_image(url).await {
                            Ok(image_bytes) => Some(Image::Binary(image_bytes)),
                            Err(err) => {
                                warn!("search_many image download failed for '{}': {}", url, err);
                                None
                            }
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                out.push(SearchResult { image, id: ccode, details: Some(details) });
            }

            Ok(out)
        } else {
            return Err(FRError::with_code(
                2000,
                "search_many doesn't currently support name search, only id",
            ));
        }
    }
}
