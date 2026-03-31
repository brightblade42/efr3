<script lang="ts">
    import { appStore,uiStore } from "$lib/app_store";
    import {onDestroy, onMount} from "svelte";
   // import  type { CameraData } from "$lib/shared/types";
    import {fmt_date, fmt_time} from "$lib/utils.ts";
    import type { RecognizedResult, Identity,Face, RecognizedStatus } from "$lib/shared/types";

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

</script>
<div class="bg-wgray-50 mt-1 mb-2 shadow-sm border border-red-700 rounded-md">

    <div class="mt-0 flex bg-red-100">

        <img class="mt-0 w-52 h-52 object-center"
             src={recognized_res.identity.face.images.expanded}
        />

        <div class="ml-10  mt-2 ">
            <h1 class="uppercase
                            font-semibold text-4xl
                            text-red-700 ">{details.name}</h1>

            <div class="mt-6 text-3xl
                                font-extrabold
                                text-wgray-700 tracking-wide">{confid}
            </div>
            <div
                    class="mt-8 text-2xl
                         font-bold
                        text-red-700  rounded-md"  >
                {details.status}
            </div>

        </div>

    </div>

</div>
