import type { FrIdentity, VideoAnalysisState, AnalyzedFrame } from "./types";
import { readable, writable } from "svelte/store";
function init_video_store() {
  return {
    snapshot_time: 1300,
    is_analyzing_frame: false,
    selected_match: undefined,
    analyzed_frame: undefined,
  }

}
function create_video_store () {
  const { subscribe, set, update} = writable<VideoAnalysisState>(init_video_store());

  function update_frame(frame: AnalyzedFrame) {
    update((state) => {
      console.log("updating video frame data");
      state.analyzed_frame = frame;
      return state;
    });
  }
  function update_selected_match(match: FrIdentity) {
    update((state) => {
      console.log("selected a match from video frame for lineup");
      state.selected_match = match;
      return state;
    });
  }
  function clear_selected_match() {
    update((state) => {
      console.log("clear video frame selection");
      state.selected_match = undefined;
      return state;
    });
  }
  function reset() {
    set(init_video_store());
  }
  return {
    subscribe,
    reset,
    update,
    update_frame,
    update_selected_match,
    clear_selected_match,
  }
}
export const video_store = create_video_store();


//maybe
export const video_settings = readable({
  //is_prod: true
  is_recognizing: false
});
