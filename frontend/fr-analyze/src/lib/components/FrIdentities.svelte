<script>
  import { image_store } from "$lib/store.ts";
  import {video_store} from "$lib/video_store.ts";
  import FrIdentity from "$components/FrIdentity.svelte";
  import { fade } from "svelte/transition";
  import {logg} from "$lib/utils.ts";
  import { null_undef } from "$lib/utils";

  export let media_type = "image"; //image or video source
  let fr_identities = [];

  //TODO: this is weird to me. perhaps we should have some kind of update callback,
  //or context thing? It's ok for now though
  $: {
    if (media_type === "image") {
      fr_identities = $image_store.fr_identities;
      //logg("image matches changed");
    } else if (media_type === "video") {
      fr_identities = $video_store?.analyzed_frame?.fr_identities;
    } else {
      fr_identities = $image_store.fr_identitites;
      //logg("image matches changed");
    }
  }
  export let ctx = undefined;


</script>

  {#if null_undef(ctx) || (null_undef(fr_identities) || fr_identities.length === 0)}
    <div class="transition md:text-4xl lg:text-7xl text-green-800 opacity-10 text-center mt-12">No Matches</div>
  {:else}
   <div transition:fade={{duration: 100}} class="flex space-x-2 py-4"  >
          {#each fr_identities as match }
            <div >
              <FrIdentity {media_type} {ctx} {match}  />
            </div>
      {/each}
   </div>
  {/if}



