<script lang="ts">
  //import type {VideoAnalysisState, FrIdentity, AnalyzedFrame} from "$lib/types.ts";
  import { image_store, settings } from "$lib/store.ts";
  import {draw_bounding_box} from "$lib/utils.ts";
  import {video_store} from "$lib/video_store.ts";
  import {onMount, onDestroy} from "svelte";
  import {RemoteApiBuilder} from "$lib/remote_api.ts";
  //i may want a video specific list.
  import FrIdentities from "$components/FrIdentities.svelte";
  import LineupList from "$components/LineupList.svelte";

  let cv;
  let ctx;
  let vid_player;
  let video_src;
  let capture_interval;
  let is_video_loaded = false;
  let api = RemoteApiBuilder($settings.is_prod);

  function createObjectURL ( file ) {
    if ( window.webkitURL ) {
      return window.webkitURL.createObjectURL( file );
    } else if ( window.URL && window.URL.createObjectURL ) {
      return window.URL.createObjectURL( file );
    } else {
      return null;
    }
  }

  function load_video(vid) {
    video_store.reset();
    video_src = createObjectURL(vid);
    is_video_loaded = true;
    vid_player.play();
  }

  function play(){
    console.log("video playing");
  }
  function pause(){
    console.log("video paused");
  }

  onDestroy(() => {
    clearInterval(capture_interval);
    video_store.reset();
  });

  $:{
    capture_interval = setInterval(capture_frame, 1300);
    //onInterval(capture_frame, 1300);
  }

  function clear_selected_match(e) {
    console.log("clearing match");
    video_store.clear_selected_match();
  }
  function capture_frame() {
    if(!is_video_loaded) return;

    if (cv === undefined) {
      console.log("need a canvas to write to");
      return;
    }
    ctx = cv.getContext('2d');
    try {
      cv.width = 2150 //video_width; why is it that size?
      // @ts-ignore
      cv.height = 900;
      //ctx.clearRect(0,0,0,0);
      ctx.drawImage(vid_player, 0, 0);
      //video_store.reset();
      cv.toBlob(async (blob) => {
        //TODO: do we need an inner try?
        const fr_identities = await api.recognize(blob);
        //console.log(possible_matches);
        console.log(fr_identities);
        console.log("I have reached this place");
        //console.log(possible_matches);
        //currently only using the possible_matches property
        let frame = {
          id: 0,
          elapsed_time: 0,
          frame_num: 0,
          //possible_matches: possible_matches,
          fr_identities: fr_identities,
          src_frame: undefined
        }

        if (fr_identities == undefined) {
          console.log("error retrieving fr_identities");
          console.log(fr_identities);
          return;
        }

        video_store.update_frame(frame);
        //we want this to draw after we've extracted faces for match list.
        setTimeout(() => {
          draw_bounding_box(ctx, 0.90, fr_identities);
        }, 25); //delay draw op just a bit. prevents bbox rect from being copies to each face in possible match list.

      }, "image/jpeg");

    } catch(e) {
      console.log("error analyzing video frame", e);
    }

  }
</script>


<div class="flex mt-20">
  <div class="flex flex-col">
    <input class="ml-4 mb-4 text-gray-900 text-lg" type="file" accept="video/*"
           on:change={(e) => load_video(e.target.files?.item(0))} />

      <div class="flex ">
      <div class="flex-shrink-0">
        <video
          style="display: hidden"
          controls
          bind:this={vid_player}
          on:play={play}
          on:pause={pause}
          src="{video_src}"
          loop
          muted
          width="640"
        ></video>
      </div>
      <div>
        <canvas id="vid_capture" bind:this={cv} class="ml-4  h-[360px]"></canvas>
      </div>
    </div>

  </div>

</div>

<div class="flex overflow-x-scroll text-wgray-200 bg-wgray-100 p-0 ml-2 mt-4  min-h-[300px] ">
  <div class="m-auto">
    <FrIdentities {ctx} media_type="video"></FrIdentities>
  </div>
</div>
<div class="flex overflow-x-scroll text-center text-wgray-200 bg-wgray-100 pt-0 ml-2 mt-4 space-x-4 min-h-[300px] m-auto">
  <div class="m-auto">
    <LineupList media_type="video" />
  </div>
</div>
<div class="flex justify-end">
  <button class="btn-light-indigo mt-8 text-lg mr-4" on:click={(e) => clear_selected_match(e)}>Clear Lineup</button>
</div>