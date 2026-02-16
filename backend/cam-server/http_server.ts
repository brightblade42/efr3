import * as oak from "https://deno.land/x/oak@v12.6.0/mod.ts";
import { Server } from "https://deno.land/x/socket_io@0.2.0/mod.ts";
import { oakCors } from "https://deno.land/x/cors/mod.ts";
import { ExpiringMapTimed } from "./ExpiringMap.ts";
import { SocketMap } from "./socket_map.ts";
import { DB, LogErrorOptions } from "./db.ts";
import Logger from "https://deno.land/x/logger@v1.1.3/logger.ts";
import { err, ok, Result } from "./utils.ts";

import {
  CameraData,
  CameraState,
  Credentials,
  DetectedResult,
  Face,
  FRAlert,
  RecognizedResult,
  WSMessage,
} from "./types.ts";
//} from "../../shared/types.ts";
import { CamManager } from "./cam_manager.ts";
//TODO: move to env variables
const PV_WS_URL = Deno.env.get("PV_DETECTION_URL") ?? "ws://192.168.3.48:5050";
const PV_API_URL = Deno.env.get("PV_STREAM_URL") ?? "http://192.168.3.48:5000";
const PV_WS_ALERT_URL = Deno.env.get("PV_ALERTS_URL") ?? "ws://localhost:5051"; //"ws://192.168.3.48:5051";

const MIN_MATCH = Deno.env.get("CAM_SRV_MIN_MATCH") ?? "0.5";
const FR_API_URL = Deno.env.get("FR_API") ?? "http://localhost:3000";
const RTSP_API_URL =
  Deno.env.get("RTSP_API_URL") ?? "http://demo:demo@localhost:8083";
const CAM_PROXY_URL =
  Deno.env.get("RTSP_CAM_PROXY_URL") ?? "http://localhost:8083"; //not necessarily the same as RTSP_API_URL
const MATCH_EXPIRES = Deno.env.get("CAM_SRV_MATCH_EXPIRES") ?? "10"; //5; //10; //seconds
const LOG_DETECTIONS = Deno.env.get("CAM_SRV_LOG_DETECTIONS") ?? "false"; //false;//the metrics, man. the metrics.
const RETAIN_DETECTION_IMAGES =
  Deno.env.get("CAM_SRV_RETAIN_DETECTION_IMAGES") ?? "false"; // false;
const DB_ADDR = Deno.env.get("FR_DB") ?? "localhost";
const DB_PORT = parseInt(Deno.env.get("FR_DB_PORT") ?? "5433");
const DB_USER = Deno.env.get("FR_DB_USER") ?? "admin";
const DB_PWD = Deno.env.get("FR_DB_PWD") ?? "admin";
const LISTEN_PORT = Deno.env.get("LISTEN_PORT") ?? "3010";
const log = new Logger();
const socket_server = new Server();
const FR_WATCH_LABEL = "FR Watch";

//the pv streaming server.
socket_server.on("connection", (socket) => {
  console.log("a client has been connected");
});

class LookupError extends Error {
  constructor(
    public code: number,
    public reason: string,
    public details: string,
  ) {
    super(reason);
    this.name = "LookupError";
    this.code = code;
    this.details = details;
  }
}

const db = new DB(DB_ADDR, DB_PORT, DB_USER, DB_PWD); //if this fails, abort the program.

const cam_manager = new CamManager(
  db,
  log,
  PV_WS_URL,
  PV_WS_ALERT_URL,
  PV_API_URL,
  RTSP_API_URL,
  CAM_PROXY_URL,
);
const identified_cache = new ExpiringMapTimed<string, string>(
  MATCH_EXPIRES * 1000,
);

//cam control, state changes and faces flow over these sockets.
const client_ws_map = new SocketMap();

async function log_error(params: LogErrorOptions) {
  try {
    log.error(
      `${params.level} ${params.kind} ${params.summary} ${params.data} `,
    );
    await db.log_error(params);
  } catch (e) {
    log.error(e);
    //TODO: log to file as fatal err.
  }
}
async function lookup(face: Face, cam: CameraData): Promise<RecognizedResult> {
  let recognized_res: RecognizedResult;

  try {
    const rec_image = face.images?.recognition_input_image;

    if (!rec_image) {
      throw new LookupError(10, "no recognition image found", ""); //TODO:
    }

    const opts = {
      top_matches: 1,
      include_detected_faces: false,
      on_match: to_match_str(cam.direction),
      min_match: cam.min_match || MIN_MATCH,
      rec_location: cam.name,
    };

    //create a fetch request.
    const form_data = new FormData();
    //TODO: not sure about the first or third args here.
    form_data.append("image", rec_image, "image");
    form_data.append("opts", JSON.stringify(opts));
    //TOOD: look into updating this endpoint
    const resp = await fetch(`${FR_API_URL}/fr/recognize-faces-b64`, {
      method: "POST",
      body: form_data,
    });

    if (!resp.ok) {
      throw new LookupError(resp.status, resp.statusText, ""); //TODO:
    }

    const res = await resp.json();
    if (res.code === 0) {
      throw new LookupError(res.code, res.message, res.details);
    }
    if (res.identity === undefined) {
      return {
        identity: undefined,
        location: cam.name,
        match_time: new Date().toISOString(),
        status: undefined,
      };
    }

    try {
      if (
        res.identity.possible_matches !== undefined &&
        res.identity.possible_matches.length > 0
      ) {
        const pmatch = res.identity.possible_matches[0];

        if (pmatch.details.status === FR_WATCH_LABEL) {
          //if we got us a watch list candidate. fire an alert.
          const fr_alert: FRAlert = {
            Type: "FR Alert",
            Image: face.images.expanded,
            CompId: pmatch.details.compId,
            PInfo: pmatch.ext_id,
          };

          const alert_res = await send_alert(fr_alert);
          if (alert_res.kind === "err") {
            console.error("FR Alert request failed ", alert_res.error);
          } else {
            fr_alert.Image = ""; //don't need
            console.log("Alert Sent", fr_alert);
          }
        }
      }
    } catch (e) {
      console.log("WTF =======================");
      console.error(e);
    }
    //res.identity
    //convert to RecognizedResult. Almost the same as what we get from the FR API
    //but we need to include an image from our face since we've requested that
    //FR API not return the image (we have it already).
    //we also add the location of the camera that sent the image
    //and the time of the match
    recognized_res = {
      identity: res.identity,
      location: cam.name,
      match_time: new Date().toISOString(),
      status: res.status,
    };

    face.images.recognition_input_image = undefined; //not useful to send this back
    recognized_res.identity.face = face; //we use our local face object
  } catch (error) {
    throw error;
  }
  return recognized_res;
}

//The auto alert func.
async function send_alert(alert: FRAlert): Promise<Result<void, AppError>> {
  try {
    const resp = await fetch(`${FR_API_URL}/fr/send-alert`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(alert),
    });

    return ok(undefined);
  } catch (e) {
    const app_err = new AppError(e.message, "UnhandledAppError", [e.name]);
    return err(app_err);
  }
}

function to_match_str(direction: number): string {
  let on_match = "check_out";

  if (direction == 1) on_match = "check_in";
  else if (direction == 2) on_match = "id_only";

  return on_match;
}

/**
 * @param face Face object from the PV API
 * @param tracking_id string
 * @param cam CameraConfig
 * @returns () => Promise<IdentifiedFace>
 */

//return a function that will be called from a job queue
function create_lookup_task(face: Face, cam: CameraData) {
  return () => lookup(face, cam);
}

//command and control for camera activity.
cam_manager.addEventListener("camera_state_changed", (evt) => {
  //@ts-ignore: this is a custom event
  const cam = evt.detail.cam as CameraData;
  //@ts-ignore: this is a custom event
  const state = evt.detail.state as CameraState;
  //TODO: we're going to need a way to get the specific client socket, otherwise
  //we'll only be able broadcast to all clients (ok for now)
  switch (state.state) {
    //TODO: unsure if I actually need to do anything here.
    case "Adding":
      break;
    case "Added":
      //console.log("Camera added");
      //await broadcast_available_cams();
      break;
    case "Updating":
      break;
    case "Updated":
      //await broadcast_available_cams();
      break;
    case "Deleting":
      break;
    case "Deleted":
      //await broadcast_available_cams();
      break;
    case "FRConnecting":
      break;
    case "FRConnected":
      break;
    case "FRDisconnecting":
      break;
    case "FRDisconnected":
      break;
    case "Error":
      break;
  }

  log.info(`Camera state changed: ${cam.name} ${state.state}`);

  //send cameras and current state to all clients
  broadcast_cam_state(cam, state);
});

//this might be a job for web worker
//every detection gets written to a log, if configured for capture
async function log_detected_frame(cam: CameraData, res: DetectedResult) {
  if (res.faces.length === 0) return;

  //TODO: investigate if there are situations where we might want an untracked face.
  const faces_to_log = [];

  for (let i = 0; i < res.faces.length; i++) {
    const face = res.faces[i];
    const face_copy = Object.assign({}, face); //TODO: This might hurt performance
    face_copy.images = {};

    //TODO: check if this env var is properly set. seems like images are still saved
    if (RETAIN_DETECTION_IMAGES === "true") {
      face_copy.images.recognition_input_image =
        face.images.recognition_input_image;
    }

    faces_to_log.push(face_copy);
  }

  const detection_frame: DetectedResult = {
    faces: faces_to_log,
    metadata: res.metadata,
    timestamp: res.timestamp,
  };

  await db.log_detected_frame(cam, detection_frame);
}

//packets of faces from the PV API
cam_manager.addEventListener("frame_decoded", async (evt) => {
  //@ts-ignore: this is a custom event
  const detected = evt.detail.detected as DetectedResult;
  //@ts-ignore: this is a custom event
  const cam = evt.detail.cam as CameraData;
  const tasks = [];
  //tracking id is what allows us to cache a detection as it moves through the frame. this cuts
  //down on the number of lookups we have to do.
  detected.faces = detected.faces.filter((face) => face.tracking_id); //not tracking? not interested

  if (LOG_DETECTIONS === "true") {
    await log_detected_frame(cam, detected);
  }

  //TODO: this would be the place to filter out poor quality faces as well
  //push lookups onto a queue
  for (let idx = 0; idx < detected.faces.length; idx++) {
    const face = detected.faces[idx];
    const tracking_id = face.tracking_id;
    if (identified_cache.get(tracking_id) === undefined) {
      tasks.push(create_lookup_task(face, cam));
      identified_cache.set(tracking_id, "cool");
    }
  }

  //run lookups in "parallel"
  const results = await Promise.allSettled(tasks.map((task) => task()));

  //TODO: true error logging perhaps
  results.forEach((result) => {
    //promise settled
    if (result.status === "fulfilled") {
      const recognized_res = result.value;
      const pm = recognized_res.identity?.possible_matches?.at(0);
      if (pm === undefined) return;
      //this might be too
      log.info(`${pm.details.name}: ${pm.confidence}`);
      client_ws_map.broadcast({
        kind: "FaceIdentified",
        payload: recognized_res,
      });
    } else if (result.reason instanceof LookupError) {
      log.error(result);
    } else if (result.reason instanceof TypeError) {
      log.error("No result from lookup");
    } else {
      log.error("Unhandled error in lookup");
      log.error(result.reason);
    }
  });
});

const router = new oak.Router();

//HTTP REQUEST HANDLERS
//not being used.
router.get("/", (ctx) => {
  ctx.response.body = `<!DOCTYPE html>
  <html>
    <head><title>Hello oak!</title><head>
    <body>
      <h2>You don't belong here!</h2>
    </body>
  </html>`;
});

router.get("/cameras", async (ctx) => {
  console.log("get cameras request");
  const cams = await cam_manager.get_all_camera_info();
  ctx.response.body = { kind: "AvailableCameras", payload: cams };
});

class AppError extends Error {
  details: string[] | undefined;
  message: string;
  constructor(message: string, name: string, details: string[] | undefined) {
    super(message);
    this.message = message;
    this.name = name;
    this.details = details;
  }
}

router.post("/cameras/add", async (ctx) => {
  console.log("add camera request");
  const cam: CameraData = await ctx.request.body().value;
  const res = await cam_manager.add_camera(cam);

  if (res.kind === "err") {
    const app_err = new AppError(res.error.message, "CamAddError", [
      "unimportant details",
    ]);
    ctx.response.body = JSON.stringify(app_err);
  } else {
    ctx.response.body = cam;
  }
});

router.post("/cameras/delete", async (ctx) => {
  console.log("delete camera request");
  const cam: CameraData = await ctx.request.body().value;
  const res = await cam_manager.delete_camera(cam);

  if (res.kind === "err") {
    const app_err = new AppError(res.error.message, "CamDeleteError", [
      "unimportant details",
    ]);
    ctx.response.body = JSON.stringify(app_err);
  } else {
    ctx.response.body = cam;
  }
});

router.post("/cameras/update", async (ctx) => {
  try {
    console.log("update camera request");
    const cam: CameraData = await ctx.request.body().value;
    const res = await cam_manager.update_camera(cam);

    if (res.kind === "err") {
      const app_err = new AppError(res.error.message, "CamUpdateError", [
        "unimportant details",
      ]);
      ctx.response.body = JSON.stringify(app_err);
    } else {
      ctx.response.body = cam;
    }
  } catch (e) {
    const app_err = new AppError(e.message, "UnhandledAppError", [e.name]);
    ctx.response.body = JSON.stringify(app_err);
  }
});

router.post("/send-fr-alert", async (ctx) => {
  try {
    let b: FRAlert = await ctx.request.body().value;

    const resp = await fetch(`${FR_API_URL}/fr/send-alert`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(b),
    });
    ctx.respons.body(JSON.stringify({}));
  } catch (e) {
    const app_err = new AppError(e.message, "UnhandledAppError", [e.name]);
    ctx.respons.body = JSON.stringify(app_err);
  }
});

router.post("/login", async (ctx) => {
  try {
    let b: Credentials = await ctx.request.body().value;
    console.log(b);
    //fake it for now
    if (b.user_name === "admin" && b.password === "njbs1968") {
      ctx.response.body = JSON.stringify({
        status: "LoggedIn",
        payload: { role: "admin" },
      });
    } else {
      ctx.response.body = JSON.stringify({
        status: "Failed",
        payload: { msg: "Invalid credentials" },
      });
    }
  } catch (e) {
    const app_err = new AppError(e.message, "UnhandledAppError", [e.name]);
    ctx.response.body = JSON.stringify({
      status: "Failed",
      payload: { msg: e.message },
    });
  }
});

//WEB SOCKET CAMERA COMMANDS

type Command = {
  cmd:
    | "GetAvailableCameras"
    | "StartCameraFR"
    | "StopCameraFR"
    | "StartAllCameras"
    | "StopAllCameras"
    | "AddCamera"
    | "DeleteCamera"
    | "UpdateCamera";

  payload?: any;
};

//client requests ws connection, add to map
router.get("/wss", (ctx) => {
  if (!ctx.isUpgradable) {
    ctx.throw(501);
  }
  console.log("did we get here?");
  const ws = ctx.upgrade();
  const sock_id = client_ws_map.gen_id();
  ws.onopen = () => {
    console.log("ws connected");
    client_ws_map.add(sock_id, ws);
  };

  ws.onmessage = async (m) => {
    if (m.data == "ping") return; //test

    try {
      const cmd = JSON.parse(m.data as string) as Command;

      switch (cmd.cmd) {
        case "GetAvailableCameras": {
          const cams = await cam_manager.get_all_camera_info();
          const avail_cam_msg: WSMessage = {
            kind: "AvailableCameras",
            payload: cams,
          };
          send_msg(ws, avail_cam_msg);
          break;
        }
        case "StartCameraFR":
          console.log("start camera fr message received");
          await cam_manager.connect_camera(cmd.payload);
          break;
        case "StopCameraFR":
          console.log("stop camera fr message received");
          await cam_manager.disconnect_camera(cmd.payload);
          break;
        case "StartAllCameras":
          ws.send("All Camera started");
          break;
        case "StopAllCameras":
          ws.send("All Camera stopped");
          break;

        case "AddCamera": {
          await cam_manager.add_camera(cmd.payload);
          break;
        }
        case "DeleteCamera":
          await cam_manager.delete_camera(cmd.payload);
          break;
        case "UpdateCamera":
          await cam_manager.update_camera(cmd.payload);
          break;
        default:
          console.log("Unknown command: ", cmd.cmd);
          ws.send(m.data as string);
      }
    } catch (e) {
      console.error(" *** UNHANDLED **** ");
      console.error("Unhandled WS message error: ", e);
    }
  };

  ws.onclose = () => {
    client_ws_map.remove(sock_id);
    console.log("Disconncted from client");
  };
});

//WS Helper functions
function send_msg(ws: WebSocket, msg: WSMessage) {
  ws.send(JSON.stringify(msg));
}
function broadcast_msg(msg: WSMessage) {
  client_ws_map.broadcast(msg);
}

function broadcast_cam_state(cam: CameraData, state: CameraState) {
  const data = { cam: cam, state: state };
  broadcast_msg({ kind: "CameraStateChanged", payload: data });
}

//Start the server
const app = new oak.Application();
app.use(oakCors());
app.use(router.routes());
app.use(router.allowedMethods());

app.addEventListener("listen", ({ hostname, port, secure }) => {
  console.log(
    `Listening on: ${secure ? "https://" : "http://"}${
      hostname ?? "localhost"
    }:${port}`,
  );
});
//this is fun
//port from env or default
console.log("This is Deno");
const listen = parseInt(LISTEN_PORT);
app.listen({ port: listen });
