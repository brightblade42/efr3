<script>
  //import { image_store } from "$lib/store.ts";
  import { format_confidence } from "$lib/utils.ts";

  export let idx = 0;
  export let match;
  let details;
  //$image_store;
  $: details = match?.possible_matches[idx].details;
  $: confid = format_confidence(match?.possible_matches[idx].confidence);

</script>

<div class="container" on:click>
  <div class="flex justify-between items-baseline py-2 px-1" >
    <h1 class="name">{details.name}</h1>
  </div>

  <div  class="content-grid">
    <div class="col-start-1 row-start-1 row-span-2">
      <slot name="image"></slot>
    </div>
    <div class="col-start-2 row-start-1 -ml-2 ">
      <div class="confidence">{confid} </div>
    </div>

    <div class="col-start-2 row-start-2 -ml-2 -mt-10  -mb-8 text-5xl text-center font-extrabold text-green-800 tracking-wide">
      <slot name="extra"></slot>
    </div>
  </div>
  <div class="flex justify-between items-end items-center bg-bgray-100 h-12 rounded-b-md">
    <div class="footer-left"></div>
    <div class="status">
      <span>{details.status}</span>
    </div>
  </div>
</div>

<style lang="postcss">
    .container {
        @apply bg-gray-50 mt-2 ml-1 mr-1 shadow-xl flex flex-col flex-shrink-0 border-2 border-green-700 rounded-md w-72 ;
    }
    .name {
        @apply ml-2 uppercase font-semibold text-lg flex-shrink-0 tracking-wider text-green-800  text-center;
    }
    .content-grid {
        @apply grid grid-cols-2 grid-rows-2 bg-white;
    }

    .confidence {
        @apply mt-8 text-2xl text-center font-extrabold text-green-800 tracking-wide;
    }

    .footer-left {
        @apply text-lg ml-2  font-semibold text-gray-600  tracking-wide;
    }
    .status {
        @apply flex space-x-1 mr-2 border border-green-900  uppercase text-sm font-extrabold bg-green-100 text-green-900 py-1 px-2 rounded-md flex-shrink-0
    }
</style>
