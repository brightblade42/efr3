use crate::config::TPassConf;
use crate::errors::TPassError;
use crate::tokens::{TPassToken, JWT};
use crate::types::*;
use crate::types::{
    AttendanceKind, AttendanceResponse, AttendanceStatus, CheckState, DeleteProfileRequest,
    EditProfileRequest, KioskRec, LastAttendanceResponse, NewProfileRequest, NewProfileResponse,
    SearchRequest, TPassSearchType,
};
use bytes::Bytes;
use chrono::prelude::*;
use futures::{stream, StreamExt};
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fmt::Write;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

const PARALLEL_REQUESTS: usize = 16;

type TResult<T> = Result<T, TPassError>;
//type FRResult<T> = Result<T, FRError>;
//type JoinFRResult<T> = Result<FRResult<T>, JoinError>;

///if we have a number hiding in a string, dig it out and let it be its true self!
///NOTE: deserializing a json string to a number turns out to be kind of weird. we get a string
///with escaped characters which prevents us from parsing directly to an integer.
///Regex solver and creator of problems!
fn id_to_num_helper(text: &str) -> i64 {
    lazy_static! {
        static ref RE_INT: Regex = Regex::new(r"\d+").expect("regex should compile");
    }

    RE_INT
        .captures(text)
        .and_then(|caps| caps.get(0))
        .and_then(|m| m.as_str().parse::<i64>().ok())
        .unwrap_or(0)
}

///The type from which we interact with TPass api
#[derive(Debug)]
pub struct TPassClient {
    pub client: reqwest::Client,
    pub token: Mutex<Option<TPassToken>>,
    refresh_lock: Mutex<()>,
    pub conf: TPassConf,
}

impl TPassClient {
    pub fn new(conf: Option<TPassConf>) -> Self {
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap_or_else(|e| {
                warn!("failed building tpass http client, using default client: {}", e);
                reqwest::Client::new()
            });

        Self {
            client,
            token: Mutex::new(None),
            refresh_lock: Mutex::new(()),
            conf: conf.unwrap_or_else(TPassConf::from_env), //always the environment
        }
    }

    async fn cached_token(&self) -> Option<String> {
        let token = self.token.lock().await;
        token.as_ref().filter(|tok| !tok.is_expired()).and_then(|tok| tok.token.clone())
    }

    ///get a fresh hot new token from nice, nice hobbitses
    //TODO: use real credentials passed in from ENV i think.
    async fn refresh_token(&self) -> TResult<TPassToken> {
        let mut map = HashMap::new();
        map.insert("username", &self.conf.user); //we'd pass this in somehow
        map.insert("password", &self.conf.pwd);

        let endpoint = format!("{}api/token", self.conf.url);
        debug!("requesting token from {}", endpoint);
        let res = self.client.post(endpoint).json(&map).send().await?;
        let jwt = res.json::<JWT>().await?;

        Ok(TPassToken::from(jwt))
    }

    ///checks that we have a token, gets one if we don't or if the one we've got is  expired
    //TODO: what about a simple retry..1,2,3...times?
    pub async fn verify_token(&self) -> TResult<()> {
        if self.cached_token().await.is_some() {
            return Ok(());
        }

        let _refresh_guard = self.refresh_lock.lock().await;
        if self.cached_token().await.is_some() {
            return Ok(());
        }

        let tok = self.refresh_token().await?;
        let mut token = self.token.lock().await;
        *token = Some(tok);

        Ok(())
    }

    async fn parse_json_value_response(res: reqwest::Response, endpoint: &str) -> TResult<Value> {
        let status = res.status();
        //TODO: find out from Sherwin if this is the success path, i forget.
        if status == StatusCode::NO_CONTENT {
            return Ok(json!([]));
        }

        let txt = res.text().await?;
        if txt.trim().is_empty() {
            return Err(TPassError::GenericError(Box::new(std::io::Error::other(format!(
                "empty response body from {endpoint} ({status})"
            )))));
        }

        let val: Value = serde_json::from_str(txt.as_str())?;
        if !status.is_success() {
            warn!("TPass endpoint {} returned non-success status {}", endpoint, status);
        }

        Ok(val)
    }

    ///creates a minimal profile in tpass that relates to a facial recognition enrollment
    pub async fn create_profile(&self, profile: &NewProfileRequest) -> TResult<NewProfileResponse> {
        let endpoint = format!("{}api/clients", &self.conf.url);
        let ts = self.get_api_token().await?;
        let val = self.client.post(&endpoint).bearer_auth(ts).json(&profile).send().await?;
        let pr = Self::parse_json_value_response(val, &endpoint).await?;
        info!("THE PROF RESP. OY!");

        info!("{:?}", &pr);
        if pr.get("Message").is_some() {
            info!("DID YOU GET THE MESSAGE ?!");
        }

        let res = serde_json::from_value::<NewProfileResponse>(pr)?;
        Ok(res)
    }

    ///Deletes a profile from tpass related to a facial recognition enrollment
    ///Note: TPass returns a lot of data for which we have no use. Maybe ask Sherwin to just
    ///return a Success or Failure.
    ///Interesting: Deleting multiple times returns the same result as if I had never deleted it.
    ///If the record was not there, I get a 200 OK with a JSON formatted .NET exception
    ///with message field: "Message": "Record does not exist.",
    pub async fn delete_profile(&self, profile: DeleteProfileRequest) -> TResult<Value> {
        let endpoint = format!("{}api/clients/delete?ccode={}", &self.conf.url, profile.ccode);
        let ts = self.get_api_token().await?;
        let res = self.client.delete(&endpoint).bearer_auth(ts).send().await?;
        Self::parse_json_value_response(res, &endpoint).await
    }

    async fn delete_profiles_report(&self, ccodes: Vec<i64>) -> TResult<BatchCallResult<Value>> {
        debug!("delete_profiles count: {}", ccodes.len());
        let urls: Vec<String> = ccodes
            .into_iter()
            .map(|ccode| format!("{}api/clients/delete?ccode={}", &self.conf.url, ccode))
            .collect();
        let attempted = urls.len();
        let ts = self.get_api_token().await?;

        let futures = stream::iter(urls)
            .map(|url| {
                let client = self.client.clone(); //bump ref counot
                let ts = ts.clone();

                debug!("delete profile url: {}", &url);

                async move {
                    let result = match client.delete(&url).bearer_auth(ts).send().await {
                        Ok(res) => {
                            debug!("delete profile response status: {}", res.status());
                            TPassClient::parse_json_value_response(res, &url).await
                        }
                        Err(e) => Err(TPassError::from(e)),
                    };
                    (url, result)
                }
            })
            .buffer_unordered(PARALLEL_REQUESTS);

        let responses = futures.collect::<Vec<(String, TResult<Value>)>>().await;

        let mut items: Vec<Value> = Vec::new();
        let mut errors: Vec<BatchCallError> = Vec::new();
        let mut succeeded = 0;

        for (url, res) in responses {
            match res {
                Ok(client) => {
                    succeeded += 1;
                    items.push(client);
                }
                Err(e) => {
                    warn!("delete_profiles call failed for {}: {}", &url, e);
                    errors.push(BatchCallError { target: url, message: e.to_string() });
                }
            }
        }

        Ok(BatchCallResult::new(items, attempted, succeeded, errors))
    }

    pub async fn delete_profiles(&self, ccodes: Vec<i64>) -> TResult<Vec<Value>> {
        let batch = self.delete_profiles_report(ccodes).await?;
        if batch.meta.failed > 0 {
            warn!(
                "delete_profiles completed with partial failures: {}/{} failed",
                batch.meta.failed, batch.meta.attempted
            );
        }
        Ok(batch.items)
    }

    pub async fn edit_profile(&self, profile: EditProfileRequest) -> TResult<Value> {
        let endpoint = format!("{}api/clients/{}", &self.conf.url, profile.ccode);
        let ts = self.get_api_token().await?;
        let res = self.client.put(&endpoint).bearer_auth(ts).json(&profile).send().await?;
        Self::parse_json_value_response(res, &endpoint).await
    }

    ///helper function to reduce the state combinations for attendance by factoring out
    ///AttendanceKind, time checks and letting us simply match over CheckStates.
    ///TPass gives us a lot to consider for determining attendance
    pub fn filter_attendance_state(
        recent: &LastAttendanceResponse,
        att_kind: &AttendanceKind,
    ) -> CheckState {
        //combination of time in and timeout let us know if we can check in or check out.
        let (time_in, time_out) = (&recent.timeIn, &recent.timeOut);

        match att_kind {
            AttendanceKind::In => {
                if time_out.is_some() || time_in.is_none() {
                    CheckState::In
                } else {
                    CheckState::LastKnown
                }
            }
            AttendanceKind::Out => {
                if time_out.is_some() {
                    CheckState::LastKnown
                } else if time_out.is_none() && time_in.is_some() {
                    CheckState::Out
                } else {
                    CheckState::None
                }
            }
        }
    }

    //helper function to record attendance with tpass.
    async fn do_attendance(
        client: &Client,
        tk_url: &str,
        ts: String,
        resp: &LastAttendanceResponse,
    ) -> TResult<AttendanceStatus> {
        let time_stamp = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        let mut kiosk = KioskRec::from(resp);
        kiosk.timeStamp = time_stamp;

        let res: AttendanceResponse =
            client.post(tk_url).json(&kiosk).bearer_auth(ts).send().await?.json().await?;

        //println!("{:?}", &res);
        Ok(AttendanceStatus::from(res))
    }

    // Facial recognition has found matches and we'll use enrollments request that tpass return their details.
    // pub async fn get_clients_from_lookup(&mut self, enrollments: Vec<CoreEnrollment>)
    //     -> TResult<Vec<CoreEnrollment>> {

    //     let ts = self.get_api_token().await?;

    //     let futures = stream::iter(enrollments)
    //         .map(|mut enrollment| {

    //             let client = self.client.clone(); //bump ref counot
    //             let token = ts.clone();
    //             let ccode = enrollment.ccode;
    //             let url = self.conf.url.clone();
    //             tokio::spawn(async move {
    //                 let url = format!("{}api/clients/load?id={}", url, ccode);
    //                 let res = client.get(url).bearer_auth(token).send().await?;
    //                 //println!("{:?}", res); //the stuff
    //                 if res.status() == StatusCode::NO_CONTENT {
    //                     //what will we get if there's nothting to get. should always get something
    //                    //TODO: fix problem where enrollment exists but tpass profile deleted.
    //                     enrollment.details = None;
    //                     Ok(enrollment)
    //                 } else {
    //                     let txt = res.text().await?;
    //                     let vv: Value = serde_json::from_str(&txt)
    //                         .expect("text couldn't be parsed as json value");
    //                     enrollment.details = Some(vv);
    //                     Ok(enrollment)
    //                 }
    //             })
    //         }).buffer_unordered(PARALLEL_REQUESTS);

    //     let client_res = futures.fold(Vec::new(), |mut acc, res: JoinTResult<CoreEnrollment>| async move {
    //         match res {
    //             Ok(Ok(k)) => {
    //                 //if what we have is an empty {} skip it
    //                 acc.push(k);
    //                 acc
    //             }
    //             Ok(Err(e)) => {
    //                 println!("There was an error with the tpass call: {}", e); //a great place to log
    //                 acc
    //             },
    //             Err(e) => {
    //                 println!("There was some kind of tokio shenanigans {}", e); //a great place to log
    //                 acc
    //             }
    //         }
    //     }).await;

    //     Ok(client_res)
    // }

    ///pass in some ccodes and get some TPass Clients. typically we use this to get a singular client
    async fn get_clients_by_ccode_report(
        &self,
        ccodes: Vec<u64>,
    ) -> TResult<BatchCallResult<Value>> {
        let urls: Vec<String> = ccodes
            .into_iter()
            .map(|ccode| format!("{}api/clients/load?id={}", &self.conf.url, ccode))
            .collect();
        let attempted = urls.len();
        let ts = self.get_api_token().await?;

        let futures = stream::iter(urls)
            .map(|url| {
                let client = self.client.clone(); //bump ref counot
                let ts = ts.clone();

                async move {
                    let result = match client.get(&url).bearer_auth(ts).send().await {
                        Ok(res) => TPassClient::parse_json_value_response(res, &url).await,
                        Err(e) => Err(TPassError::from(e)),
                    };
                    (url, result)
                }
            })
            .buffer_unordered(PARALLEL_REQUESTS);

        let responses = futures.collect::<Vec<(String, TResult<Value>)>>().await;

        let mut items: Vec<Value> = Vec::new();
        let mut errors: Vec<BatchCallError> = Vec::new();
        let mut succeeded = 0;

        for (url, res) in responses {
            match res {
                Ok(client) => {
                    succeeded += 1;
                    items.push(client);
                }
                Err(e) => {
                    warn!("get_clients_by_ccode call failed for {}: {}", &url, e);
                    errors.push(BatchCallError { target: url, message: e.to_string() });
                }
            }
        }

        Ok(BatchCallResult::new(items, attempted, succeeded, errors))
    }

    pub async fn get_clients_by_ccode(&self, ccodes: Vec<u64>) -> TResult<Vec<Value>> {
        let batch = self.get_clients_by_ccode_report(ccodes).await?;
        if batch.meta.failed > 0 {
            warn!(
                "get_clients_by_ccode completed with partial failures: {}/{} failed",
                batch.meta.failed, batch.meta.attempted
            );
        }
        Ok(batch.items)
    }

    fn build_search_url(&self, search: &SearchRequest) -> String {
        let mut out = String::new();
        //bashe
        out.push_str(&self.conf.url);
        out.push_str("api/");
        match &search.search_term {
            TPassSearchType::CCode(ccode) => {
                write!(&mut out, "clients/load?id={}", ccode).unwrap();
            }
            TPassSearchType::Name(term) => {
                match search.client_type.as_str() {
                    "Student" => {
                        write!(
                            &mut out,
                            "clients/searchclient?id={}&type={}&compid={}",
                            term, search.client_type, search.comp_id
                        )
                        .unwrap();
                    }
                    "Visitor" => {
                        write!(&mut out, "clients/searchvisitor?&value={}", term).unwrap();
                    }
                    "Personnel" => {
                        write!(
                            &mut out,
                            "clients/searchpersonnel?compid={}&value={}",
                            search.comp_id, term
                        )
                        .unwrap();
                    }
                    _ => {
                        // probably should be an error
                    }
                }
            }
        }
        out
    }

    ///returns the set of urls that we will search over based on comma sep search terms.
    fn parse_search_terms(&self, search: SearchRequest) -> Vec<String> {
        let depth = search.depth.unwrap_or(1);

        let sub_terms: Vec<String> = match &search.search_term {
            //NOTE: make sure we have the single term if there's no comma
            TPassSearchType::Name(term) => term
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect(),

            _ => {
                warn!("search term must be string based");
                return vec![];
            }
        };

        let new_url = |st: &str| -> String {
            let sr = SearchRequest {
                search_term: TPassSearchType::Name(st.to_string()),
                client_type: search.client_type.clone(),
                comp_id: search.comp_id,
                depth: None,
            };

            self.build_search_url(&sr)
        };

        let mut urls = Vec::new();
        for s in sub_terms {
            if depth == 0 || s.len() >= 3 {
                urls.push(new_url(&s));
                continue;
            }

            match depth {
                1 => {
                    for c in b'A'..=b'Z' {
                        let mut t = String::with_capacity(s.len() + 1);
                        t.push_str(&s);
                        t.push(c as char);
                        urls.push(new_url(&t));
                    }
                }
                _ => {
                    //search url count grows exponentially, we cap it but show that were getting
                    //deep search requests.
                    if depth > 2 {
                        warn!(
                            "search depth {} exceeds maximum supported depth (2); expansion is capped",
                                    depth
                                );
                    }
                    for c1 in b'A'..=b'Z' {
                        for c2 in b'A'..=b'Z' {
                            let mut t = String::with_capacity(s.len() + 2);
                            t.push_str(&s);
                            t.push(c1 as char);
                            t.push(c2 as char);
                            urls.push(new_url(&t));
                        }
                    }
                }
            }
        }

        urls
    }

    //full a-z search
    pub async fn search_tpass(&self, search: SearchRequest) -> TResult<BatchCallResult<Value>> {
        //TOOD: get depth from client
        let urls = self.parse_search_terms(search);
        let attempted = urls.len();
        let ts = self.get_api_token().await?;

        let futures = stream::iter(urls)
            .map(|url| {
                let client = self.client.clone();
                let ts = ts.clone();

                async move {
                    let result = match client.get(&url).bearer_auth(ts).send().await {
                        Ok(res) => TPassClient::parse_json_value_response(res, &url).await,
                        Err(e) => Err(TPassError::from(e)),
                    };
                    (url, result)
                }
            })
            .buffer_unordered(PARALLEL_REQUESTS);

        let responses = futures.collect::<Vec<(String, TResult<Value>)>>().await;

        let mut items: Vec<Value> = Vec::new();
        let mut errors: Vec<BatchCallError> = Vec::new();
        let mut succeeded = 0;

        for (url, res) in responses {
            match res {
                Ok(v) => {
                    succeeded += 1;
                    if v.as_array().is_some_and(|arr| !arr.is_empty()) {
                        items.push(v);
                    }
                }
                Err(e) => {
                    warn!("search_tpass call failed for {}: {}", &url, e);
                    errors.push(BatchCallError { target: url, message: e.to_string() });
                }
            }
        }

        Ok(BatchCallResult::new(items, attempted, succeeded, errors))
    }
    //TODO: there doesn't seem to be any error response, I can send anything and it gives me
    //a 200. I'll only know if I get an email or an sms.
    pub async fn send_fr_alert(&self, alert: FRAlert) -> TResult<Value> {
        let endpoint = format!("{}api/notification/sendalert", self.conf.url);
        let ts = self.get_api_token().await?;
        let _res = self.client.post(endpoint).json(&alert).bearer_auth(ts).send().await?;
        Ok(json!({ "message": "alert sent" }))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TPassResults {
    pub details: Vec<Value>,
}

//NOTE: impl block for functions for new version that can directly carry over. Temporary
impl TPassClient {
    pub async fn status_types(&self) -> TResult<Value> {
        let endpoint = format!("{}api/status", self.conf.url);
        let ts = self.get_api_token().await?;
        let res = self.client.get(&endpoint).bearer_auth(ts).send().await?;
        Self::parse_json_value_response(res, &endpoint).await
    }

    pub async fn client_types(&self) -> TResult<Value> {
        let endpoint = format!("{}api/clienttypes/getclienttype", self.conf.url);
        let ts = self.get_api_token().await?;
        let res = self.client.get(&endpoint).bearer_auth(ts).send().await?;
        Self::parse_json_value_response(res, &endpoint).await
    }

    pub async fn get_companies(&self) -> TResult<Value> {
        //call wants role from jwt but server is always admin so we'll just pass admin
        //could parse it out of the token if we need to
        let endpoint = format!("{}api/companies/restricted?groups=admin", self.conf.url);
        let ts = self.get_api_token().await?;
        let res = self.client.get(&endpoint).bearer_auth(ts).send().await?;
        Self::parse_json_value_response(res, &endpoint).await
    }

    ///We need to make sure we have valid auth token to make tpass calls.
    async fn get_api_token(&self) -> TResult<String> {
        if let Some(token) = self.cached_token().await {
            return Ok(token);
        }

        self.verify_token().await?;
        self.cached_token().await.ok_or_else(|| {
            TPassError::GenericError(Box::new(std::io::Error::other(
                "missing API token after verification",
            )))
        })
    }

    ///download a tpass image by their generated url
    pub async fn download_tpass_image(&self, url: &str) -> TResult<Bytes> {
        let bytes = self.client.get(url).send().await?.bytes().await?;
        Ok(bytes)
    }
}

//TPass Replacement Theory
//new impl block for new funs. kinda neat way to keep new ver distinct from old
impl TPassClient {
    ///mark_attendance is a simplified  version of record_attendance. This method deals with a single
    ///identity whereas record_attendance will spawn a task for each identity in a vec.
    //pub async fn mark_attendance(&mut self, details: Value, att_kind: AttendanceKind ) -> TResult<Option<AttendanceStatus>> {
    pub async fn mark_attendance(
        &self,
        idpair: (String, u64),
        att_kind: AttendanceKind,
    ) -> TResult<Option<AttendanceStatus>> {
        let ts = self.get_api_token().await?;

        let date_time = Local::now().format("%Y-%m-%d").to_string();
        //TODO: reject if idnumber doesn't exist
        //let id_string = details["idnumber"].to_string(); //req for checkin and out.
        //let ccode = details["ccode"].as_u64().unwrap(); //should be infallible
        let id_string = idpair.0; //details["idnumber"].to_string(); //req for checkin and out.
        let ccode = idpair.1; //details["ccode"].as_u64().unwrap(); //should be infallible
        let id_num = id_to_num_helper(&id_string);
        debug!(" ****  details ids: ccode: {} idnumber: {}", ccode, id_num);
        let last_known_url = format!(
            "{}api/clients/searchclientwithlogs?id={}&date={}",
            self.conf.url, id_num, date_time
        );
        let tk_url = format!("{}api/timekeeper", self.conf.url);

        let res: Vec<LastAttendanceResponse> =
            self.client.get(last_known_url).bearer_auth(&ts).send().await?.json().await?;

        //NOTE: We actually only recieve a single result for a given id_number even though it is in a vector.
        //There is also nothing preventing TPASS from storing identical id_numbers. This means we could get the wrong thing back.
        //The match code below assumes we receive more than one result for matching id_numbers. This assumption is currently wrong.
        //if we receive more than one result for an id, we need to narrow to the ccode.
        //the rules for idnumber in tpass are very relaxed and very undude
        let att_opt = match res.len() {
            0 => None, //i doubt this will occur, more likely early error return above.
            1 => Some(res[0].clone()),
            _ => res.iter().find(|r| r.ccode == Some(ccode)).cloned(),
        };

        //now that we have recent checkin/out for peeps lets see what we can check in
        //what would be sweet is an is_checked_in property or something, this be weird.
        //I don't got control over this data though.
        let chk = if let Some(att_resp) = att_opt {
            match TPassClient::filter_attendance_state(&att_resp, &att_kind) {
                //trick to reduce the possible match states
                CheckState::In => {
                    let mut status =
                        TPassClient::do_attendance(&self.client, &tk_url, ts.clone(), &att_resp)
                            .await?;
                    status.kind = AttendanceKind::In;
                    Some(status)
                }
                CheckState::Out => {
                    let mut status =
                        TPassClient::do_attendance(&self.client, &tk_url, ts.clone(), &att_resp)
                            .await?;
                    status.kind = AttendanceKind::Out;
                    Some(status)
                }
                CheckState::LastKnown => {
                    //println!("RETURN LAST KNOWN ");
                    Some(AttendanceStatus::from(att_resp))
                }
                CheckState::None => {
                    info!("mark_attendance: Client has not been to the building today");
                    None
                }
            }
        } else {
            //don't do anything, already checked in.
            None
        };

        Ok(chk)
    }

    ///The older more complicated way. This may not be needed anymore but keeping around
    pub async fn record_attendance(
        &self,
        enrollments: Vec<Value>,
        att_kind: AttendanceKind,
    ) -> TResult<Vec<AttendanceStatus>> {
        let ts = self.get_api_token().await?;
        let futures = stream::iter(enrollments)
            .map(|details| {
                let client = self.client.clone();
                let ts = ts.clone();
                let date_time = Local::now().format("%Y-%m-%d").to_string();
                let id_string = details["idnumber"].to_string(); //req for checkin and out.
                let id_num = id_to_num_helper(&id_string);
                let ccode = details["ccode"].as_u64();
                let last_known_url = format!(
                    "{}api/clients/searchclientwithlogs?id={}&date={}",
                    self.conf.url, id_num, date_time
                );
                let tk_url = format!("{}api/timekeeper", self.conf.url);
                let att_type = att_kind.clone();

                async move {
                    let Some(ccode) = ccode else {
                        warn!("record_attendance detail missing ccode");
                        return Ok(None);
                    };

                    let res: Vec<LastAttendanceResponse> =
                        client.get(last_known_url).bearer_auth(&ts).send().await?.json().await?;

                    //if we receive more than one result for an id, we need to narrow to the ccode.
                    //the rules for idnumber in tpass are very relaxed and very undude
                    let att_opt = match res.len() {
                        0 => None, //i doubt this will occur, more likely early error return above.
                        1 => Some(res[0].clone()),
                        _ => res.iter().find(|r| r.ccode == Some(ccode)).cloned(),
                    };

                    //now that we have recent checkin/out for peeps lets see what we can check in
                    //what would be sweet is an is_checked_in property or something, this be weird.
                    //I don't got control over this data though.
                    let chk = if let Some(att_resp) = att_opt {
                        match TPassClient::filter_attendance_state(&att_resp, &att_type) {
                            //trick to reduce the possible match states
                            CheckState::In => {
                                let mut status = TPassClient::do_attendance(
                                    &client,
                                    &tk_url,
                                    ts.clone(),
                                    &att_resp,
                                )
                                .await?;
                                status.kind = AttendanceKind::In;
                                Some(status)
                            }
                            CheckState::Out => {
                                let mut status = TPassClient::do_attendance(
                                    &client,
                                    &tk_url,
                                    ts.clone(),
                                    &att_resp,
                                )
                                .await?;
                                status.kind = AttendanceKind::Out;
                                Some(status)
                            }
                            CheckState::LastKnown => {
                                //println!("RETURN LAST KNOWN ");
                                Some(AttendanceStatus::from(att_resp))
                            }
                            CheckState::None => {
                                info!("Client has not been to the building today");
                                None
                            }
                        }
                    } else {
                        //don't do anything, already checked in.
                        None
                    };

                    Ok(chk)
                }
            })
            .buffer_unordered(PARALLEL_REQUESTS);

        let client_res = futures
            .fold(
                Vec::new(),
                |mut acc: Vec<AttendanceStatus>, res: TResult<Option<AttendanceStatus>>| async move {
                    match res {
                        Ok(resp) => {
                            if let Some(status) = resp {
                                acc.push(status);
                            }
                        }
                        Err(e) => {
                            //TODO: log failed attendance to db for logging
                            warn!("record_attendance call failed: {}", e);
                        }
                    }
                    acc
                },
            )
            .await;

        Ok(client_res)
    }

    //we need to pass in some details fr_id, ccode
    pub async fn register_frid(&self, ccode: u64, fr_id: String) -> TResult<Value> {
        let tk = self.get_api_token().await?;
        //paravision endpoint is technically no longer accurate but that's not up to us.
        let url = format!("{}api/paravision/updatepvid", self.conf.url);

        let reg_info = json!({ "CCode": ccode, "ID": fr_id });
        let res = self.client.put(&url).bearer_auth(&tk).json(&reg_info).send().await?;

        let reg_val = Self::parse_json_value_response(res, &url).await?;

        info!("REG: ccode: {} fr_id: {}", &reg_val["cCode"], &reg_val["id"]);
        //this is weird because we are going to return ok, even if we receive an error here.
        //when we fix our TPassError debacle we'll revisit this with a better solution.

        Ok(reg_val)
        //Ok(json!({"msg" : "champion!"}))
    }

    //rethink errors here
    ///searches tpass based on a given name in last, first format
    ///returns a Vec<Value> because it's possible to send a partial name which could result in multiple
    ///results. This also searches All statuses.. The same person may be in the system more than once,.
    ///I don't know if this is goood but it exists. Since I can't know which is the real Slim Shady,
    pub async fn search_by_name(&self, full_name: &str) -> TResult<Vec<Value>> {
        //build the url.
        let tk = self.get_api_token().await?;
        let status_type = "All"; //case sensitive
                                 //NOTE: docs say to set compid to null for all companies search, does null mean leave out?
        let endpoint = format!(
            "{}api/clients/searchclient?id={}&type={}",
            self.conf.url, full_name, status_type
        );
        info!("{}", &endpoint);

        let resp = self.client.get(endpoint).bearer_auth(tk).send().await?;

        //NOTE: we should check among a known set of non 200 codes to determine what to return
        if resp.status() == StatusCode::NO_CONTENT {
            info!("search_by_name for: {}  returned no results", full_name);
            return Ok(Vec::new()); //we return empty, No content isn't an error
        }

        let res: Value = resp.json().await?;

        let tpr: Result<Vec<Value>, serde_json::Error> = serde_json::from_value(res);

        if let Ok(t_vec) = tpr {
            return Ok(t_vec);
            //return Ok(TPassResults {details: t});
        }

        //Ok(TPassResults {details: Vec::new()})
        Ok(Vec::new())
    }
}
