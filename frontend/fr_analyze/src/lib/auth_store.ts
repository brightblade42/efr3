import { writable } from "svelte/store";

export type LoginState =
  | { type: "NotLoggedIn"  | "InFlight" } //| "LoggedIn" }
  | { type: "LoggedIn"; role: string}
  | { type: "Failed"; msg: string }

export const auth_store = writable<LoginState>({
  type: "NotLoggedIn"
});