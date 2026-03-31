import { readable, writable } from "svelte/store";
import { WebSocketClient } from "$lib/websocketclient";
import type {WSMessage, CameraState, CameraInfo, CameraData, PVStreamingApiError, RecognizedResult, FRStreamSettings }  from "$lib/shared/types";

export type AppState  = {

    is_connected: boolean, //TODO: websocket connection. do we need this?
    all_identities: RecognizedResult[],
    identities: RecognizedResult[],
    watch_list: RecognizedResult[],
    max_items: number
    camera_info: CameraInfo,
    cam_profiles: CameraProfile[],
    selected_profile: string | undefined,
    last_cam_error: PVStreamingApiError | undefined, //keep an error stack maybe?
    cam_display_order: string[];
    pending_update: CameraProfile | undefined;
}

export type CameraProfile = {
    data: CameraData,
    state: CameraState //TODO: get rid of this, use state map instead
    media: MediaData | undefined
}

export type MediaData = {
    name: string,
    remote_stream: MediaStream | undefined,
    webrtc_url: string | undefined
}

//TODO: will we use this?
export type CameraSettingsDialog =  {
    selected_cam: CameraProfile | undefined,
    is_open: boolean,
    selected_idx: number | undefined
}

export type UIState = {
    is_cam_settings_open: boolean,
    is_single_cam_view_open: boolean
}


function init_app_state() {

    function load_cam_order(): string[] {
        const order_str = localStorage.getItem("cam_display_order");
        let cam_display_order = [];
        if (order_str) {
           cam_display_order = JSON.parse(order_str);
        }

        return cam_display_order;
    }

   return {
        is_connected: false,
       all_identities: [],
       identities: [],
       watch_list: [],
       max_items: 50,
       camera_info: {},
       cam_profiles: [],
       last_cam_error: undefined,
       cam_display_order: load_cam_order()
   }
}

function init_ui_state() {
    return {
        is_cam_settings_open: false,
        is_single_cam_view_open: false
    }
}

//if browser was refreshed, we need to sync the camera state with current running streams
//this is shit
function sync_camera_state(state: AppState) {
   const live_fr_streams = state.camera_info.live_fr_streams;
   for (const p of state.cam_profiles) {
       //there are live streams, but are we connected at the server end
       const fr_setting = live_fr_streams.find((s: FRStreamSettings) =>  s.name === p.data.name);
         if (fr_setting) { //presence indicates stream is running
                p.state = { name: p.data.name, state: "FRConnected" };
         }
   }
}

function createUIStore() {
    const { subscribe, set, update} = writable(init_ui_state());

     function toggle_cam_settings() {
        update((state: UIState) => {
                state.is_cam_settings_open = !state.is_cam_settings_open;
                return state;
            });
     }

    function toggle_single_cam_view() {
        update((state: UIState) => {
            state.is_single_cam_view_open= !state.is_single_cam_view_open;
            return state;
        });
    }

    return {
        subscribe,
        update, 
        toggle_cam_settings,
        toggle_single_cam_view,
    }
}

function generate_ws_url(): string {
    const ws_protocol = window.location.protocol === 'https:' ? 'wss://' : 'ws://';
    const base_url = window.location.hostname;
    let port = window.location.port ? `:${window.location.port}` : '';
    if (window.location.port === "5173") {
        port = ":3000"
    }
    return `${ws_protocol}${base_url}${port}/wss`;

}
///we need to convert the rtsp url and info to a proxy url
function createAppStore() {

  const { subscribe, set, update} = writable(init_app_state());
  const ws_url = generate_ws_url();
  console.log(ws_url);
  const ws = new WebSocketClient(ws_url);

  console.log("hello there");
  ws.addEventListener("connected", (event: any) => {
        update((state: AppState) => {
            state.is_connected = true;
            return state;
        });

        //first thing we do is ask for the available cameras
        let msg = {"cmd": "GetAvailableCameras", "payload": {}};
        ws.send(JSON.stringify(msg))
    });
    
    ws.addEventListener("disconnected", (event: any) => {
        console.log("socket disconnected: ", event)
        update((state: AppState) => {
            state.is_connected = false;
            return state;
        });
    });

    ws.addEventListener("message", (ev: { detail: string; }) => {

        const msg: WSMessage =  JSON.parse(ev.detail);

        switch (msg.kind) {
            case "AvailableCameras":
                console.log("Genius!");
                update_available_cameras(msg);
                break;
            case "CameraStateChanged":
                update_camera_state(msg);
                break;
            case "FaceIdentified":
                //console.log("face identified");
                update_faces(msg);
                break;
            default:
                console.log("unknown message");
        }
    });

    //STATE Update
    function update_available_cameras(msg: WSMessage) {
        console.log("Available cameras returned!")
        update((state: AppState) => {
            state.camera_info = msg.payload;
            console.log(state.camera_info)
            //remove any profiles that are no longer available
            state.cam_profiles = state.cam_profiles
                .filter(p => msg.payload.available_cams.some(av => p.data.name === av.name));

            state.cam_profiles = msg.payload.available_cams.map((cam: CameraData) => {
                return {data: cam, state: {name: cam.name, state: "None"}};
            });

            //initialize display order.
            if (state.cam_display_order.length === 0) {
                console.log("setting cam display order from  default feed");
                state.cam_display_order = msg.payload.available_cams.map((cam: CameraData) => cam.name);
            }

            sync_camera_state(state);
            //TODO: possibly reconect to any running streams
            //sync_camera_state(state);
            return state;
        });
    }

    // f (array.length > maxLength) {
    //     // Calculate the number of items to keep. This removes items from the end of the array.
    //     const keepLength = maxLength - itemsToRemove;
    //
    //     // Update the existing array by slicing the array from the start to the desired 'keep' length
    //     // The slice() method returns a new array and doesn't modify the original array.
    //     return array.slice(0, keepLength);
    // }
    function update_faces(msg: WSMessage) {
        update((state: AppState) => {
            if (state.identities.length > state.max_items) {
                state.identities = state.identities.slice(0, state.max_items - 10);
            }
            if (state.watch_list.length > state.max_items) {
                state.watch_list = state.watch_list.slice(0, state.max_items - 10);
            }

            //TODO: temp
            if (state.all_identities.length > 200) {
                state.all_identities = state.all_identities.slice(0, state.max_items - 20);
            }

            let rec: RecognizedResult = msg.payload;
            const status = rec.identity.possible_matches.at(0)?.details?.status;

            if (status === undefined || status === null) {
                console.log("No status found for recognized person:  ", status);
                return state;
            }

            rec = fmt_res(rec)
            state.all_identities = [rec, ...state.all_identities];

            if (status.includes("FR")) {
                state.watch_list = [rec, ...state.watch_list];
            } else {
                state.identities = [rec, ...state.identities];
            }

            return state;
        });
    }

    function update_camera_state(msg: WSMessage) {
        const cam = msg.payload.cam;
        const cstate = msg.payload.state;
        console.log(`${cstate.state} : ${cam.name}`);

        update((app_state: AppState) => {

            //we could be adding a new camera
            let profile: CameraProfile | undefined = app_state.cam_profiles.find((p: CameraProfile) => p.data.name === cam.name);
            if (cstate.state === "Added" || cstate.state === "Adding") {
               profile =  {data: cam, state: cstate, media: undefined};
            }
            if (cstate.state === "Updating" ) {
                console.log("camera update pending. the before");
                app_state.pending_update =  {data: cam, state: cstate, media: undefined};
            }

            //updates may have name change and won't be seen as current profile
            if (!profile && cstate.state != "Updated") {
                    console.log("can't update camera state. No profile for camera: ", cam.name);
                    return app_state;
            }

            //this looks stupid now, but we'll need to do more here later
            switch (cstate.state) {
                case "None":
                    profile.state = cstate;
                    break;
                case "FRConnecting":
                    profile.state = cstate;
                    break;
                case "FRConnected":
                    profile.state = cstate;
                    break;
                case "FRDisconnecting":
                    profile.state = cstate;
                    break;
                case "FRDisconnected":
                    profile.state = cstate;
                    break;
                case "Adding":
                    //profile won't alter state yet..
                    console.log("=== Added camera profile =====")
                    console.log(profile)
                    profile.state = cstate;
                    break;
                case "Added":
                    profile.state = cstate;
                    console.log("=== Added camera profile =====")
                    console.log(profile)
                    app_state.cam_profiles = [...app_state.cam_profiles, profile];
                    app_state.cam_display_order = [...app_state.cam_display_order, profile.data.name];
                    break;
                case "Updating":
                    profile.state = cstate;
                    //profile.data = cam;
                    app_state.pending_update = profile;
                    //TODO: if name changed, update display order with new name
                    break;
                case "Updated": {
                    //remove the old profile
                    let pending_update = app_state.pending_update;
                    app_state.cam_profiles = app_state.cam_profiles.filter((p: CameraProfile) => p.data.name !== pending_update.data.name);
                    let order_index = app_state.cam_display_order.indexOf(pending_update.data.name);

                    console.log("===== upate from the server ==========")
                    console.log(cam);
                    //update pending with final values
                    pending_update.state = cstate;

                    pending_update.data = cam;
                    //const copy = JSON.parse(JSON.stringify(original));
                    app_state.cam_profiles = [...app_state.cam_profiles, pending_update];
                    //keep the order, replace the old name with the new one
                    //app_state.cam_display_order = app_state.cam_display_order.filter((p: string) => p !== pending_update.data.name);

                    if (order_index !== -1) {
                        // Replace the name
                        app_state.cam_display_order[order_index] = pending_update.data.name;
                        app_state.cam_display_order = app_state.cam_display_order;
                    }

                    app_state.pending_update = undefined; //done son.
                    break;
                }
                case "Deleting":
                    profile.state = cstate;
                    break;
                case "Deleted":
                    profile.state = cstate;
                    //TODO: what happens if we remove profiles with RTC connections? mem leak?
                    app_state.cam_profiles = app_state.cam_profiles.filter((p: CameraProfile) => p.data.name !== cam.name);
                    app_state.cam_display_order = app_state.cam_display_order.filter((p: string) => p !== cam.name);
                    break;
                case "Error":
                    console.log("!!!! Error:", cam.name);
                    break;
                default:
                    console.log("Server returned unknown camera state: ", cstate.state);
            }
            return app_state;
        });
    }

    function fmt_res(res: RecognizedResult): RecognizedResult {
        const fmt =  `data:image/png;base64,${res.identity.face.images.expanded}`;
        res.identity.face.images.expanded = fmt;
        return res;
    }

    function start_camera_fr(cam: CameraData){
        //test with first available
        ws.send(JSON.stringify( { "cmd": "StartCameraFR",  "payload": cam }));
    }

    function stop_camera_fr(cam: CameraData) {
        console.log("stop camera. whoaaahh nellie!");
        //test with first available
        ws.send(JSON.stringify( { "cmd": "StopCameraFR",  "payload": cam }));  //shouldn't this be kind?
    }

    function update_camera(cam: CameraData) {
        console.log(cam);
        ws.send(JSON.stringify( { "cmd": "UpdateCamera",  "payload": cam }));  //shouldn't this be kind?
        update((state: AppState) => {
            return state;
        });
    }
    function reorder_cameras(cam_list: string[]) {
        console.log("we would REORDER the camera here");
        update((state: AppState) => {
            state.cam_display_order = cam_list;
            return state;
        });

        localStorage.setItem("cam_display_order", JSON.stringify(cam_list));
    }
    function delete_camera(cam: CameraData) {
        console.log("we would DELETE the camera here");
        //update occurs when CameraStateChanged message is received
        ws.send(JSON.stringify( { "cmd": "DeleteCamera",  "payload": cam }));  //shouldn't this be kind?
        // update((state: AppState) => {
        //     return state;
        // });
    }

    function add_camera(cam: CameraData) {
        console.log("we would ADD NEW the camera here");
        console.log(cam);
        ws.send(JSON.stringify( { "cmd": "AddCamera",  "payload": cam }));  //shouldn't this be kind?
        // update((state: AppState) => {
        //     return state;
        // });
    }

    function clear_watchlist() {
        update((state: AppState) => {
            state.watch_list = [];
            return state;
        });
    }

    function clear_identities() {
        update((state: AppState) => {
            state.identities = [];
            return state;
        });
    }

    function select_profile(cam_name: string) {

        update((state: AppState) => {
            state.selected_profile = cam_name;
            return state;
        });
    }
    function deselect_profile() {
        update((state: AppState) => {
            state.selected_profile = undefined;
            return state;
        });
    }


    function connect_server() {
        console.log("------ CALLING CONNECT SERVER")
        ws.open();
    }

    function disconnect_server() {
        ws.close();
    }

    function reset_state() {
        update((state: AppState) => {
            state = init_app_state();
            return state;
        }) ;
    }

  // Return the store
  return {
    subscribe,
    update,
    select_profile: select_profile,
    deselect_profile: deselect_profile,
    start_camera_fr: start_camera_fr,
    stop_camera_fr: stop_camera_fr,
    update_camera: update_camera,
    add_camera: add_camera,
    delete_camera: delete_camera,
    reorder_cameras: reorder_cameras,
    clear_watchlist,
    connect_server,
    disconnect_server,
    reset_state,
    clear_identities
  } 
}

// Usage:ws://your-websocket-url
//export const appStore = createAppStore("ws://localhost:3000/wss");
export const appStore = createAppStore();
//for things like selected items, open dialogs, etc
export const uiStore = createUIStore();


export const settings = readable({
    to_be_determined: true
});


