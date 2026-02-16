<svelte:head>
  <link href="/cropper.min.css" rel="stylesheet" >
  <script src="/cropper.min.js"></script>
</svelte:head>

<script>
  import { onDestroy, onMount } from "svelte";
  import {image_store} from "$lib/store.ts";
  import {createEventDispatcher} from "svelte";

  const dispatch = createEventDispatcher();
  export let image_h = 400;
  let img_data = undefined;
  let is_image_loaded = false;
  let img, cropper;
  //update the css variable for image size
  $: img && img.style.setProperty('--image-height', image_h )

  onMount(() => {
    img.addEventListener('load', init_cropper);
  })

  onDestroy(() => {
    console.log("kill it!");
    img.removeEventListener('load', init_cropper);
  })

  export function get_canvas() {
     return cropper.getCroppedCanvas()
  }

  function init_cropper() {
    console.log("init cropper: imag loaded");
    // eslint-disable-next-line no-undef
    cropper = new Cropper(img, {
      //aspectRatio: 16 / 9,
    crop(event) {
      /*
        console.log(event.detail);
        console.log(event.detail.y);
        console.log(event.detail.width);
        console.log(event.detail.height);
        console.log(event.detail.rotate);
        console.log(event.detail.scaleX);
        console.log(event.detail.scaleY);

       */
      },

    })

  }


  function create_image_from_file(im) {
    //TODO: . I don't know why this works but fixes issue where new images
    // dont load into cropper after initial image load.
    image_store.reset();
    const reader = new FileReader();
    const listen = () => {
      if(cropper) cropper.destroy()
      img_data = reader.result;
    }
    reader.addEventListener("load", listen, true); //true means remove after one run.
    if (im) reader.readAsDataURL(im); //fires event that calls listen
    is_image_loaded = true;
    dispatch('loaded');
  }

</script>

<div class="">
  <input type="file" accept="image/*"
         on:change="{(e) => create_image_from_file(e.target.files?.item(0))}" />
  <div class="mt-4 ">
      <img bind:this={img} class:visible="{is_image_loaded}"   src={img_data} alt="an image"/>
      <div class=" mt-12 flex-shrink-0 transition md:text-4xl lg:text-7xl text-green-800 opacity-10 "
           class:invisible={is_image_loaded}
      > Image  </div>
  </div>
</div>

<style>
    img {
        --image-height: inherit;
        display: none;
        height: calc( var(--image-height) * 1px);
        /* this is important somehow */
        max-width: 100%;
    }
    .visible {
        display: block ;
    }

    .invisible {
        display: none;
    }

</style>
