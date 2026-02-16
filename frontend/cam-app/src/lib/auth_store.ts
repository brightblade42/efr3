import { writable } from "svelte/store";
import type {Credentials} from "$lib/shared/types.ts";

export type LoginState =
  | { type: "NotLoggedIn"  | "InFlight" } //| "LoggedIn" }
  | { type: "LoggedIn"; role: string}
  | { type: "Failed"; msg: string }


function generate_url(): string {
    const http_protocol = window.location.protocol === 'https:' ? 'https://' : 'http://';
    const base_url = window.location.hostname;
    let port = window.location.port ? `:${window.location.port}` : '';
    return `${http_protocol}${base_url}${port}`;
}
function create_auth_store() {
    const { subscribe, set, update } = writable<LoginState>({ type: "LoggedIn" });

    const url = generate_url();
    async function login(user: string, password: string) {
        const cred: Credentials = {
          user_name: user,
          password: password
        };

        update((state: LoginState) => {
            state.type = "InFlight";
            return state;
        });

        try {

          let resp = await fetch(url + "/login",
              {
                method: "POST",
                headers: {
                  'Content-Type': 'application/json'
                },
                body: JSON.stringify(cred),
              });

          console.log("hell from login");
          //TOOD: this isn't quite right.
          if (resp.status == 200) {
               console.log("resp is 200");
                let res =  await resp.json();
              console.log(res);
                if (res.status === "LoggedIn") {

                   update((state: LoginState) => {
                        state.type = "LoggedIn";
                        state.role = "admin";
                        return state;
                   });
                } else {
                    update((state: LoginState) => {
                        state.type = "Failed";
                        state.msg = e;
                        return state;
                    });

                }

          }
        } catch (e) {
            update((state: LoginState) => {
                state.type = "Failed";
                state.msg = e;
                return state;
            });
        }


    }

    return {
      subscribe,
        set,
      login,
    }
}
export const auth_store =  create_auth_store();
