
import type { Profile } from "./types";

export function default_settings (is_prod:boolean) {


  const urlWithoutPort = window.location.protocol + '//' + window.location.hostname;
  //const port = window.location.protocol === 'https:' ? '443' : '9002'
  const port = window.location.port; //protocol === 'https:' ? '443' : '9002'

  return {
    api_root: `${urlWithoutPort}:${port}/fr/`,
    tpass_root: `${urlWithoutPort}:${port}/tpass/`,
  }
}

export function RemoteApiBuilder(is_prod: boolean) {

  const settings   = default_settings(is_prod);
  const api_root = settings.api_root;
  const tpass_root = settings.tpass_root;

  const create_json_post = (json: any) => {
    return {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(json)
    }

  }

  async function recognize(b: Blob) {

    const endpoint = `${api_root}recognize-faces`;
    //should probably be an arg
    const opts = {
      top_matches: 5,
      include_detected_faces: false
    };

    const form_data = new FormData();
    form_data.append("image", b, "file.jpg");
    form_data.append("opts", JSON.stringify(opts))

    try {

      const res = await fetch(endpoint,
        {
              method: 'POST',
              body: form_data
            });

      return await res.json()

    } catch (e) {
      console.log(e)
    }

  }



  //TODO: why do we clear the image for nprof rather than just,
  //set to undefined
  async function create_profile(profile: Profile) {

    const b = profile.image;
    console.log("===== the blob =====");
    console.log(b);
    profile.image = undefined; //clear this out
    console.log("about to create profile");
    console.log(profile);
    const api_endpoint = `${api_root}create-profile`;
    const form_data = new FormData();

    //tpass new profile requires a slightly different format than
    //the profile type
    const nprof = {
      ccode: profile.ccode,
      compId: profile.compId,
      clntTid: profile.client_type,
      sttsId: profile.status,
      fName: profile.first,
      lName: profile.last,
      base64Image: profile.image, //we're sending this as undefined. why have it?
      type: profile.type,

    }

    console.log("new profile");
    //console.log(nprof);

    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore
    form_data.append("image", b, "file.jpg");
    form_data.append("profile", JSON.stringify(nprof));

    try {
      const res = await fetch(api_endpoint,
        {
          method: 'POST',
          body: form_data
        });

      const json = await res.json();
      //console.log("==== create profile result ===");
      //console.log(JSON.stringify(json))
      //console.log("==== end profile result ===");
      return json
    } catch (e) {
      console.log(e)
    }

  }
/*
  async function edit_profile(profile: Profile) {
    console.log("about to update profile");
    console.log(profile);
    const api_endpoint = `${api_root}edit-profile`;
    //let form_data = new FormData();

    const nprof = {
      ccode: profile.ccode,
      clntTid: profile.client_type,
      sttsId: profile.status,
      fName: profile.first,
      lName: profile.last,
    }

    try {
      const json_post = create_json_post( nprof );
      const resp = await fetch(api_endpoint, json_post);
      return resp.json();
    } catch (e) {
      console.log(e)
    }

  }

  async function delete_profile(ccode, pv_id) {
    const endpoint = `${api_root}delete-profile`;
    const json_post = create_json_post({
      ccode: ccode,
      pv_id: pv_id
    });

    const resp = await fetch(endpoint, json_post);
    return resp.json();
  }
*/
  async function validate_user (user: string, pwd: string)  {

    const endpoint = `${api_root}validate_user`

    const json_post =
      create_json_post( {
        user: user,
        password: pwd
      });

    const resp = await fetch(endpoint, json_post);
    return resp.json();

  }


  async function get_client_types () {

    const endpoint = `${tpass_root}get-client-types`;
    const resp = await fetch(endpoint);
    return resp.json();
  }

  async function get_status_types () {

    const endpoint = `${tpass_root}get-status-types`;
    const resp = await fetch(endpoint);
    return resp.json()
  }
  async function get_companies () {

    const endpoint = `${tpass_root}get-companies`;
    const resp = await fetch(endpoint);
    return resp.json()
  }

  return Object.freeze({
    root: api_root,
    validate_user,
    get_client_types,
    get_status_types,
    get_companies,
    recognize,
    create_profile,
    //edit_profile,
    //delete_profile,
  });
}
