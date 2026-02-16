<script>
  import { image_store, settings } from "$lib/store.ts";
  import { RemoteApiBuilder } from "$lib/remote_api.ts";
  import ImageCropper from "$components/ImageCropper.svelte";
  import FrIdentities from "$components/FrIdentities.svelte";
  import LineupList from "$components/LineupList.svelte";
  import {onDestroy} from 'svelte';


  $image_store;
  let img_cropper;
  let ctx = undefined;
  let api = RemoteApiBuilder($settings.is_prod);
  let is_image_loaded = false;
  console.log("api url: " + api.root);
  console.log("is production?: " + $settings.is_prod);

  function clear_list() {
    console.log("hello");
    image_store.reset();
  }

  onDestroy(()=> {
      image_store.reset();
  });

  //use cropped image, send to server for facial recognition
  async function analyze() {

    image_store.reset(); //i don't care for this here.
    console.log("who are you? who who.. i really wanna know. analyzing");
    let cv = img_cropper.get_canvas();
    ctx = cv.getContext('2d'); //cropper context..we might want to store this.
    //we may need the canvas context to cut out the face for displaying in the
    //analyzed image list.
    //cv.toBl
    await cv.toBlob(async (blob) => {
        try {
          console.log("in analyze.. send to server..get face data");
          //TODO: consider moving this into store?
          const fr_identities = await api.recognize(blob);
          console.log(fr_identities);
          if (fr_identities.error !== undefined) {
            console.log("error retrieving possible matches");
            console.log(fr_identities);
            return;
          }

          image_store.update_fr_identities(fr_identities);
          console.log("got some faces, ready to update state");

        } catch(e) {
          console.log("could not build recognition results");
          console.log(e);
        }
    }, "image/jpeg");

  }


</script>

<div class="flex mt-20">
  <div class="min-h-[400px] ">
    <ImageCropper bind:this={img_cropper} image_h="400" on:loaded={() => is_image_loaded = true}/>
    {#if is_image_loaded}
      <div class="flex justify-end ">
        <button class=" btn-light-indigo mt-4 text-xl" on:click={analyze}>Analyze</button>
        <button class=" btn-light-indigo mt-4 ml-4 text-lg" on:click={clear_list}>Clear</button>
      </div>
      {/if}
  </div>
</div>

<div class="flex overflow-x-scroll text-center text-bgray-200 bg-bgray-100 pt-0 ml-2 mt-4 space-x-4 min-h-[300px] m-auto">
  <FrIdentities {ctx}/>
</div>

<div class="flex overflow-x-scroll text-center text-wgray-200 bg-wgray-100 pt-0 ml-2 mt-4 space-x-4 min-h-[300px] m-auto">
  <LineupList />
</div>

