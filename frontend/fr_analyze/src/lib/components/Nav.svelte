<script>
  import eye from '$lib/images/eye_logo.png';
  import {auth_store} from "$lib/auth_store.ts";
  import { goto } from "$app/navigation";
  import { VideoCameraIcon, CogIcon } from "@rgossiaux/svelte-heroicons/solid";

  function toggle_settings() {
    console.log("toggle settings function called");

  }

  function logout() {
    $auth_store.type = "NotLoggedIn";
    goto('login');
  }

</script>
<div class="fixed inset-x-0 top-0 z-10  flex justify-between items-end  bg-blue-800 text-blue-300 p-1">
  <div class="flex space-x-4 items-end">
    <img src="{eye}"  class="inline-block w-[99px] h-[58px] opacity-100 " alt="eyemetric"/>
    <!-- first link -->
    <a href="/" class="mt-0  btn-indigo ml-2 uppercase text-sm tracking-wide text-blue-50 " >
      <div class="flex space-x-2 items-end">
        <VideoCameraIcon class="h-6 w-6"/>
        <span class="inline-block ml-2">Video</span>
      </div>
    </a>

    <a href="/image_analyzer" class="mt-0  btn-indigo ml-2 uppercase text-sm tracking-wide text-blue-50">Image</a>
  </div>

  <div class="flex mr-2">
    <button on:click={toggle_settings}
      class="btn-indigo mt-0 ml-2 uppercase text-sm tracking-wide text-blue-50"
    >

      <div class="flex space-x-2 items-end">
        <CogIcon class="h-6 w-6" />
        <span class="inline-block">Settings</span>
      </div>

    </button>
    <button class="btn-indigo mt-0 ml-2 uppercase text-sm tracking-wide text-blue-50"
            on:click={logout}
    >Logout</button>

  </div>

</div>