<script>
  //we need settings for min confidence
  import FaceCard from "$components/FaceCard.svelte";
  import WatchlistCard from "$components/WatchlistCard.svelte";
  import { image_store } from "$lib/store.ts";
  import {video_store} from "$lib/video_store.ts";
  import { fade } from "svelte/transition";

  export let media_type = "image";
  let selected_match;

  $: {
    if (media_type === "image") {
      selected_match = $image_store.selected_match;
      console.log("image selected match changed");
    } else if (media_type === "video") {
      selected_match = $video_store?.selected_match;
      //console.log("video selected match changed");
    } else {
      selected_match = $image_store.selected_match;
      console.log("image selected match changed");
    }
  }
</script>


  {#if selected_match === undefined}
    <div class="transition md:text-4xl lg:text-7xl text-green-800 opacity-10 text-center mt-12">Lineup</div>
  {:else}

    <div class="flex space-x-4"  >

      {#each selected_match.possible_matches as pmatch, idx (idx)}
        <div  in:fade>
          {#if pmatch.details.status.includes("FR")}
            <WatchlistCard match={selected_match} {idx}>
              <img class="image" src={pmatch.details.imgUrl} slot="image" />
              <div slot="extra" >{idx+1}</div>
            </WatchlistCard>
            {:else}
            <FaceCard match={selected_match} {idx}>
              <img class="image" src={pmatch.details.imgUrl} slot="image" />
                <div slot="extra" class="mt-6 -mb-8 text-5xl text-center font-extrabold text-green-800 tracking-wide">{idx+1}</div>
            </FaceCard>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
<style lang="postcss">
    .image {
        @apply mt-0 h-52 bg-gray-300 object-cover object-center col-start-1 row-start-1 row-span-2;
    }
</style>
