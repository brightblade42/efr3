<script>
  import { createEventDispatcher, onMount } from "svelte";
  import SelectBox from "$components/SelectBox.svelte";
  let dispatch = createEventDispatcher();
  import {safe_call, is_array, logg} from "$lib/utils.ts";
  import {settings} from "$lib/store.ts";
  import { profile_store} from '$lib/profile_store.ts';
  import {RemoteApiBuilder} from "$lib/remote_api.ts";

  const api = RemoteApiBuilder($settings.is_prod); //remember this is created every time.

  //TODO: how might we make this look nicer?
  let st = safe_call(api.get_status_types);
  let {in_flight: st_in_flight, error: st_error, data: st_data, execute: get_status} = st;
  let {in_flight: comp_in_flight, error: comp_error, data: comp_data, execute: get_comps} = safe_call(api.get_companies);
  let  {in_flight: cl_in_flight, error: cl_error, data: cl_data, execute: get_client_types} = safe_call(api.get_client_types);

  onMount(() => {
     //pull our profile options from server
     get_comps();
     get_status();
     get_client_types();
   })

  //element bindings
  let status_select, client_select, comp_select;
  //passing empty array to select box is problematic, this is fine.
  let status_types = [{id:0, name: "no data"}];
  let client_types = [{id:0, name: "no data"}];
  let comp_types = [{id:0, name: "no data"}];

  $: {
    //some duplication here, extract to function?
    if (is_array($st_data)) {
      status_types = $st_data.map((s) => {
        return {
          id: s.sttsId,
          name: s.description
        }
      });
      if (status_select) {
        status_select.set_default(0);
        $profile_store.status = status_types[0].id
      }
    }

    if (is_array($cl_data)) {
      client_types = $cl_data.map((item ) => {
        return {
          id: item.clntTid,
          name: item.description
        }
      });
      if (client_select) {
        client_select.set_default(0);
        $profile_store.client_type = client_types[0].id;
        $profile_store.type = client_types[0].name; //TPASS requires the name of client type even with provided id
      }
    }

    if (is_array($comp_data)) {
      comp_types = $comp_data.map((item ) => {
        return {
          id: item.compId,
          name: item.name
        }
      });
      if (comp_select) {
        comp_select.set_default(0);
        $profile_store.compId = comp_types[0].id;
      }
    }
  }

  function on_status_selected(e) {
    $profile_store.status = e.detail.item.id;
  }
  function on_client_selected(e) {
    $profile_store.client_type = e.detail.item.id;
    $profile_store.type = e.detail.item.name;
  }
  function on_comp_selected(e) {
    $profile_store.compId = e.detail.item.id;
  }

</script>

<div class="flex ">

  <div id="profile-image" class="md:col-span-1 w-[14rem]">
    <slot name="image"></slot>
  </div>


  <div class="w-full -mt-8 sm:space-y-5 sm:pt-10">
    <div class="space-y-6 sm:space-y-5">

      <div class="entry-row">
        <label for="first-name">First name</label>
        <div class="mt-1 sm:col-span-2 sm:mt-0">
          <input type="text" name="first-name" id="first-name" bind:value={$profile_store.first} />
        </div>
      </div>

      <div class="entry-row">
        <label for="last-name" >Last name</label>
        <div class="mt-1 sm:col-span-2 sm:mt-0">
          <input type="text" name="last-name" id="last-name"  bind:value={$profile_store.last} />
        </div>
      </div>

      <div class="entry-row">
        <label for="status" >Status</label>
        <div class="mt-1 sm:col-span-2 sm:mt-0">
          {#if $st_in_flight}
            <div class="selectbox">loading status</div>
          {:else}
            <div id="status" class="selectbox">
            <SelectBox bind:this={status_select} options="{status_types}" direction="up" on:selected={on_status_selected} />
            </div>
          {/if}
        </div>
      </div>

      <div class="entry-row">
        <label for="typ" >Type</label>
        <div id="typ" class="mt-1 sm:col-span-2 sm:mt-0">
          {#if $cl_in_flight}
            <div class="selectbox">loading clients</div>
          {:else}
            <div class="selectbox">
            <SelectBox bind:this={client_select} options="{client_types}" direction="up" on:selected={on_client_selected}/>
            </div>
          {/if}
        </div>
      </div>

      <div class="entry-row">
        <label for="loc" >Location</label>

        <div id="loc" class="mt-1 sm:col-span-2 sm:mt-0">
          {#if $comp_in_flight}
            <div class="selectbox">loading companies</div>
          {:else}
            <div class="selectbox">
              <SelectBox bind:this={comp_select} options="{comp_types}"  direction="up" on:selected={on_comp_selected}/>
            </div>
          {/if}
        </div>
      </div>
  </div>
</div>
</div>

<style lang="postcss">
  input {
     @apply block w-full max-w-lg rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:max-w-xs sm:text-sm;
  }
  .selectbox {
      @apply block w-full max-w-lg rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:max-w-xs sm:text-sm;
  }
  label {
      @apply block text-sm font-medium text-gray-700 sm:mt-px sm:pt-2;
  }
  .entry-row {
      @apply sm:grid sm:grid-cols-3 sm:items-start sm:gap-4 sm:border-t sm:border-gray-200 sm:pt-5;
  }

</style>