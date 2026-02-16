//use serde::{Deserialize, Serialize};
//TODO::types are kind of scattered
//use libpv::types::{AddFaceResponse, Face, GetFacesResponse, FaceInfo, ProcessFullImageRequest};
//use crate::errors::TPassError;
//use crate::types::*;
//V1 types exist so that fr service can be updated without breakaing existing TPASS installs.

// #[derive(Serialize, Deserialize, Debug)]
// pub struct AddFaceResponseV1 {
//     pub face_id: String,
//     pub fr_id: String
// }

/*
impl From<AddFaceResponse> for AddFaceResponseV1 {
    fn from(resp: AddFaceResponse) -> Self {

        let face: FaceInfo = resp.faces[0].clone();
        Self {
            face_id: face.id,
            fr_id:  "".to_string()
        }
    }
}*/

// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub struct FaceInfoV1 {
//     created_at: String,
//     id: String
// }

// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub struct GetFacesResponseV1 {
//    pub created_at: String,
//    pub id: String,
//    pub faces: Vec<FaceInfoV1>,
// }

// impl From<GetFacesResponse> for GetFacesResponseV1 {
//     fn from(resp: GetFacesResponse) -> Self {
//         let mut info_v1 = vec![];
//         for face in resp.faces {
//             info_v1.push(FaceInfoV1 {
//                 created_at: face.created_at,
//                 id: face.id
//             });
//         }

//         Self {
//                id: "".to_string(),
//                created_at: "".to_string(),
//                faces: info_v1
//         }
//     }
// }

//A set of options to customize what we want from an image recognition
//
//We can choose the top number of possible matches to return for each recognized face.
//We can include the base64 data for each face that was cropped out of the image frame and
//used for recognition. This could be used for some kind of display
// #[derive(Serialize, Deserialize, Debug)]
// pub struct RecognizeOptions {
//     #[serde(default = "default_top_matches")]
//     pub top_matches: i32,
//     #[serde(default = "default_include_detected_faces")]
//     pub include_detected_faces: bool, //the face images extracted from the frame.
//     #[serde(default = "default_on_match")]
//     pub on_match: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub min_match: Option<f64>,
// }
// impl Default for RecognizeOptions {
//     fn default() -> Self {
//         RecognizeOptions {
//             top_matches: 1,
//             include_detected_faces: true,
//             on_match: Some("identify".to_string()),
//             min_match: Some(0.95)
//         }
//     }
// }

// fn default_top_matches() -> i32 { 2 }
// fn default_include_detected_faces() -> bool {
//     false
// }
// fn default_on_match() -> Option<String> {
//     Some("identify".to_string())
// }

//TODO: figure out where the best place for From<T> implementations should go.
//IS it confusing to have this hanging out here or is it just that I'm not used to this sort of thing
// impl From<&NewProfileRequest> for ProcessFullImageRequest {
//     fn from(pr: &NewProfileRequest) -> Self {
//         let img = match pr.image.as_ref() {
//             Some(x) => x.clone(),
//             None => "".to_string()
//         } ;

//         ProcessFullImageRequest {
//             image: img,
//             outputs: Some(vec![
//                 "EMBEDDING".to_string(),
//                 "QUALITY".to_string(),
//                 "MASK".to_string(),
//             ]),
//             find_most_prominent_face: true,
//         }
//     }
// }

//Whenever a face is identitified it's never really 100%, it's always a probability even
// if that probability is claimed to be 100%. This type also contains a list of N probable
// enrollments of descreasing probability.
// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub struct PossibleMatches {
//     pub face: Face,
//     pub attendance: Option<AttendanceStatus>, //None = hasn't been on property yet.
//     pub core_enrollments: Vec<CoreEnrollment>,
// }

// impl TryFrom<EnrollCommand> for SearchRequest {
//     type Error = TPassError;

//     fn try_from(cmd: EnrollCommand) -> Result<Self, Self::Error> {
//         //hey if we are in this function then json was converted to type so we know we
//         //have the data we need and unwrapping should never fail
//         let ccode = cmd.candidates.first().expect("need a ccode to convert to Search Req.").ccode.clone();
//         match ccode.parse() {
//            Ok(ccode) => {
//                Ok(Self {
//                    depth: None,
//                    comp_id: 0,
//                    client_type: "".to_string(),
//                    search_term: TPassSearchType::CCode(ccode) //only used value
//                })
//            },
//             Err(e) => {
//                 Err(TPassError::GenericError(Box::new(e)))
//             }
//         }

//     }
// }

//------- end v1 types -----
