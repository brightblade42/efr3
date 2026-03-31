export interface ImageAnalysisState {
  //TODO: is_analyzing represents bot detection and recognition.. not sure if we want more fine grained with distinct phases
  is_detecting_faces: boolean;
  is_recognizing_faces: boolean;
  is_analyzing_frame: boolean;
  is_analyzing_image: boolean;
  min_identity_confidence: number,
  //analyzed_image: AnalyzedImage | undefined,
  fr_identities: FrIdentity [] | undefined,
  selected_match: FrIdentity  | undefined
}

export interface VideoAnalysisState {
  snapshot_time: number, //when to copy a frame for analysis in milliseconds
  //TODO: look into what we should keep here, delete or move into different store
  //small_frame_step: FrameStep;  //Mistuh Hotsteppah.
  //med_frame_step: FrameStep;
  //large_frame_step: FrameStep;
  //is_detecting_faces: boolean;
  //is_recognizing_faces: boolean;
  is_analyzing_frame: boolean;
  //status_list: ProfileStatus [];
  //video_play_state: VideoPlayState;
  //video_analysis: VideoAnalysis,
  //selected_detection: ImageData | undefined //A captured face from the video or image frame
  selected_match: FrIdentity | undefined;//a selected and known face.

 //TODO: we may want to keep track of all the frames, in editing mode
 //which doesn't exist. ATM we only care about current frame.
  analyzed_frame: AnalyzedFrame | undefined;
}

export interface AnalyzedFrame {
  id: number                 //what are we.
  elapsed_time: number | 0;  //when were we
  frame_num: number | 0;     //which frame is we am be?
  fr_identities: FrIdentity [] | undefined;
  src_frame: string [] | Uint8Array [] | Uint8ClampedArray [] | undefined //the image copied from a video frome

}


export type FrIdentity = {
  face: NFace;
  possible_matches: PossibleMatch []
}

export type PersonalInfo = {
  //amPkId: number;
  //aptmnId: number;
  ccode: number;
  compId: number;
  company: string;
  fName: string;
  //imageFile: "F:\\ImageStore\\K12Images\\Photos\\Students\\J201122717_VALLE-MARTINEZ_ERNESTO.JPG",
  imgUrl: string;
  lName: string;
  name: string;
  sttsId: number;
  status: string;
  clntTid: number;
  type: string; //Student, Employeee
}

export type PossibleMatch = {
  fr_id: string;
  ext_id: number;
  confidence: number;
  details: PersonalInfo;
}

// export type CoreEnrollment = {
//   pv_id: string;
//   ccode: number;
//   confidence: number;
//   details: PersonalInfo;
// }
//NFace while we're not sure where existingFace type is used. no breaky

export type NFace = {
  //ages: number | undefined; //this is actually a set of distributions. not using yet.
  //aligned_face_image: string | undefined; //base64 image from server. don't currently need since we're cropping
  bbox: BoundingBox | undefined;
  //embedding: number | undefined;  //the face template repr as an array of float. not using here (yet)
  //genders: string | undefined; //a distribution
  //landmarks: number | undefined;
  //mask_probability: number;
  mask: number;
  quality: number | undefined
}

export type BoundingBox  = {
  origin: Origin;
  width: number;
  height: number;
}

export type Origin = {
  x: number;
  y: number
}

export interface Profile {
  ccode: number;
  compId: number;
  first: string;
  middle: string;
  last: string;
  status: number | undefined;  //from a list of options provided by TPass.
  client_type: number | undefined
  type: string | undefined
  image: Blob | undefined   //base64 or what else?
}

export interface ClientType  {
  clntTid: number,
  description: string,
  insrtDate: string,
  updtDate: string,
  insrtBy: string,
  updtBy: string,
  client: string [] | undefined,
  subClientType: string [] | undefined
}
export interface ProfileStatus  {
  sttsId: number,
  description: string,
  insrtDate: string,
  updtDate: string,
  insrtBy: string
  updtBy: string
  client: string [] | undefined
}


