//Face Recognition Types
export type Face = {
  acceptability: number;
  images: Images;
  landmarks?: Landmarks;
  bounding_box: BoundingBox;
  mask?: number;
  pitch: number;
  quality: number;
  roll: number;
  yaw: number;
  tracking_id?: string;
};

export type Point = { x: number; y: number };
type BoundingBox = { origin: Point };

export type Images = {
  expanded?: string;
  recognition_input_image?: string;
};

export type Landmarks = {
  left_eye: Point;
  right_eye: Point;
  left_mouth: Point;
  right_mouth: Point;
  nose: Point;
};

export type Credentials = {
  user_name: string;
  password: string;
}
/**
 * IdentifiedFace is the type of the result of a lookup.
 * Face contains info about the detected face
 * Attendance contains the check-in/out info
 */
export type RecognizedResult = {
  identity: Identity;
  location?: string; //camera name
  status?: RecognizedStatus;
  match_time?: string;
};
export type Identity = {
  face: Face;
  possible_matches: PossibleMatch[];
};

export type PossibleMatch = {
  ext_id?: number;
  confidence: number;
  fr_id?: string;
  details?: any; // Same note as above for JsonNode
};
export type RecognizedStatus = {
  time_stamp: string;
  tardy: boolean;
  kind: string;
};

//paravision results from Socket stream.
export type DetectedResult = {
  faces: Face[];
  metadata: {
    width: number;
    height: number;
    codec: string;
    frame_rate: string;
  };
  timestamp: Date;
};


export type FRAlert = {
  Type: string;
  CompId: number;
  PInfo: number;
  Image: string;
};

export type StateCallback = (cam: CameraData, state: CameraState) => void;
//TODO: Figure out how to simplify all the Camera and Stream types

//holds all available camera info and the status of any running camera stream
export type CameraInfo = {
  available_cams?: CameraData[];
  live_fr_streams?: FRStreamSettings[];

  //fr_streams_status?: FRStreamsStatus; //TODO: use to compare current live settings with saved settings
};

export type CameraState =
  | { name: string, state: 'None' }  //idle, disconnected
  | { name: string, state: 'FRConnecting'; } //connecting to server, attempting to start stream for recognition
  | { name: string, state: 'FRConnected'; } //all good. connected and streaming
  | { name: string, state: 'FRReconnecting'; } //attempting to reconnect to server
  | { name: string, state: 'FRDisconnecting'; } //attempting to disconnect from server and stop recognition stream
  | { name: string, state: 'FRDisconnected'; } //attempting to disconnect from server and stop recognition stream
  | { name: string, state: 'Adding'; } //attempting to add new camera. make sense?
  | { name: string, state: 'Added'; } //attempting to add new camera. make sense?
  | { name: string, state: 'Updating'; } //attempting to update camera settings
  | { name: string, state: 'Updated'; } //attempting to update camera settings
  | { name: string, state: 'Deleting'; } //attempting to delete camera
  | { name: string, state: 'Deleted'; } //attempting to delete camera
  | { name: string, state: 'Error'; error: Error; }; //error state



//Camera Data. Combined local cam settings, FR Stream settings, and Video Display info
export type CameraData = {
  //base cam settings
  id: number | undefined;
  name: string;
  direction: number; //Entrance, Exit, Corridor, Other
  rtsp_url: string; //top level rtsp url
  min_match: number | null; //anything below this confidence is not a match
  user_name: string; //TODO: user and password can be set at camera create but shouldn't be available when getting info.
  password: string; //TOOD: shouldn't use passwords here.
  fr_stream_settings: FRStreamSettings | undefined; //how we build recognition stream
  rtsp_stream_info: RTSPStreamInfo | undefined; //how we build the display
  proxy_url: string | undefined; //the base address of the cam proxy server (could also route through rev. proxy
  feed_position: number | undefined;
  //TODO: think. is this really the best place for this? not part of the camera settings
  //state: CameraState | undefined;

  /*is_updating: boolean | undefined;
  is_streaming: boolean | undefined; //TODO: deprecate
  is_detecting: boolean | undefined; //replaces streaming. more precise. is the fr face detection running on this camera stream?

   */
};


//RTSPVideo Api types
export type RTSPChannel = {
  name: string;
  url: string;
  on_demand: boolean;
  debug: boolean;
  status: number;
};

export type RTSPStreamInfo = {
  id: string | undefined;
  name: string;
  channels: Record<string, RTSPChannel>;
};

export type Payload = Record<string, RTSPStreamInfo>;

export type GetRTSPStreamsResp = {
  status: number;
  payload: Payload;
};

//the status of all running camera streams being analyzed for faces
//the result of a fetch to the paravision streams endpoint
//Config information about how paravision is analyzing a camera stream

export type FRStreamSettings = {
  detect_frame_rate: number;
  detect_mask: boolean;
  enable_tracking: boolean;
  expanded_image_scale: number;
  face_acceptability_tracking_threshold: number | undefined;
  face_quality_tracking_threshold: number | undefined;
  face_similarity_tracking_threshold: number | undefined;
  inference_worker_id: number | undefined;
  max_frames_per_track: number;
  metadata: Metadata | undefined;
  min_frames_per_track: number;
  name: string;
  output_faces_only: boolean | undefined;
  postprocess_worker_id: number | undefined;
  rotation: number;
  skip_identical_frames: boolean;
  source: string;
  tracking_duration: number;
  tracking_min_face_size: number;
};

//pv specific types
export type Metadata = {
  codec: string;
  frame_rate: string;
  height: number;
  width: number;
};

//paravision api structs
export type PVStreamResp = {
  message: string;
  stream: FRStreamSettings;
  success: boolean;
}

export type PVStreamsResp = {
  message: string;
  streams: FRStreamSettings[];
  success: boolean;
}


export class PVApiError extends Error {
  status_code: number | undefined;
  type: string | undefined;
  name: string;
  constructor(message: string) {
    super(message);
    this.name = "PVApiError";

  }

  static from_json_str(json: string) {
    let obj = JSON.parse(json);
    let err = new PVApiError(obj.message);
    err.name = obj.name;
    err.status_code = obj.status_code;
    err.type = obj.type;
    return err;
  }
}
//{"message":"Requested stream not found","name":"Cam1d","status_code":404,"type":"StreamNotFound"}
//the error result of a fetch to the paravision streams endpoint
export type PVStreamingApiError = {
  message: string;
  name: string;
  source: string;
  status_code: number;
  type: string;
};
//the types used to exchange data between client and server
export type WSMessage = {
  kind: string;
  payload: any;
};
