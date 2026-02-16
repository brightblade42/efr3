<script lang="ts">
  import { fmt_date, fmt_time } from "$lib/utils"; 
  import type { RecognizedResult, Identity,Face, RecognizedStatus } from "$lib/shared/types";
  import {fralert_store} from "$lib/fralert_store";
  export let recognized_res: RecognizedResult;
  const best_match = recognized_res.identity.possible_matches.at(0);
  const confid = fmt_confidence(best_match?.confidence);
  const details = best_match?.details;
  const recognized_status = recognized_res.status; 
  const match_date = fmt_date(recognized_res.match_time);
  const match_time = recognized_res.match_time;
  let display_time = fmt_time(match_time);
  
  export function format_confidence (conf: number | undefined)  {
    if (conf === undefined) return "0%";
    const truncated = parseFloat(conf.toString().slice(0, (conf.toString().indexOf(".")) + 5)) * 100;
    return (conf >= 1) ? "100%" : `${truncated.toFixed(2)}%`;
  }

  export function fmt_confidence (conf: number | undefined)  {
    if (conf === undefined) return "0%";
    const truncated = parseFloat(conf.toString().slice(0, (conf.toString().indexOf(".")) + 5)) * 100;
    return (conf >= 1) ? "100%" : `${truncated.toFixed(2)}%`;
  }
  function begin_fr_alert() {

      if(best_match?.ext_id === undefined) {
          console.error("no ccode found for person. this shouldn't happen. aborting alert");
          return;
      }
      //recognized_res.identity.face.images.expanded
      //create our FRAlert payload MSG
      let fr_alert = {
          Type: "FR Alert",
          CompId: details.compId,
          PInfo: best_match?.ext_id,
          Image: recognized_res.identity.face.images.expanded,
      }


      fralert_store.prepare_alert(fr_alert);
      
  }
</script>

<div
    class="bg-wgray-50 ml-4 mt-2 shadow-xl flex flex-col flex-shrink-0 w-96 h-auto border-2 border-red-700 rounded-md">
    <div class="flex justify-between items-baseline py-2 px-1 bg-red-700">

        <h1 class="ml-1 uppercase font-semibold text-lg flex-shrink-0 tracking-wider text-gray-50 text-center">{details.name}
        </h1>
    </div>

    <div class="grid grid-cols-2 grid-rows-2 bg-red-100">
        <img class="mt-0 w-44 h-52 object-cover object-center col-start-1 row-start-1 row-span-2"
            src={recognized_res.identity.face.images.expanded} alt="face" />

        <div class="col-start-2 row-start-1 ">
            <div class="mt-8 text-2xl text-center font-extrabold text-red-800 tracking-wide">{confid}
            </div>

            <div class="justify-self-center pt-2 ">
                <div class="uppercase font-semibold mr-1.5 text-bgray-600 text-md text-center ">{match_date}</div>
                <div class="uppercase font-semibold mr-1.5 text-bgray-600 text-lg text-center ">{display_time}</div>
            </div>
        </div>
    </div>

    <div class="flex justify-between items-end  bg-red-100 h-12 rounded-b-md pb-2">
        <div class="text-lg ml-2  font-semibold text-bgray-600 tracking-wide ">{recognized_res.location}</div>
        <button
            on:click={() => begin_fr_alert()}
            class={`flex space-x-1 mr-2 border border-red-900  uppercase text-sm font-extrabold bg-red-700 text-gray-200 py-1 px-2 rounded-md flex-shrink-0 `}>
            <span>FR Watch</span>
        </button>

    </div>

</div>


<style lang="postcss">
</style>
