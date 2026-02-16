pub mod errors;
pub mod types;
//we may want these up in the backend
use base64::{engine::general_purpose, Engine as _};
use errors::PVApiError;
use futures::stream::{self, StreamExt};
use reqwest::{Client, StatusCode};
use serde_json::Value;
use tracing::{debug, error, info, warn};
use types::{
    AddFaceRequest, AddFaceResponse, CreateIdentitiesRequest, CreateIdentitiesResponse,
    DeleteFaceRequest, DeleteFaceResponse, DeleteIdentitiesRequest, Embedding, Face,
    GetFacesRequest, GetFacesResponse, GetIdentitiesRequest, GetIdentityRequest, Identities,
    Identity, LookupIdentities, LookupRequest, LookupResponse, ProcessFullImageRequest,
    ProcessImageResponse,
};

//hello
const PARALLEL_REQUESTS: usize = 16;

#[derive(Clone)]
pub struct PVApi {
    proc_url: String,
    ident_url: String,
    client: Client,
}

//these are just aliases to reduce noise.
type PVResult<T> = Result<T, PVApiError>;
type PVResultMany<T> = PVResult<Vec<PVResult<T>>>; //sometimes we collect the results of many calls

impl PVApi {
    //we should build the client in here and keep it around.
    pub fn new(proc_url: String, ident_url: String) -> Self {
        Self {
            proc_url,
            ident_url,
            client: Client::new(),
        }
    }
    //FRResult<HealthCheckResponse>
    pub async fn proc_health_check(&self) -> PVResult<Value> {
        let url = format!("{}/health_check", self.proc_url);
        debug!("proc health URL: {}", &url);
        let val = self.client.get(url).send().await?.json().await; //http error return.. map_err i think
        self.maybe_api_error(val)

        // if res.status().is_success() {

        //     let jresult = res.json::<HealthCheckResponse>().await?;
        //     Ok(jresult)
        // } else {
        //     let pv_err = res.json::<PVError>().await?;
        //     Err(FRError::ApiError(pv_err))
        // }
    }

    pub async fn ident_health_check(&self) -> PVResult<Value> {
        let url = format!("{}/health_check", self.ident_url);
        debug!("ident health URL: {}", &url);
        let val = self.client.get(url).send().await?.json().await; //http error return.. map_err i think
        self.maybe_api_error(val)
    }

    pub async fn get_identities(&self, req: Option<GetIdentitiesRequest>) -> PVResult<Identities> {
        let url = format!("{}/identities", self.ident_url);

        let hreq = match req {
            None => self.client.get(url),
            Some(mut ip) => {
                ip.page_token = ip.page_token.map(|pt| general_purpose::STANDARD.encode(pt));
                self.client.get(url).query(&ip)
            }
        };

        let idents = hreq.send().await?.json().await;
        self.maybe_api_error(idents)
            .and_then(|v| serde_json::from_value::<Identities>(v).map_err(PVApiError::from))
    }

    // pub async fn get_identity(&mut self, req: GetIdentityRequest) -> PVResult<Identity> {

    //     let url = format!("{}/identity", self.ident_url);
    //     println!("getting pv identity {}", &url);

    //     let val = self.client.get(url).query(&[("id", req.fr_id)]).send().await?.json().await;
    //     //TODO: what if return was ok but no identity was matched
    //     self.maybe_api_error(val).map(|v| serde_json::from_value::<Identity>(v).expect("could not parse identity"))

    // }

    pub async fn get_identity(&self, req: GetIdentityRequest) -> PVResult<Identity> {
        let url = format!("{}/identity", self.ident_url);
        debug!("getting pv identity {}", &url);

        let val = self
            .client
            .get(url)
            .query(&[("id", req.fr_id)])
            .send()
            .await?
            .json()
            .await;
        //TODO: what if return was ok but no identity was matched
        self.maybe_api_error(val)
            .and_then(|v| serde_json::from_value::<Identity>(v).map_err(PVApiError::from))
    }

    pub async fn create_identities(
        &self,
        req: CreateIdentitiesRequest,
    ) -> PVResult<CreateIdentitiesResponse> {
        let url = format!("{}/identities", self.ident_url);
        debug!("creating pv identities {}", &url);
        let val = self.client.post(url).json(&req).send().await?.json().await;
        self.maybe_api_error(val).and_then(|v| {
            serde_json::from_value::<CreateIdentitiesResponse>(v).map_err(PVApiError::from)
        })
    }

    //--- Additional Face methods
    //Adds another face to an existing enrollment.
    pub async fn add_face(&self, req: AddFaceRequest) -> PVResult<AddFaceResponse> {
        let url = format!("{}/faces", self.ident_url);
        info!("{}", &url);

        let val = self.client.post(&url).json(&req).send().await?.json().await;
        info!("add face rep");
        self.maybe_api_error(val)
            .and_then(|v| serde_json::from_value::<AddFaceResponse>(v).map_err(PVApiError::from))
    }

    pub async fn delete_face(&self, req: &DeleteFaceRequest) -> PVResult<DeleteFaceResponse> {
        let url = format!("{}/faces", self.ident_url);

        let val = self
            .client
            .delete(&url)
            .query(&[
                ("identity_id", &req.fr_id.as_str()),
                ("face_ids", &req.face_id.as_str()),
            ])
            .send()
            .await?
            .json()
            .await;

        info!("delete face resp");
        info!("{:?}", &val);

        self.maybe_api_error(val)
            .and_then(|v| serde_json::from_value::<DeleteFaceResponse>(v).map_err(PVApiError::from))
    }

    pub async fn get_faces(&self, req: GetFacesRequest) -> PVResult<GetFacesResponse> {
        let url = format!("{}/faces", self.ident_url);
        info!("getting faces {}", &url);
        let val = self
            .client
            .get(url)
            .query(&[("identity_id", req.fr_id)])
            .send()
            .await?
            .json()
            .await;

        self.maybe_api_error(val)
            .and_then(|v| serde_json::from_value::<GetFacesResponse>(v).map_err(PVApiError::from))
    }

    //this will provide us with embeddings
    //pub async fn process_image(&self, req: ProcessFullImageRequest) -> PVResult<ProcessImageResponse> {
    pub async fn process_image<I: Into<ProcessFullImageRequest>>(
        &self,
        req: I,
    ) -> PVResult<ProcessImageResponse> {
        //reqw.outputs determines if embeddings are returned.. and some other data.
        let url = format!("{}/process_full_image", self.proc_url);
        //println!("process_image URL: {}", &url);
        let val = self
            .client
            .post(url)
            .json(&req.into())
            .send()
            .await?
            .json()
            .await;

        self.maybe_api_error(val).and_then(|v| {
            serde_json::from_value::<ProcessImageResponse>(v).map_err(PVApiError::from)
        })
    }

    ///Delete identities deletes some or all of the entries in the paravision service.
    pub async fn delete_identities(
        &self,
        fr_ids: Option<DeleteIdentitiesRequest>,
    ) -> PVResultMany<String> {
        let url = format!("{}/identities", self.ident_url);
        debug!("delete identities {}", &url);

        //if we do not explicitly pass in a list of ids, delete them all
        let ids = match fr_ids {
            None => {
                warn!("Delete them alll hahahah");
                //a default setting for tests
                let req = Some(GetIdentitiesRequest {
                    page_size: 100000,
                    page_token: Some("".to_string()),
                    group_ids: None,
                });

                //TODO: convert value to identities.
                //this is why using an actual type is good.
                self.get_identities(req)
                    .await?
                    .identities
                    .into_iter()
                    .map(|i| i.id)
                    .collect()
            }
            Some(ids) => {
                debug!("deleting selected identities");
                ids.fr_ids
                //ids.clone()
            }
        };

        debug!("pending delete count: {}", &ids.len());

        let futures = stream::iter(ids)
            .map(|id| {
                let client = self.client.clone();
                let url = url.clone();
                async move {
                    //we only get a rows_affected result but we want to pass the id
                    let res = client
                        .delete(url)
                        .query(&[("ids", &id)])
                        .send()
                        .await?
                        .json()
                        .await;
                    PVApi::maybe_api_error2(res).map(|_| id) //we only want id
                }
            })
            .buffer_unordered(PARALLEL_REQUESTS);

        let vec_res = futures
            .fold(
                Vec::new(),
                |mut acc, api_res: PVResult<String>| async move {
                    debug!("{:?}", &api_res);
                    acc.push(api_res);
                    acc
                },
            )
            .await;

        Ok(vec_res)
        //TODO: create a proper type or json struct for deletion
        //Ok(json!({ "total": del_count }))
    }

    pub async fn lookup_single(&self, embedding: Embedding) -> PVResult<LookupIdentities> {
        let url = format!("{}/lookup", self.ident_url);

        let look_req = LookupRequest {
            faces: None,
            embeddings: vec![embedding],
            limit: 1,
        };

        let res = self
            .client
            .post(&url)
            .json(&look_req)
            .send()
            .await?
            .json()
            .await;
        self.maybe_api_error(res)
            .and_then(|v| serde_json::from_value::<LookupIdentities>(v).map_err(PVApiError::from))
    }
    //pub async fn lookup<I: Into<LookupRequest>>(&mut self, req: I) -> FRResult<Vec<LookupResponse>> {
    //pub async fn lookup(&mut self, req: LookupRequest) -> PVResult<Vec<LookupResponse>> {
    pub async fn lookup<I: Into<LookupRequest>>(&self, req: I) -> PVResultMany<LookupResponse> {
        let req: LookupRequest = req.into();
        let url = format!("{}/lookup", self.ident_url);

        //TODO: consider if this could faile with no faces.
        // //there will always be faces because we sent embeddings and where there embeddings, ther are faces.
        let faces = req
            .faces
            .ok_or_else(|| PVApiError::with_code(500, "lookup: There were no faces provided"))?;
        let limit = req.limit;

        //marty! it's your kids.
        let futures = stream::iter(faces)
            .map(|face| {
                let client = self.client.clone();
                let url = url.clone();

                async move {
                    let face = face.clone();
                    let embedding = face.embedding.clone().ok_or_else(|| {
                        PVApiError::with_code(500, "lookup: face was provided without an embedding")
                    })?;

                    //breaking the original LookupRequest into a Request per face
                    let look_req = LookupRequest {
                        faces: None,
                        embeddings: vec![Embedding { embedding }],
                        limit,
                    };

                    let res = client.post(&url).json(&look_req).send().await?.json().await;
                    PVApi::maybe_api_error2(res).and_then(|jr| {
                        let ids = serde_json::from_value::<LookupIdentities>(jr)
                            .map_err(PVApiError::from)?;

                        Ok(LookupResponse {
                            face: Face {
                                embedding: None,
                                ..face
                            },
                            identities: ids,
                        })
                    })
                }
            })
            .buffer_unordered(PARALLEL_REQUESTS);

        //awaiting an iterating over our tasks as they become available.
        let res = futures
            .fold(
                Vec::new(),
                |mut acc, api_res: PVResult<LookupResponse>| async {
                    acc.push(api_res);
                    acc
                },
            )
            .await;

        Ok(res)
    }

    //helpers -----------

    ///This helper function helps us determine between 3 possible outcomes from an API server
    /// 1. HTTP OK with the expected results
    /// 2. HTTP OK with an api error as the payload
    /// 3. HTTP ERR indicating a problem with the api call or server.
    ///
    /// We pass in the Result type from our reqwest call and figure out what should be returned.
    /// All or most of our api calls will use this convenience function
    fn maybe_api_error(&self, val: Result<Value, reqwest::Error>) -> PVResult<Value> {
        Self::maybe_api_error_with_context(val, "maybe_api_error")
    }

    fn maybe_api_error2(val: Result<Value, reqwest::Error>) -> PVResult<Value> {
        Self::maybe_api_error_with_context(val, "maybe_api_error2")
    }

    fn maybe_api_error_with_context(
        val: Result<Value, reqwest::Error>,
        boundary: &str,
    ) -> PVResult<Value> {
        match val {
            Ok(val) => {
                //NOTE: not sure this error string will be present.
                //if val.get("error").is_some() {
                if val.get("code").is_some() {
                    error!("{}: paravision payload included error object", boundary);
                    error!("====== {} ERROR Boundary =========", boundary);
                    error!("code: {:?}", val.get("code"));
                    error!("details: {:?}", val.get("details"));

                    if let Some(msg) = val.get("message").and_then(|v| v.as_str()) {
                        error!("message{:?}", PVApi::get_message_slice(msg, 100));
                    } else {
                        error!("no value for message");
                    }
                    error!("====== ERROR Boundary ==========");
                    match serde_json::from_value::<PVApiError>(val) {
                        Ok(e) => Err(e),
                        Err(e) => Err(PVApiError::from(e)),
                    }
                } else {
                    Ok(val)
                }
            }
            Err(e) => {
                let err_c = e
                    .status()
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
                    .as_u16();

                error!("{}: reqwest error while calling paravision", boundary);

                Err(PVApiError {
                    code: err_c,
                    message: e.to_string(),
                    details: None,
                })
            }
        }
        //index into res and see if api based /error exists
    }

    fn get_message_slice(input_str: &str, len: usize) -> String {
        // Specify the start and end character indices
        let start_char_index = 0;
        let end_char_index = len;

        // Collect the chars in the specified range
        let result: String = input_str
            .chars()
            .skip(start_char_index)
            .take(end_char_index - start_char_index)
            .collect();

        result
    }
}
