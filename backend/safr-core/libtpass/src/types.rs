use serde::{Deserialize, Serialize};

//maybe type. This may be overkill for things we're just passing along to TPass.
#[derive(Serialize, Deserialize, Debug)]
pub struct FRAlert {
    #[serde(rename = "Type")]
    #[serde(default = "default_resource")]
    pub typ: String, //Visitor client type (required)
    #[serde(rename = "CompId")]
    pub compid: u64,
    #[serde(rename = "PInfo")]
    pub pinfo: u64, //ccode
    #[serde(rename = "Image")]
    pub image: Option<String>, // base64string of the photo. This is optional
}

fn default_resource() -> String {
    "FR Alert".to_string()
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EditProfileRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compId: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sndCompId: Option<u64>,

    pub ccode: u64,
    pub clntTid: u64, //Visitor client type (required)
    pub sttsId: u32,  //-- Active status (required)
    #[serde(rename = "base64Image")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>, // base64string of the photo. This is optional
    pub fName: String, //-- First name (required)
    pub lName: String, // Last name (required)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idnumber: Option<String>, //idnumber is a string. lol
    #[serde(rename = "street1")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zipcode: Option<String>,
}

//base our edit profile from a new one
//TODO: check if json NewProfileResponse contains comp_id
impl From<&NewProfileResponse> for EditProfileRequest {
    fn from(prof: &NewProfileResponse) -> Self {
        Self {
            compId: prof.compId,
            sndCompId: None,
            ccode: prof.ccode,
            image: None,
            idnumber: Some(prof.ccode.to_string()),
            clntTid: prof.clntTid,
            sttsId: prof.sttsId,
            fName: prof.fName.clone(),
            lName: prof.lName.clone(),
            street: prof.street1.clone(),
            state: prof.state.clone(),
            zipcode: None,
        }
    }
}

//was called NewClient
#[derive(Serialize, Deserialize, Debug)]
pub struct NewProfileRequest {
    //#[serde(skip_serializing_if = "Option::is_none")]
    //pub compId: Option<u64>,
    pub compId: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sndCompId: Option<u64>,
    pub clntTid: u64,  //Visitor client type (required)
    pub sttsId: u64,   //-- Active status (required)
    pub fName: String, //-- First name (required)
    pub lName: String, // Last name (required)
    #[serde(rename = "base64Image")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>, // base64string of the photo. This is optional
    //-- Below are the address information which are all optional
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typ: Option<String>,
    #[serde(rename = "street1")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zipcode: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteProfileRequest {
    pub ccode: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pv_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegisterEnrollmentRequest {
    #[serde(rename = "CCode")]
    pub ccode: u64,
    #[serde(rename = "ID")]
    pub fr_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewProfileResponse {
    #[serde(skip_serializing)]
    pub amPkId: u32,
    #[serde(skip_serializing)]
    pub aptmnId: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compId: Option<u64>,
    pub ccode: u64,
    pub clntTid: u64,
    pub sttsId: u32,
    pub fName: String,
    pub lName: String,
    //#[serde(skip_serializing_if = "Option::is_none")]
    #[serde(skip_serializing)]
    pub base64Image: Option<String>,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typ: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zipcode: Option<String>,
    #[serde(skip_serializing)]
    pub photoFilename: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KioskRec {
    pub logSource: String, //maybe create enum //from searchclient with logs result.
    pub logID: i32,        //from search client with logs result.
    pub compId: i32,       //school to checkin
    //pub ccode: i64,        //person's unique id from searchclient with logs
    pub ccode: u64,     //person's unique id from searchclient with logs
    pub vstPid: String, //how do I represent json null value when empty?
    pub brCode: String,
    pub timeStamp: String,
    pub status: String, //useless till end
    pub tempBadge: bool,
    pub tardy: bool,
}
impl From<&LastAttendanceResponse> for KioskRec {
    fn from(last_known: &LastAttendanceResponse) -> Self {
        Self {
            logSource: last_known.logSource.clone().unwrap_or("None".to_string()),
            logID: last_known.logID.unwrap_or(0),
            compId: last_known.compId.unwrap_or(0),
            ccode: last_known.ccode.unwrap_or(0),
            vstPid: "".to_string(),
            brCode: "".to_string(),
            timeStamp: "".to_string(), //this is not great
            status: "".to_string(),
            tempBadge: false,
            tardy: false,
        }
    }
}
impl KioskRec {
    pub fn new() -> Self {
        Self {
            logSource: "NONE".to_string(),
            logID: 0,
            compId: 0,
            ccode: 0,
            vstPid: "".to_string(),
            brCode: "".to_string(),
            timeStamp: "".to_string(), //this is not great
            status: "".to_string(),
            tempBadge: false,
            tardy: false,
        }
    }
}

impl Default for KioskRec {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum TPassSearchType {
    CCode(u64),
    Name(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SearchRequest {
    pub search_term: TPassSearchType,
    #[serde(default)]
    pub comp_id: u64,
    #[serde(default = "default_client_type")]
    pub client_type: String,
    pub depth: Option<u8>,
}

fn default_client_type() -> String {
    "Visitor".to_string()
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RecentCheckinRequest {
    //pub compId: u32,
    //pub ccode: u64,
    pub id_number: String,
    pub date_time: String,
}
//also covers checkout.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LastAttendanceResponse {
    pub actId: Option<i32>,
    pub address: Option<String>,
    pub amPkId: Option<i32>,
    pub aptmnId: Option<i32>,
    pub canSgnOut: Option<bool>,
    pub canUseKiosk: Option<bool>,
    //pub ccode: Option<i64>,
    pub ccode: Option<u64>,
    pub city: Option<String>,
    pub clntTid: Option<i32>,
    pub cntryIso: Option<String>,
    pub compId: Option<i32>,
    pub company: Option<String>,
    pub fName: Option<String>,

    #[serde(default = "default_idnum")]
    pub idnumber: String,
    pub imgUrl: Option<String>,
    pub lName: Option<String>,
    pub logCompID: Option<i32>,
    pub logID: Option<i32>,
    pub logSource: Option<String>,
    pub mName: Option<String>,
    pub name: Option<String>,
    pub remarks: Option<String>,
    pub sndCompId: Option<i32>,
    pub state: Option<String>,
    pub status: Option<String>,
    pub street1: Option<String>,
    pub sttsId: Option<i32>,
    pub timeIn: Option<String>,
    pub timeOut: Option<String>,
    #[serde(rename = "type")]
    pub typ: Option<String>,
    pub zipcode: Option<String>,
}

fn default_idnum() -> String {
    "".to_string()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AttendanceResponse {
    #[serde(skip_serializing)]
    pub brCode: Option<String>,
    #[serde(skip_serializing)]
    //pub ccode: i64,
    pub ccode: u64,
    #[serde(skip_serializing)]
    pub compId: i32,
    #[serde(skip_serializing)]
    pub healthFlag: bool,
    #[serde(skip_serializing)]
    pub logID: i32,
    #[serde(skip_serializing)]
    pub logSource: Option<String>,
    pub status: Option<String>,
    pub tardy: bool,
    #[serde(skip_serializing)]
    pub tempBadge: bool,
    pub timeStamp: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AttendanceKind {
    In,
    Out,
}
//a more concise type for showing current attendance status
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AttendanceStatus {
    #[serde(skip_serializing)]
    //pub ccode: i64,
    pub ccode: u64,
    pub time_stamp: Option<String>,
    pub tardy: bool,
    pub kind: AttendanceKind,
}

impl From<AttendanceResponse> for AttendanceStatus {
    fn from(resp: AttendanceResponse) -> Self {
        //logSource is what lets us know if we got a checkin or checkout
        //resp.

        Self {
            ccode: resp.ccode,
            time_stamp: resp.timeStamp,
            tardy: resp.tardy,
            kind: AttendanceKind::In,
        }
    }
}

impl From<LastAttendanceResponse> for AttendanceStatus {
    fn from(resp: LastAttendanceResponse) -> Self {
        //we know if the last thing a person did was checkout if there's a timeout present.
        let (time_stamp, kind) = match resp.timeOut {
            Some(t_out) => (Some(t_out), AttendanceKind::Out),
            _ => (resp.timeIn, AttendanceKind::In),
        };

        Self {
            ccode: resp.ccode.unwrap_or(0),
            time_stamp,
            tardy: false, //maybe this shoudl be an option
            kind,
        }
    }
}

pub enum CheckState {
    In,
    Out,
    LastKnown,
    None,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BatchCallError {
    pub target: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BatchCallMeta {
    pub attempted: usize,
    pub succeeded: usize,
    pub failed: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BatchCallResult<T> {
    pub items: Vec<T>,
    pub meta: BatchCallMeta,
    pub errors: Vec<BatchCallError>,
}

impl<T> BatchCallResult<T> {
    pub fn new(
        items: Vec<T>,
        attempted: usize,
        succeeded: usize,
        errors: Vec<BatchCallError>,
    ) -> Self {
        let failed = errors.len();
        Self { items, meta: BatchCallMeta { attempted, succeeded, failed }, errors }
    }
}
