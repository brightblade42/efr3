import type {Profile} from "./types";
import { readable, writable } from "svelte/store";
import {RemoteApiBuilder} from "./remote_api";
import {safe_call} from "./utils";

function init_tpass_lists() {
  return {
    is_loading: false,
    client_list: [],
    status_list:  [],
    company_list: []
  }
}
function create_tpass_store() {
  const store = writable(init_tpass_lists());

  //const api = RemoteApiBuilder(false); //not production.
  const api = RemoteApiBuilder(true); //not production.
  async function load_client_types() {

    const j_resp = await api.get_client_types();
    const json = await j_resp;
    store.update((state) => {

      state.client_list = json; //is this an array?
      return state;
    });
  }

  return {
    subscribe: store.subscribe,
    set: store.set,
    update: store.update,
    load_client_types,
  };
}



export function init_profile(): Profile{
  return {
    ccode: 0,
    compId: 0, //-1?
    first: "",
    middle: "",
    last: "",
    status: undefined,  //from a list of options provided by TPass.
    client_type:  undefined,
    type:  undefined,
    image: undefined //blob  //base64 or what else?
  }
}

function create_profile_store () {
  //const { subscribe, set, update} = writable<Profile>(init_profile());
  const prof_store = writable<Profile>(init_profile());

  function save_profile(profile: Profile) {
    prof_store.update((state) => {
      console.log("saving local profile state");
      state = profile;
      //do a validation check
      //use fetch, remote api
      return state;
    });
  }

  function reset() {
    prof_store.set(init_profile());
  }

  return prof_store;

}

export let profile_store = create_profile_store();

export let is_saving = writable(false);


