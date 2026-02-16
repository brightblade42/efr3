<script lang="ts">
  import { fmt_date, fmt_time } from "$lib/utils";
  import type { RecognizedResult, Identity,Face, RecognizedStatus } from "$lib/shared/types";
  export let recognized_res: RecognizedResult;
  const best_match = recognized_res.identity.possible_matches.at(0);
  const confid = fmt_confidence(best_match?.confidence);
  const details = best_match?.details;
  const recognized_status = recognized_res?.status;
  const match_date = fmt_date(recognized_res?.match_time);
  const status_time = recognized_status?.time_stamp;//status time trumps match time
  const match_time = recognized_res?.match_time;
  //show status (check in / checkout ) time if available, otherwise show actual camera match time
  let display_time = status_time ? fmt_time(status_time) : fmt_time(match_time);

  export function fmt_confidence (conf: number | undefined)  {
    if (conf === undefined) return "0%";
    const truncated = parseFloat(conf.toString().slice(0, (conf.toString().indexOf(".")) + 5)) * 100;
    return (conf >= 1) ? "100%" : `${truncated.toFixed(2)}%`;
  }

  function fmt_status(status: RecognizedStatus) {
      let display_status = "";
      if (status?.kind) {
          if (status.kind.toLowerCase().includes("in")) {
              display_status = "CHECKED IN"
          } else {
              display_status = "CHECKED OUT"
          }
      }

      return display_status;
  }

  const status = fmt_status(recognized_status);


</script>


<div  class="bg-gray-50 mt-4 shadow-xl flex flex-col flex-shrink-0 w-96 h-auto border border-green-700 rounded-md">

    <div class="flex justify-between items-baseline py-2 px-1 ">
        <h1 class="ml-1 uppercase font-semibold text-lg flex-shrink-0 tracking-wider text-green-900 text-center">{details.name}</h1>
    </div>

    <div class="grid grid-cols-2 grid-rows-2 bg-white">

        <img class="mt-0 w-44 h-52 object-cover object-center col-start-1 row-start-1 row-span-2"
            src={recognized_res.identity.face.images.expanded} alt="face" 
        />

        <div class="col-start-2 row-start-1 ">
            <div class="mt-8 text-2xl text-center font-extrabold text-green-800 tracking-wide">{confid}
            </div>

            <div class="justify-self-center pt-2 ">
                <div class="uppercase font-semibold mr-1.5 text-bgray-600 text-md text-center ">{match_date}</div>
                <div class="uppercase font-semibold mr-1.5 text-bgray-600 text-lg text-center ">{display_time}</div>
            </div>
        </div>

    </div>

    <div class="flex justify-between items-end items-center bg-bgray-100 h-12 rounded-b-md">
        <div class="text-lg ml-2  font-semibold text-bgray-600 tracking-wide ">{recognized_res.location}</div>
       
        <div
            class="flex space-x-1 mr-2 border border-green-900 uppercase text-sm font-extrabold bg-green-100 text-green-900 py-1 px-2 rounded-md flex-shrink-0 ">

            {#if recognized_status?.kind }
                {#if recognized_status.kind.toLowerCase().includes("in") }
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 font-bold" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M11 16l-4-4m0 0l4-4m-4 4h14m-5 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h7a3 3 0 013 3v1" />
                    </svg>
                {:else }

                  <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 font-bold" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
                    </svg>

                {/if}
              {/if}
  
            <span>{status}</span>
        </div>
      
    </div>
</div> 

            <!--{inout}-->
<style lang="postcss">
</style>
