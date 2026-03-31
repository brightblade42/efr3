// deno-lint-ignore-file no-explicit-any
import { io, Socket } from "https://cdn.socket.io/4.6.0/socket.io.esm.min.js";
import { Err, err, ok, Result } from "./utils.ts";
import Logger from "https://deno.land/x/logger@v1.1.3/logger.ts";
import {
  CameraData,
  CameraInfo,
  CameraState,
  DetectedResult,
  FRStreamSettings,
  GetRTSPStreamsResp,
  PVApiError,
  PVStreamResp,
  PVStreamsResp,
  RTSPStreamInfo,
} from "./types.ts";
//} from "../../shared/types.ts";
import { DB, LogErrorOptions } from "./db.ts";
export type CamName = string;

///paravision provides a socket for each camera over which
///it sends decoded frames of faces
export class CamManager extends EventTarget {
  camdb: DB; //setting from environment variables
  log: Logger;
  sockets: Map<string, Socket>;
  alert_socket: Socket | undefined;
  pv_ws_url: string;
  pv_ws_alert_url: string;
  pv_api_url: string;
  rtsp_api_url: string;
  cam_proxy_url: string;
  max_connect_retries = 3;
  retry_map = new Map<string, number>();
  constructor(
    db: DB,
    log: Logger,
    pv_ws_url: string,
    pv_ws_alert_url: string,
    pv_api_url: string,
    rtsp_api_url: string,
    cam_proxy_url: string,
  ) {
    super();
    this.sockets = new Map<string, Socket>();
    this.pv_ws_url = pv_ws_url;
    this.pv_ws_alert_url = pv_ws_alert_url;
    this.pv_api_url = pv_api_url;
    this.rtsp_api_url = rtsp_api_url;
    this.cam_proxy_url = cam_proxy_url;
    this.camdb = db;
    this.log = log;

    this.connect_alerts();
    log.info("CamManager initialized");
  }

  //TODO: retry logic is completely broken. need to rethink this. when count reaches 0 it resets to max retries
  ///connect to the paravision websocket for each camera, could fail because networks are unreliable and so we retry
  async attempt_reconnect(cam: CameraData) {
    let retries = this.retry_map.get(cam.name);
    if (!retries) {
      this.retry_map.set(cam.name, this.max_connect_retries);
      retries = this.max_connect_retries;
    }
    if (retries > 0) {
      retries -= 1;
      this.retry_map.set(cam.name, retries);
      this.log.warn(
        "attemting to reconnect: " + cam.name + " retries left: " + retries,
      );

      this.connect_pv(cam);
    } else {
      const err = new Error(
        `Max retries of ${this.max_connect_retries} exceeded for Camera: ` +
          cam.name,
      );
      await this.log_error({
        level: "serious",
        kind: "attempt_reconnect",
        summary: err.message,
      });
      this.retry_map.delete(cam.name); //we want to be able to try again
      this.fire_camera_state_changed(cam, {
        name: cam.name,
        state: "Error",
        error: err,
      });
    }
  }

  connect_alerts() {
    const alert_socket = io(this.pv_ws_alert_url + "/alerts", {
      transports: ["websocket"],
    });

    alert_socket.on("connect", () => {
      this.log.info(`alert socket connected`);
    });

    alert_socket.on("disconnect", async () => {
      await this.log_error({
        level: "warn",
        kind: "fr_streaming_error",
        summary: "paravision alert socket disconnected ",
      });
    });

    alert_socket.on("connect_error", (e: any) => {
      this.log.error(
        `alert socket connect error. indicates service problem: ${e}`,
      );
    });

    alert_socket.on("alert", async (msg: any) => {
      try {
        const data = JSON.parse(msg);
        this.fire_alert_received(data);

        await this.log_error({
          level: "critical",
          kind: "fr_streaming_error",
          summary: "alert from paravision streaming service",
          data: data,
        });
      } catch (e) {
        await this.log_error({
          level: "critical",
          kind: "fr_streaming_error",
          summary: "alert handler failed to parse alert message",
        });
      }
    });

    if (this.alert_socket) {
      this.alert_socket.close();
    }
    this.alert_socket = alert_socket;
  }

  async log_error(params: LogErrorOptions) {
    try {
      this.log.error(
        `${params.level} ${params.kind} ${params.summary} ${params.data} `,
      );
      await this.camdb.log_error(params);
    } catch (e) {
      this.log.error(e);
      //TODO: log to file as fatal err.
    }
  }

  //paravision socket connections. each camera has it's own socket
  connect_pv(cam: CameraData) {
    const pv_socket: Socket = io(this.pv_ws_url + "/" + cam.name, {
      reconnectionDelayMax: 10000,
      transports: ["websocket"],
    });

    pv_socket.on("connect", () => {
      this.fire_camera_state_changed(cam, {
        name: cam.name,
        state: "FRConnected",
      });
      this.log.info(`${cam.name} pv socket connected`);
      this.retry_map.set(cam.name, this.max_connect_retries); //reset retries count
    });

    pv_socket.on("disconnect", () => {
      //reconnect logic?
      this.log.warn(`${cam.name} pv socket disconnected`);
      this.fire_camera_state_changed(cam, {
        name: cam.name,
        state: "FRDisconnected",
      });
    });

    pv_socket.on("connect_error", async (err: any) => {
      this.fire_camera_state_changed(cam, {
        name: cam.name,
        state: "Error",
        error: err,
      });

      await this.log_error({
        level: "warn",
        kind: "camera_connection_error",
        summary: `${cam.name} socket connect error. ${err}`, //err pass as data?
      });
      //try again.
      await this.attempt_reconnect(cam);
    });

    //Faces were detected in a video frame,
    pv_socket.on("frame_decoded", async (msg: any) => {
      //TODO: write to Detection_log
      try {
        const decoder = new TextDecoder("utf-8");
        const txt = decoder.decode(msg);
        const detected: DetectedResult = JSON.parse(txt);

        //pass up the detected faces and the camera they were detectec on.
        const evt_data = {
          cam: cam,
          detected: detected,
        };

        //let the world know
        this.dispatchEvent(
          new CustomEvent("frame_decoded", { detail: evt_data }),
        );
      } catch (e) {
        await this.log_error({
          level: "serious",
          kind: "frame_decoded_error",
          summary: `Error decoding frame ${e.message}`,
          data: e,
        });
      }
    });

    this.addSocket(cam.name, pv_socket);
  }

  private fire_camera_state_changed(cam: CameraData, state: CameraState) {
    const evt_data = { cam: cam, state: state };
    //TODO: Validate state transitions
    this.dispatchEvent(
      new CustomEvent("camera_state_changed", { detail: evt_data }),
    );
  }
  private fire_alert_received(alert: any) {
    const evt_data = alert;
    //TODO: Validate state transitions
    this.dispatchEvent(new CustomEvent("alert_received", { detail: evt_data }));
  }

  private addSocket(id: CamName, socket: Socket) {
    this.sockets.set(id, socket);
  }

  private removeSocket(id: string) {
    //TODO:check if socket exists and if it's connected, disconnect it
    this.sockets.delete(id);
  }

  async get_cameras(): Promise<CameraData[]> {
    return await this.camdb.get_cameras();
  }

  async get_camera(name: string): Promise<Result<CameraData, Error>> {
    try {
      const res = await this.camdb.get_camera_by_name(name);
      if (res.kind == "err") {
        return res;
      }
      return ok(res.value);
    } catch (e) {
      return await Promise.reject(e);
    }
  }

  set_proxy_url(cams: CameraData[]) {
    if (this.cam_proxy_url) {
      cams.forEach((cam) => {
        cam.proxy_url = this.cam_proxy_url;
      });
    }
  }
  //TODO: make sure an error is thrown if any api calls fail
  async get_all_camera_info(): Promise<CameraInfo> {
    try {
      const cams = await this.get_cameras();
      this.set_proxy_url(cams);
      //const resp = await fetch(this.streams_url); //pv
      const pv_res = await this.get_pv_streams();
      if (pv_res.kind == "err") {
        throw pv_res.error;
      }
      const cam_info = { available_cams: cams, live_fr_streams: pv_res.value };
      await this.merge_rtsp_streams(cam_info);
      return cam_info;
    } catch (e) {
      await this.log_error({
        level: "serious",
        kind: "get_all_camera_info",
        summary: `Unhandled Error in get_all_camera_info. ${e.message}`,
      });

      return await Promise.reject(e);
    }
  }

  //if any streams are being actively recognized, we want to know about it
  async get_pv_streams(): Promise<Result<FRStreamSettings[], PVApiError>> {
    try {
      const url = `${this.pv_api_url}/streams`;
      const resp = await fetch(url); //pv

      if (!resp.ok) {
        try {
          const api_err = await resp.json();
          const pv_err = PVApiError.from_json_str(JSON.stringify(api_err));
          await this.log_error({
            level: "serious",
            kind: "get_pv_streams",
            summary: `Error getting streams from pv api. ${pv_err.message}`,
            data: JSON.stringify(api_err),
          });

          return err(pv_err);
        } catch (e) {
          await this.log_error({
            level: "serious",
            kind: "get_pv_streams",
            summary: `Exception caught: ${e.message}`,
            data: JSON.stringify(e),
          });
          return err(
            new PVApiError("Unknown error getting streams from pv api"),
          );
        }
      }

      const pv_resp: PVStreamsResp = await resp.json();

      if (!pv_resp.success) {
        throw new Error(
          "Error getting stream from api. got response but not successful",
        );
      }

      return ok(pv_resp.streams);
    } catch (e) { //explicit catchall for things we could have missed.
      await this.log_error({
        level: "serious",
        kind: "get_pv_streams",
        summary: `Unhandled Exception caught: ${e.message}`,
        data: JSON.stringify(e),
      });

      return Promise.reject(e);
    }
  }

  async get_pv_stream(
    name: string,
  ): Promise<Result<FRStreamSettings, PVApiError>> {
    try {
      this.log.info("Getting stream: " + name);
      const endpoint = `${this.pv_api_url}/streams/${name}`;
      const resp = await fetch(endpoint);

      if (!resp.ok) {
        const api_err = await resp.json();
        const pv_err = PVApiError.from_json_str(JSON.stringify(api_err));
        await this.log_error({
          level: "serious",
          kind: "get_pv_stream",
          summary: ` ${pv_err.name} : ${pv_err.message}`,
          data: JSON.stringify(pv_err),
        });
        return err(pv_err);
      }

      const pv_resp: PVStreamResp = await resp.json();

      if (!pv_resp.success) {
        throw new Error(
          "Error getting stream from api. got response but not successful",
        );
      }

      return ok(pv_resp.stream);
    } catch (e) { //explicit catchall for things we could have missed.
      await this.log_error({
        level: "serious",
        kind: "get_pv_stream",
        summary: `Unhandled Exception caught: ${e.message}`,
        data: JSON.stringify(e),
      });

      return Promise.reject(e);
    }
  }

  // deno-lint-ignore no-explicit-any
  async add_camera(cam: CameraData): Promise<Result<any, Error>> {
    try {
      if (this.sockets.has(cam.name)) {
        const er = new Error(`${cam.name}: exists and in use!`);
        this.fire_camera_state_changed(cam, {
          name: cam.name,
          state: "Error",
          error: er,
        });
        return err(er);
      }
      this.fire_camera_state_changed(cam, { name: cam.name, state: "Adding" });
      //in db?
      const cam_exists = await this.camdb.camera_exists(cam.name);
      if (cam_exists) {
        const er = new Error(
          `${cam.name}: already exists. Camera name must be unique`,
        );

        this.fire_camera_state_changed(cam, {
          name: cam.name,
          state: "Error",
          error: er,
        });
        return err(er);
      }

      //rtsp api will not prevent streams added with same name, so we much check here
      const streams_res = await this.get_rtsp_streams();
      if (streams_res.kind === "err") {
        const er = new Error(
          `Error validating any existing rtsp streams for ${cam.name}.`,
        );
        this.fire_camera_state_changed(cam, {
          name: cam.name,
          state: "Error",
          error: er,
        });
        return err(er);
      }
      const rtsp_exists = streams_res.value.find((s) => s.name === cam.name);
      if (rtsp_exists) {
        const er = new Error(
          `${cam.name}: rtsp stream already exists. Camera name must be unique`,
        );
        this.fire_camera_state_changed(cam, {
          name: cam.name,
          state: "Error",
          error: er,
        });
        return err(er);
      }

      //we want this to be a 404 error. is this redundant. If the socket is connected
      //to the pv stream service, then we know it exists.
      //is it possible that the stream exists in pv but not connected?
      //That feels like an out of sync issue. Regardless, this shouldn't cause
      //a problem since we'll probably never get here.
      const res = await this.get_pv_stream(cam.name);
      if (res.kind === "err") {
        if (res.error.status_code !== 404) {
          this.fire_camera_state_changed(cam, {
            name: cam.name,
            state: "Error",
            error: res.error,
          });
          return err(res.error);
        }
      }

      const db_res = await this.camdb.save_new_camera(cam);
      // if (!db_res.length) {
      //     const er = new Error(`db save failed. could not save cam: ${cam.name}`);
      //     await this.log_error({
      //         level: "serious",
      //         kind: "add_camera",
      //         summary: `db save failed. Could not save cam: ${cam.name}: ${er.message}`,
      //     });
      //
      //     this.fire_camera_state_changed(n_cam, {name: n_cam.name, state: "Error", error: er});
      //     return err(er);
      // }

      //this is hot trash. the point of result is not to throw but deadlines man
      if (db_res.kind === "err") {
        throw db_res.error;
      }

      const n_cam = db_res.value;

      //add to the rtsp api
      const rtsp_res = await this.add_rtsp_stream(n_cam);
      if (rtsp_res.kind === "err") {
        const er = new Error(`rtsp api failed to add stream: ${n_cam.name}`);
        this.fire_camera_state_changed(n_cam, {
          name: n_cam.name,
          state: "Error",
          error: er,
        });
        return err(er);
      }

      n_cam.proxy_url = this.cam_proxy_url;
      const get_rtsp_res = await this.get_rtsp_stream(n_cam.name);

      if (get_rtsp_res.kind === "err") {
        const er = new Error(
          `rtsp api failed to get new stream: ${n_cam.name}`,
        );
        this.fire_camera_state_changed(n_cam, {
          name: n_cam.name,
          state: "Error",
          error: er,
        });
        return err(er);
      }
      n_cam.rtsp_stream_info = get_rtsp_res.value;

      this.fire_camera_state_changed(n_cam, {
        name: n_cam.name,
        state: "Added",
      });

      return ok(n_cam); //just needs to convey success.
    } catch (e) {
      //TODO:should we try to undo what succeeded if something fails?
      await this.log_error({
        level: "serious",
        kind: `add_camera`,
        summary: `Exception: ${cam.name} -  ${e.message}`,
        data: JSON.stringify(e),
      });

      this.fire_camera_state_changed(cam, {
        name: cam.name,
        state: "Error",
        error: e,
      });
      return await Promise.reject(e);
    }
  }

  async update_camera(cam: CameraData): Promise<Result<CameraData, Error>> {
    try {
      //make sure camera we want to update exists
      if (!cam.id) {
        const e = new Error(`existing camera id is required to update camera`);
        await this.log_error({
          level: "warn",
          kind: `update_camera`,
          summary: `${cam.name} :  ${e.message}`,
          data: JSON.stringify(e),
        });
        this.fire_camera_state_changed(cam, {
          name: cam.name,
          state: "Error",
          error: e,
        });
        return err(e);
      }

      //we need the pre-update camera values to stop and remove old streams. external apis may rely on
      //data that has been requested for update. ie (name)
      const res = await this.camdb.get_camera(cam.id);

      if (res.kind === "err") {
        await this.log_error({
          level: "warn",
          kind: `update_camera`,
          summary: `${cam.name} :  ${res.error.message}`,
          data: JSON.stringify(res.error),
        });

        this.fire_camera_state_changed(cam, {
          name: cam.name,
          state: "Error",
          error: res.error,
        });
        return err(res.error);
      }

      const unchanged_cam = res.value;
      let was_pv_streaming = false;
      //let cam_name = cam.name;
      this.fire_camera_state_changed(unchanged_cam, {
        name: unchanged_cam.name,
        state: "Updating",
      });

      //step 1
      const pv_res = await this.get_pv_stream(unchanged_cam.name);
      if (pv_res.kind === "err") { //we want the error case
        if (pv_res.error.status_code != 404) {
          throw pv_res.error; //unexpected error
        }
      } else {
        const pv_del_res = await this.disconnect_camera(unchanged_cam);
        if (pv_del_res.kind === "err") {
          //disconnect fires its own state change event
          return pv_del_res;
        } else {
          was_pv_streaming = true;
          this.log.info(`${unchanged_cam.name} pv stream stopped`);
        }
      }

      //step 2: delete from rtsp api
      let rtsp_res = await this.delete_rtsp_stream(unchanged_cam);
      if (rtsp_res.kind === "err") {
        this.fire_camera_state_changed(unchanged_cam, {
          name: unchanged_cam.name,
          state: "Error",
          error: rtsp_res.error,
        });
        return rtsp_res;
      }

      //step 3 update camera db
      const db_res = await this.camdb.update_camera(cam);
      if (db_res.kind === "err") {
        await this.log_error({
          level: "serious",
          kind: `update_camera`,
          summary:
            `could not update ${unchanged_cam.name} :  ${db_res.error.message}`,
          data: JSON.stringify(db_res.error),
        });
        this.fire_camera_state_changed(unchanged_cam, {
          name: unchanged_cam.name,
          state: "Error",
          error: db_res.error,
        });
        return db_res;
      } else if (!db_res.value.length) {
        const er = new Error("camera not in db. nothing to update");
        await this.log_error({
          level: "warn",
          kind: `update_camera`,
          summary: `could not update ${unchanged_cam.name} :  ${er.message}`,
          data: JSON.stringify(er),
        });
        this.fire_camera_state_changed(unchanged_cam, {
          name: unchanged_cam.name,
          state: "Error",
          error: er,
        });
      }

      //step 4: add the updated rtsp stream
      rtsp_res = await this.add_rtsp_stream(cam);
      if (rtsp_res.kind === "err") {
        const e = new Error(`rtsp api failed to add stream: ${cam.name}`);
        this.fire_camera_state_changed(cam, {
          name: cam.name,
          state: "Error",
          error: e,
        });
        return err(e);
      }
      const cam_res = await this.merge_rtsp_stream(cam);

      if (cam_res.kind === "err") {
        const e = new Error(`failed to get new rtsp stream: ${cam.name}`);
        this.fire_camera_state_changed(cam, {
          name: cam.name,
          state: "Error",
          error: e,
        });
        return err(e);
      }

      cam = cam_res.value;
      //only start the pv stream if it was streaming before update
      this.fire_camera_state_changed(cam, { name: cam.name, state: "Updated" });

      //I think we always want to connect to pv after update
      if (was_pv_streaming) {
        await this.connect_camera(cam);
      }

      return ok(cam);
    } catch (e) {
      await this.log_error({
        level: "serious",
        kind: `update_camera`,
        summary: `could not update ${cam.name} :  ${e.message}`,
        data: JSON.stringify(e),
      });
      this.fire_camera_state_changed(cam, {
        name: cam.name,
        state: "Error",
        error: e.message,
      });
      return await Promise.reject(e);
    }
  }
  async delete_camera(cam: CameraData): Promise<Result<CameraData, Error>> {
    try {
      this.fire_camera_state_changed(cam, {
        name: cam.name,
        state: "Deleting",
      });
      const pv_res = await this.get_pv_stream(cam.name);
      if (pv_res.kind === "err") { //we want the error case
        if (pv_res.error.status_code != 404) {
          throw pv_res.error; //unexpected error
        }
      } else {
        //we got a stream back, so we need to stop it, if it's running
        //if camera fails to stop, it will return so that
        //we can make another attempt? Not sure about this.
        const pv_stream = pv_res.value;
        const pv_del_res = await this.disconnect_camera(cam);
        if (pv_del_res.kind === "err") {
          return pv_del_res;
        }
      }

      //delete from rtsp api
      const rtsp_res = await this.delete_rtsp_stream(cam);
      if (rtsp_res.kind === "err") {
        this.fire_camera_state_changed(cam, {
          name: cam.name,
          state: "Error",
          error: rtsp_res.error,
        });
        return rtsp_res;
      }

      //delete from db
      const _db_res = await this.camdb.delete_camera(cam.name);
      this.fire_camera_state_changed(cam, { name: cam.name, state: "Deleted" });
      return ok(cam);
    } catch (e) {
      await this.log_error({
        level: "serious",
        kind: `delete_camera`,
        summary:
          `Unhandled Exception: could not delete ${cam.name} :  ${e.message}`,
        data: JSON.stringify(e),
      });
      this.fire_camera_state_changed(cam, {
        name: cam.name,
        state: "Error",
        error: e,
      });
      return await Promise.reject(e);
    }
  }

  //contact paravision to start the stream
  //take cameraConfig and start the stream.
  //contact pv , if successful, check the socket map
  //if the socket exists, disconnect it and remove it from the map
  //connect the new socket and add it to the map by calling connect
  async connect_camera(cam: CameraData) {
    const url = `${this.pv_api_url}/start_decode`;
    try {
      //contact paravision to start the stream
      //TODO: handle start_decode errors
      //TODO: figure out which of our settings we want to use, instead of the defaultsd
      const req_params = {
        name: cam.name,
        source: cam.rtsp_url,
        skip_identical_frames: true,
        detect_frame_rate: cam.fr_stream_settings?.detect_frame_rate || 10,
        detect_mask: true, //is this a thing anymore?
      };

      console.log("----- connect stream ------");
      console.log("fram rate: ", req_params.detect_frame_rate);
      this.fire_camera_state_changed(cam, {
        name: cam.name,
        state: "FRConnecting",
      });

      const request = new Request(url, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(req_params),
      });
      const resp = await fetch(request);

      //holy hell. this is trash
      if (!resp.ok) {
        if (resp.status == 400) { //400 here means.. already decoding.
          const _err_resp = await resp.json();
          if (!this.sockets.has(cam.name)) {
            this.connect_pv(cam); //connect to pv to get faces from cam
          } else {
            //socket is already connected, so we're good. update state
            this.fire_camera_state_changed(cam, {
              name: cam.name,
              state: "FRConnected",
            });
          }
          return;
        } else {
          const er = new Error(resp.statusText);
          er.name = `${resp.status}`;
          throw er;
        }
      }

      //TODO: make sure the response is good, handle not good ones.
      const _cam_start_resp = await resp.json();
      this.log.info(`${cam.name} pv is decoding successfully`);
      this.connect_pv(cam); //reports connected state
    } catch (e) {
      await this.log_error({
        level: "serious",
        kind: `connect_camera`,
        summary: `Unhandled Exception: ${cam.name} :  ${e.message}`,
        data: JSON.stringify(e),
      });
      this.fire_camera_state_changed(cam, {
        name: cam.name,
        state: "Error",
        error: e,
      });
    }
  }

  //tell paravision to stop recognizing faces for a camera.
  async disconnect_camera(cam: CameraData): Promise<Result<CameraData, Error>> {
    try {
      this.fire_camera_state_changed(cam, {
        name: cam.name,
        state: "FRDisconnecting",
      });

      const url = `${this.pv_api_url}/stop_decode`;
      const req_params = {
        name: cam.name,
      };
      const request = new Request(url, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(req_params),
      });

      let resp = await fetch(request);

      if (!resp.ok) {
        if (resp.status === 400) {
          const err_resp = await resp.json(); //wat if this failes too
          if (err_resp.message === "Stream is not being decoded") {
            this.fire_camera_state_changed(cam, {
              name: cam.name,
              state: "FRDisconnected",
            });
            this.removeSocket(cam.name); //just in case.
            return ok(cam);
          }
          const er = new Error(err_resp.statusText);
          er.name = "DisconnectCameraError";
          await this.log_error({
            level: "serious",
            kind: `disconnect_camera`,
            summary: `disconnect camera failed: ${cam.name} :  ${er.message}`,
            data: JSON.stringify(er),
          });
          this.fire_camera_state_changed(cam, {
            name: cam.name,
            state: "Error",
            error: er,
          });
          return err(er);
        } else {
          //some non 400 error
          const er = new Error(resp.statusText);
          er.name = "DisconnectCameraError";

          await this.log_error({
            level: "serious",
            kind: `disconnect_camera`,
            summary: `disonnect camera failed: ${cam.name} :  ${er.message}`,
            data: JSON.stringify(er),
          });

          this.fire_camera_state_changed(cam, {
            name: cam.name,
            state: "Error",
            error: er,
          });
          return err(er);
        }
      } else {
        const strm_resp = await resp.json();

        if (!strm_resp.success) { //json failed to parse
          const e = new Error(strm_resp.message);
          await this.log_error({
            level: "serious",
            kind: `disconnect_camera`,
            summary: `${cam.name} :  ${e.message}`,
            data: JSON.stringify(e),
          });
          this.fire_camera_state_changed(cam, {
            name: cam.name,
            state: "Error",
            error: e,
          });
          return err(e);
        }
      }

      if (this.sockets.has(cam.name)) {
        this.sockets.get(cam.name)?.disconnect();
        this.sockets.delete(cam.name);
      }

      this.fire_camera_state_changed(cam, {
        name: cam.name,
        state: "FRDisconnected",
      });

      return ok(cam);
    } catch (e) {
      await this.log_error({
        level: "serious",
        kind: `disconnect_camera`,
        summary: `Unhandled Exception: ${cam.name} :  ${e.message}`,
        data: JSON.stringify(e),
      });
      this.fire_camera_state_changed(cam, {
        name: cam.name,
        state: "Error",
        error: e,
      });
      return await Promise.reject(e);
    }
  }

  async get_rtsp_stream(
    cam_name: string,
  ): Promise<Result<RTSPStreamInfo, Error>> {
    const endpoint = `${this.rtsp_api_url}/stream/${cam_name}/info`;
    try {
      const resp = await fetch(endpoint);
      if (!resp.ok) {
        const er = new Error(resp.statusText);
        er.name = resp.status.toString();

        await this.log_error({
          level: "serious",
          kind: "get_rtsp_stream",
          summary: `Could not get rtsp stream from api for ${cam_name}`,
          data: JSON.stringify(er),
        });
        return err(er);
      }
      const streams_resp: GetRTSPStreamsResp = await resp.json();
      const rtsp_info: RTSPStreamInfo = streams_resp.payload;

      return ok(rtsp_info);
    } catch (e) {
      await this.log_error({
        level: "serious",
        kind: "get_rtsp_stream",
        summary: `Exception: ${e.message}`,
        data: JSON.stringify(e),
      });

      return await Promise.reject(e);
    }
  }
  //get all the rtsp streams from rtsp api, these should be in sync with the cameras. What if they aren't?
  async get_rtsp_streams(): Promise<Result<RTSPStreamInfo[], Error>> {
    const endpoint = `${this.rtsp_api_url}/streams`;
    try {
      const resp = await fetch(endpoint);
      if (!resp.ok) {
        const er = new Error(resp.statusText);
        er.name = resp.status.toString();

        await this.log_error({
          level: "serious",
          kind: "get_rtsp_streams",
          summary: `Could not get rtsp streams from api`,
          data: JSON.stringify(er),
        });
        return err(er);
      }
      const streams_resp: GetRTSPStreamsResp = await resp.json();
      const rtsp_list: RTSPStreamInfo[] = [];

      for (const id in streams_resp.payload) {
        const rtsp_info = streams_resp.payload[id];
        rtsp_info.id = id;
        rtsp_list.push(rtsp_info);
      }
      return ok(rtsp_list);
    } catch (e) {
      await this.log_error({
        level: "serious",
        kind: "get_rtsp_streams",
        summary: `Exception: ${e.message}`,
        data: JSON.stringify(e),
      });

      return await Promise.reject(e);
    }
  }

  async delete_rtsp_stream(cam: CameraData): Promise<Result<any, Error>> {
    try {
      const url = `${this.rtsp_api_url}/stream/${cam.name}/delete`;
      const res = await fetch(url);
      if (!res.ok) {
        //500 means stream wasn't found, which is ok
        if (res.status != 500) throw new Error(res.statusText); //unhandled error

        const msg = await res.json();
        //don't return err here.  we want to continue with the delete
        //return err(new Error(`rtsp  ${msg.payload}`));
      }

      return ok({});
    } catch (e) {
      await this.log_error({
        level: "serious",
        kind: `delete_rtsp_stream for ${cam.name}`,
        summary: `Exception: ${e.message}`,
        data: JSON.stringify(e),
      });

      return await Promise.reject(e);
    }
  }

  async add_rtsp_stream(cam: CameraData): Promise<Result<any, Error>> {
    try {
      const url = `${this.rtsp_api_url}/stream/${cam.name}/add`;
      //we are only supporting one channel for now
      const rtsp_info: RTSPStreamInfo = {
        id: undefined, //this will be set by the api
        name: cam.name,
        channels: {
          "0": {
            name: "ch1",
            url: cam.rtsp_url,
            on_demand: true,
            debug: false,
            status: 0,
          },
        },
      };

      const res = await fetch(url, {
        method: "POST",
        body: JSON.stringify(rtsp_info),
      });
      if (!res.ok) {
        await this.log_error({
          level: "serious",
          kind: "add_rtsp_stream",
          summary: `could not add rtsp stream: ${cam.name} for ${res.url}`,
          data: JSON.stringify(res),
        });

        return err(new Error(res.statusText));
      }

      return ok({});
    } catch (e) {
      await this.log_error({
        level: "serious",
        kind: "add_rtsp_stream",
        summary:
          `Exception could not add rtsp stream: ${cam.name} ${e.message}`,
        data: JSON.stringify(e),
      });
      return await Promise.reject(e);
    }
  }

  //grabbing all the streams but really should just pull the single.
  async merge_rtsp_stream(cam: CameraData): Promise<Result<CameraData, Error>> {
    try {
      const rtsp_list_res = await this.get_rtsp_streams();
      if (rtsp_list_res.kind == "err") {
        return err(rtsp_list_res.error);
      }
      const rtsp_list = rtsp_list_res.value;

      const rtsp_stream = rtsp_list.find((rtsp) => rtsp.name == cam.name);
      if (rtsp_stream) {
        cam.rtsp_stream_info = rtsp_stream;
      }

      return ok(cam);
    } catch (e) {
      await this.log_error({
        level: "serious",
        kind: "merge_rtsp_streams",
        summary: `Exception: ${e.message}`,
        data: JSON.stringify(e),
      });
      return await Promise.reject(e);
    }
  }
  //get all the rtsp streams from rtsp api, these should be in sync with the cameras. What if they aren't?
  async merge_rtsp_streams(
    cam_info: CameraInfo,
  ): Promise<Result<CameraInfo, Error>> {
    try {
      const rtsp_list_res = await this.get_rtsp_streams();
      if (rtsp_list_res.kind == "err") {
        return err(rtsp_list_res.error);
      }
      const rtsp_list = rtsp_list_res.value;
      //merge the rtsp streams with the camera info
      cam_info.available_cams?.forEach((cam) => {
        const rtsp_stream = rtsp_list.find((rtsp) => rtsp.name == cam.name);
        if (rtsp_stream) {
          cam.rtsp_stream_info = rtsp_stream;
        }
      });

      return ok(cam_info);
    } catch (e) {
      await this.log_error({
        level: "serious",
        kind: "merge_rtsp_streams",
        summary: `Exception: ${e.message}`,
        data: JSON.stringify(e),
      });
      return await Promise.reject(e);
    }
  }
}
