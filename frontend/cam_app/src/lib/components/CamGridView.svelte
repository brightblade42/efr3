<script lang="ts">
    import type {AppState} from "$lib/app_store";
    import { appStore } from "$lib/app_store";
    import { CameraConfig, CameraInfo } from "$lib/shared/types";

    let available_cams: CameraConfig[];

    $: available_cams = $appStore.camera_info.available_cams;
    $: enabled_cams = available_cams.find(cam => cam.enabled === true);    

</script>
<!--
  This example requires some changes to your config:
  
  ```
  // tailwind.config.js
  module.exports = {
    // ...
    plugins: [
      // ...
      require('@tailwindcss/aspect-ratio'),
    ],
  }
  ```
-->
<ul role="list" class="ml-4 grid sm:grid-cols-1 grid-cols-2 gap-x-4 gap-y-8  sm:gap-x-6 lg:grid-cols-4 xl:gap-x-8">
    {each cam in available_cams (cam.name)}
    <li class="relative">
      <div class="group aspect-h-7 aspect-w-10 block w-full overflow-hidden rounded-lg bg-gray-100 focus-within:ring-2 focus-within:ring-indigo-500 focus-within:ring-offset-2 focus-within:ring-offset-gray-100">
        <img src="https://images.unsplash.com/photo-1582053433976-25c00369fc93?ixid=MXwxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHw%3D&ixlib=rb-1.2.1&auto=format&fit=crop&w=512&q=80" alt="" class="pointer-events-none object-cover group-hover:opacity-75" />
        <button type="button" class="absolute inset-0 focus:outline-none">
          <span class="sr-only">View details for {cam.name}</span>
        </button>
      </div>
      <p class="pointer-events-none mt-2 block truncate text-sm font-medium text-gray-900">{cam.name}</p>
      <p class="pointer-events-none block text-sm font-medium text-gray-500">{cam.detect_frame_rate}</p>
    </li>
    {/each}
  
    
    <!-- More files... -->
  </ul>
