<script>
  import {createEventDispatcher} from "svelte";
  let dispatch = createEventDispatcher();


  function create_profile(e) {
    dispatch('enroll');
  }
</script>

<div class="card-container">
  <div class="flex justify-between items-baseline py-2 px-1 opacity-80">
    <h1 class="ml-2 uppercase font-semibold text-lg flex-shrink-0 tracking-wider text-gray-600 text-center">Unknown</h1>
  </div>

  <div class="grid grid-cols-2 grid-rows-2 bg-white">
    <div class="col-start-1 row-start-1 row-span-2">
      <slot name="image" ></slot>
    </div>
    <div class="col-start-2 row-start-1 -ml-2 opacity-80">
      <div class={`mt-8 text-2xl text-center font-extrabold text-gray-600 tracking-wide`}>0%
      </div>
    </div>

  </div>

  <div class="flex justify-between items-end items-center bg-bgray-100 h-12 rounded-b-md opacity-80">
    <div class="text-lg ml-2  font-semibold text-bgray-600 tracking-wide "></div>
    <button on:click={create_profile} class="btn-enroll">
        <span>Enroll</span>
    </button>

  </div>
</div>

<style lang="postcss">
  .btn-enroll {
     @apply flex space-x-1 mr-2 border border-gray-900 hover:bg-gray-300 uppercase text-sm font-extrabold bg-gray-100 text-gray-600 py-1 px-2 rounded-md flex-shrink-0;
  }
  .card-container {
      @apply bg-gray-50 mr-2 mt-2 shadow-xl flex flex-col flex-shrink-0 border border-gray-700 rounded-md w-72;
  }
</style>

