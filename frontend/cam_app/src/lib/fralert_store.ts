import { readable, writable } from "svelte/store";
import {toasts, addToast, dismissToast} from "$lib/notifications";
import type {RecognizedResult, FRAlert }  from "$lib/shared/types";


export type FRAlertState = {
    is_alerting: boolean;
    fr_alert: FRAlert | undefined;
}

function init_fr_alert_state() {
    return {
        is_alerting: false,
        fr_alert: undefined,
    }
}


function create_fr_alert_store(api_url:string) {

    const { subscribe, set, update} = writable(init_fr_alert_state());
    const api = api_url;
    //only change state to Alerting if it's not already alerting?
    function prepare_alert(alert: FRAlert) {

        //is_alerting.. show an alert ui with a confirm / cancel. 
        //confirm => fire forget. show notification after 200 resp or error response. 

        console.log("ALERT STARTED");
        console.log(alert);
        
        update((state: FRAlertState) => {
            state.is_alerting = true;
            state.fr_alert = alert;
            return state;
        });

    }

    function send_alert() {

        update(async (state: FRAlertState) => {
            //await the api call
            try {
                const json_post = create_json_post(state.fr_alert );
                let resp = await fetch(api, json_post);
                console.log("ALERT RESP");
                console.log(resp);
                addToast({
                    message: "FR Alert sent! ",
                    type: "success",
                    dismissible: false,
                    timeout: 3000});
            } catch (e) {
                console.log(e)
                addToast({
                    message: "Could not send FR Alert! Try again.",
                    type: "error",
                    dismissible: false,
                    timeout: 3000});
            }

            state.is_alerting = false;
            state.fr_alert = undefined;

            return state;
        });
    }

    function abort_alert() {
        update((state: FRAlertState) => {
            state.is_alerting = false;
            state.fr_alert = undefined;
            //await the api call
            return state;
        });
    }


    // async function send_fr_alert(alert: FRAlert) {
    //
    //     const api_endpoint = `${api_root}send-alert`;
    //
    //     try {
    //         const json_post = create_json_post( alert );
    //
    //         let resp = await fetch(api_endpoint, json_post);
    //         return resp.json();
    //     } catch (e) {
    //         console.log(e)
    //     }
    // }


    const create_json_post = (json) => {
        return {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(json)
        }

    }



    return {
        subscribe,
        update, 
        prepare_alert,
        abort_alert,
        send_alert
    }
}

//TODO: remove hard coded values
export const fralert_store = create_fr_alert_store("http://localhost:3000/send-fr-alert");

