<script>
  //we need settings for min confidence
  import FaceCard from "$components/FaceCard.svelte";
  import UnknownFaceCard from "$components/UnknownFaceCard.svelte";
  import WatchlistCard from "$components/WatchlistCard.svelte";
  import { logg, null_undef } from "$lib/utils";
  import {onMount, onDestroy } from "svelte";
  import { image_store } from "$lib/store.ts";
  import { video_store} from "$lib/video_store.ts";
  import ProfileDialog from "$components/dialog/ProfileDialog.svelte";

  export let ctx;
  export let match;
  export let media_type = "image";

  let cv;
  let cvp;
  let timeout_id;
  let mod_profile = false; //enroll, edit or delete profile state

  $: {
    match;
    draw_face(cv)
    if (cvp !== undefined)
      draw_face(cvp)
  }
  function get_image_d () {

    const box = match.face.bbox;
    const imgW = 150;

    //these settings seem like we were throwing darts.
    if ((imgW - box.width) < 5) {
        return ctx.getImageData(box.origin.x , box.origin.y , imgW, 175);
    }
    return ctx.getImageData(box.origin.x -45, box.origin.y -25, imgW, 175);
  }

  let img;
  function draw_face(canv) {

    timeout_id = setTimeout(()=> {
      if (null_undef(canv)) return;
      const local_ctx = canv.getContext('2d');
      local_ctx.clearRect(0,0, canv.width, canv.height);
      img = get_image_d();
      local_ctx.putImageData(img, 0,0,10,10, canv.width, canv.height) ;
      //console.log("DRAW NEW FACE!!!!");

    },20);
  }

  function show_dialog() {
    mod_profile = true;
  }
  function dialog_close() {
    mod_profile = false;
  }

  onDestroy(() => {
    if(timeout_id !== undefined)
      clearTimeout(timeout_id)
  })

  $: known = is_known(match);
  function is_known(m) {
    let enroll = m?.possible_matches[0];
    let temp_min = 0.90; //TODO: should be a store setting
    return enroll.confidence >= temp_min;
  }

  $: watchlist = is_watchlist(match);

  function is_watchlist(m){
    let enroll = m?.possible_matches[0];
    console.log("in is watch");
    console.log(enroll);
    return enroll.details.status.includes("FR");
  }

  function select_identity() {
    if (media_type === "image") {
      image_store.update_selected_match(match);
    } else {
      video_store.update_selected_match(match);
    }

  }
</script>
{#if mod_profile}
    <ProfileDialog on:close={dialog_close} {img}>
      <canvas bind:this={cvp} slot="image"></canvas>
    </ProfileDialog>
{/if}

{#if known}
  {#if watchlist}
    <WatchlistCard {match} on:click={select_identity}>
     <canvas bind:this={cv} slot="image"></canvas>
    </WatchlistCard>
    {:else}
   <FaceCard {match} on:click={select_identity}>
      <canvas bind:this={cv} slot="image"></canvas>
    </FaceCard>

    {/if}
{:else}
  <UnknownFaceCard on:enroll={show_dialog}>
   <canvas bind:this={cv} slot="image"></canvas>
  </UnknownFaceCard>
 {/if}
