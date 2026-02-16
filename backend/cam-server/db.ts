import postgres from "https://deno.land/x/postgresjs/mod.js";
import {err, ok, Result} from "./utils.ts";
//import {CameraData, DetectedResult, FRStreamSettings} from "../../shared/types.ts";
import {CameraData, DetectedResult, FRStreamSettings} from "./types.ts";

export interface LogErrorOptions {
    level?: string;
    kind?: string;
    data?: any;
    extra?: any;
    summary?: string;
    // ... potentially many more properties ...
}
export class DB {
    sql: postgres.Sql;

    constructor(host: string, port: number, username: string, password: string ) {
        //TODO: account for connection errors
        //TODO: use env vars
        this.sql = postgres({
            database: "safr",
            host: host,
            username: username,
            password: password,
            port: port
        });
    }

    async get_camera(id: number): Promise<Result<CameraData, Error>> {

        let res: CameraData[] = []
        try {
            res = await this.sql<CameraData[]>`
                 select id, name, rtsp_url, 
                 user_name , password, direction, fr_stream_settings, 
                 min_match from camera  where id = ${id}`;

            if (!res.length)
                return err(new Error(`No camera found in db with id ${id}` ))

            return  ok(res[0]);
        }
        catch(e) {
            console.log("Error getting camera from db", e);
            return Promise.reject(e);
        }
    }
    async get_camera_by_name(name: string): Promise<Result<CameraData, Error>> {

        let res: CameraData[] = []
        try {
            res = await this.sql<CameraData[]>`
                 select id, name, rtsp_url, 
                 user_name , password, direction, fr_stream_settings, 
                 min_match from camera  where name = ${name}`;

            if (!res.length)
                return err(new Error(`No camera found in db with id ${name}` ))

            return  ok(res[0]);
        }
        catch(e) {
            console.log("Error getting camera from db", e);
            return Promise.reject(e);
        }
    }
    async get_cameras(): Promise<CameraData[]> {

        let res: CameraData[] = []
        try {
            res = await this.sql<CameraData[]>
                `select id, name, rtsp_url,  user_name , password, direction, fr_stream_settings, min_match from camera  order by feed_position`;
        }
        catch(e) {
            console.log("Error getting cameras from db", e);
        }
        return res

    }

    async camera_exists(name: string): Promise<boolean> {
        const res = await this.sql<CameraData[]>`select id from camera where name=${name}`;
        return res.length > 0;
    }

    async get_last_id(): Promise<number> {
        const res = await this.sql<CameraData[]>`select id from camera order by id desc limit 1`;
        if (res.length > 0) {
            return res[0].id || 0 ;
        }
        return 0;
    }

    async delete_camera(name: string): Promise<CameraData[]> {
        const res = await this.sql<CameraData[]>`
            delete from camera where name=${name} returning *`;
        return res;
    }

    //async log_error(level: string, kind: string, data: any, extra?: any, summary?: string ): Promise<void> {
    async log_error(params: LogErrorOptions): Promise<void> {

        const { level = "info", kind = "error", data = {}, extra = {}, summary = ""} = params;

        try {
            const id_res = await this.sql`
            insert into error_log (level, kind, data, extra, summary) values (${level}, ${kind}, ${data}, ${extra}, ${summary})
            returning id `;
            //don't keep this
            //console.log("error logged id", id_res);
        } catch (e) {
            console.error(`${new Date().toLocaleString()} - error logging error: ${kind} : ${summary} : `, e);
            //TODO: log error logging errors to file as FATAL
        }

    }

    async log_detected_frame(cam: CameraData, dres: DetectedResult) {
        try {
           const _res =  await this.sql`
                insert into detection_log 
                (camera,  data, face_count)
                values (${cam.name} , ${JSON.stringify(dres)}, ${dres.faces.length})`;

        } catch (e) {
           console.error("Error logging detected frame to db", e);
        }
    }

    async update_camera(cam : CameraData): Promise<Result<CameraData[],Error>> {
        try {


            if (cam.id === undefined) {
                throw new Error("Camera id is undefined.");
            }

            const res = await this.sql`
              update camera set ${
                            this.sql(cam, 'name', 'rtsp_url', 'user_name', 'password', 'direction', 'fr_stream_settings', 'min_match')
                        }
              where id = ${ cam.id }
            `;

            return ok([cam]); //why

        } catch (e) {
            console.log("error updating camera db");

            if (e.message.includes("UNDEFINED_VALUE")) {
                console.log("We got undefined values here!");
                return err(new Error("Camera Data provided is incomplete."));
            }
            console.error(e);
            throw e;
        }
    }

    async save_new_camera(camera: CameraData): Promise<Result<CameraData, Error>> {
        try {

            await this.set_camera_defaults(camera);
            console.log("Camera settings for db ===== ");
            console.log(camera);
            //TODO: check if camera already exists

            //otherwise an escaped string is inserted. why? javascript, that's why
            const fr_settings = camera.fr_stream_settings; //JSON.stringify(camera.fr_stream_settings);
            //@ts-ignore
            const res = await this.sql`
            insert into camera (
                name,
                rtsp_url,
                user_name,  
                password,
                direction,
                fr_stream_settings,
                min_match
              ) values (
                ${camera.name},
                ${camera.rtsp_url},
                ${camera.user_name},
                ${camera.password},
                ${camera.direction},
                ${fr_settings},
                ${camera.min_match}
               )
               returning id `;


             let [{ id } ] = res;
            //console.log(id);
            camera.id = id;
            // ${ JSON.stringify(camera.fr_stream_settings) },
            return ok(camera); //not sure about this.
        } catch (e) {
            console.log("error saving new camera to db: ", camera.name);
            throw e
        }
    }

    async set_camera_defaults(camera: CameraData) {
        //these should default to env vars
        //@ts-ignore
        let default_fr_settings: FRStreamSettings =
            {
                name: camera.name,
                source: camera.rtsp_url,
                detect_frame_rate: 1,
                detect_mask: true,
                enable_tracking: true,
                expanded_image_scale: 1.5,
                rotation: 0,
                skip_identical_frames: true,
                tracking_duration: 20,
                min_frames_per_track: 0,
                max_frames_per_track: -1,
                tracking_min_face_size: 0,
                output_faces_only: true, //env var
                face_acceptability_tracking_threshold: 0.15,
                face_quality_tracking_threshold: 0.25,
                face_similarity_tracking_threshold: 2.8,

            }

        //merge the defaults with the camera settings
        if (camera.fr_stream_settings)
            default_fr_settings = {...default_fr_settings, ...camera.fr_stream_settings};


        if (!camera.feed_position)
            camera.feed_position =  await this.get_last_id() + 1;


        camera.fr_stream_settings = default_fr_settings;
        camera.min_match = camera.min_match || 0.5; //TODO: env var
        //this is bad, mmkay.. very bad
        camera.user_name = camera.user_name || "root";
        camera.password = camera.password || "root";
        camera.direction = camera.direction || 1;
    }
}
