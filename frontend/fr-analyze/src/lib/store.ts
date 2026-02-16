import type { FrIdentity, ImageAnalysisState } from "./types";
import { readable, writable } from "svelte/store";
function init_analysis() {
  return {
    //we use these?
    is_analyzing_frame: false,
    is_analyzing_image: false,
    is_detecting_faces: false,
    is_recognizing_faces: false,
    min_identity_confidence: 0.75, //default
    //analyzed_image: undefined
    fr_identities: [],
    selected_match: undefined,
  }

}

function create_analysis_store () {
     const { subscribe, set, update} = writable<ImageAnalysisState>(init_analysis());

     function update_fr_identities(idents: FrIdentity []) {
        update((state) => {
          console.log("updated some state");
          state.fr_identities = idents
          return state;
        });

     }
     function update_selected_match(match: FrIdentity) {
       update((state) => {
         console.log("selected a match for lineup");
         state.selected_match = match;
         return state;
       });
     }
     function clear_selected_match() {
        update((state) => {
          state.selected_match = undefined;
          return state;
        });
     }
     function reset() {
       set(init_analysis());
     }
     return {
       subscribe,
       reset,
       update,
       update_fr_identities: update_fr_identities,
       update_selected_match,
       clear_selected_match,
     }
}
export const image_store = create_analysis_store();


export const settings = readable({
  //is_prod: true
  is_prod: false
});
