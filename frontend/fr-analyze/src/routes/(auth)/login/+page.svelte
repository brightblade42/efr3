<script >
  import eye from '$lib/images/eye_logo.png';
  import { auth_store} from "$lib/auth_store.ts";
  //import {LockClosedIcon } from "@rgossiaux/svelte-heroicons/solid";
  import Fa from 'svelte-fa'
  import { faFlag, faSpinnerThird } from '@fortawesome/pro-solid-svg-icons';   //free-solid-svg-icons'
  import { goto } from "$app/navigation";
  import { profile_store } from "$lib/profile_store.ts";
  import { onDestroy, onMount } from "svelte";
  let user_name = "";
  let password = "";
  let pass_type = "password";

  function handle_login() {
    console.log("handling the login");
    $auth_store.type = "InFlight";
    setTimeout(() => {
      console.log("waited for 2 sec");
      //$auth_store = { type: "Failed", msg: "Bad mojo mofo!"};
      $auth_store = { type: "LoggedIn", role: "admin"};
      console.log($auth_store);
      if ($auth_store.type === "LoggedIn") {
        goto('/');
      }
    }, 2000)
  }

  let login_failed = false;
  let is_login_disabled = true;
  let in_flight = false;
  $: in_flight = $auth_store.type === "InFlight";
  $: is_login_disabled = ($auth_store.type === "InFlight" || (user_name.length < 1 || password.length < 1))
  $: login_failed = ($auth_store.type === "Failed");

  function handleKeydown(event) {
    if (event.code === 'Enter') {
      handle_login();
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
  });


</script>

<div class="bg-bgray-50 min-h-screen bg-white flex flex-col justify-center sm:py-12">
  <div class="-mt-48 p-10 xs:p-0 mx-auto md:w-full md:max-w-md">

    <div
      class="text-lg font-semibold text-red-700 mb-2 bg-red-100 p-2 rounded-lg text-center opacity-0"
      class:failed={login_failed}

    >Incorrect user name or password.</div>
    <div class="shadow-2xl w-full rounded-lg divide-y divide-gray-200">
      <div class="bg-white py-6 mb-2 " >
        <img src="{eye}" class="mx-auto" alt="eyemetric"/>
      </div>


      <div class="bg-gray-100 px-5 py-7">

        <label class="font-semibold text-sm text-gray-700 pb-1 block">User name</label>
        <input type="text" bind:value={user_name}
               class="border rounded-lg px-3 py-2 mt-1 mb-5 text-sm w-full"/>
        <label class="font-semibold text-sm text-gray-700 pb-1 block">Password</label>

        <input type="password"
               bind:value={password}
               class="border rounded-lg px-3 py-2 mt-1 mb-5 text-sm w-full"/>
        <button type="button"
                disabled={is_login_disabled}
                on:click={handle_login}
                class="relative flex justify-center
                                items-center transition duration-200
                                bg-blue-500 hover:bg-blue-600 focus:bg-blue-700
                                disabled:font-bold disabled:bg-gray-400
                                disabled:cursor-not-allowed
                                focus:shadow-sm focus:ring-4 focus:ring-blue-500 focus:ring-opacity-50
                                text-white w-full py-2
                                rounded-lg text-sm
                                shadow-sm hover:shadow-md font-semibold text-center  inline-block">
          <span class="inline-block mr-2 text-lg">Login</span>
          <span class="opacity-0 animate-spin  inline-block ml-1 text-2xl"
                class:in-flight={in_flight}
          >
                  <Fa icon={faSpinnerThird} />
          </span>
        </button>
      </div>

      <div class="bg-bgray-100 py-5">
        <div class="grid grid-cols-2 gap-1">


          <div class="hidden opacity-0 text-center sm:text-right  whitespace-nowrap">

            <button
              class="transition duration-200 mx-5 px-5 py-4 cursor-pointer font-normal text-sm rounded-lg text-gray-500 hover:bg-gray-100 focus:outline-none focus:bg-gray-200 focus:ring-2 focus:ring-gray-400 focus:ring-opacity-50 ring-inset">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"
                   stroke="currentColor" class="w-4 h-4 inline-block align-text-bottom	">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                      d="M18.364 5.636l-3.536 3.536m0 5.656l3.536 3.536M9.172 9.172L5.636 5.636m3.536 9.192l-3.536 3.536M21 12a9 9 0 11-18 0 9 9 0 0118 0zm-5 0a4 4 0 11-8 0 4 4 0 018 0z"/>
              </svg>
              <span class="inline-block ml-1 ">Help</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</div>
<style lang="postcss">
  .failed {
      @apply opacity-100;
  }
  .in-flight {
      @apply opacity-100;
  }
</style>