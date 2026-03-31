<script>
  import {
    Dialog,
    DialogOverlay,
    DialogTitle,
    DialogDescription
  } from "@rgossiaux/svelte-headlessui";
  import {fade} from 'svelte/transition';
  import {settings} from "$lib/store.ts";
  import { logg, safe_call } from "$lib/utils.ts";
  import { profile_store, init_profile} from '$lib/profile_store.ts';
  import {RemoteApiBuilder} from "$lib/remote_api.ts";
  import {createEventDispatcher} from "svelte";
  import CreateProfileForm from "$components/dialog/CreateProfileForm.svelte";
  let dispatch = createEventDispatcher();
  let api = RemoteApiBuilder($settings.is_prod);

  //TODO: consider timeout for hanging request.
  let {in_flight, error, data, execute: commit_profile} = safe_call(api.create_profile);
  let is_open = true;

  export let img;
  function reset_profile() {
    $profile_store = init_profile();
  }
  //create new profile!
  async function save() {

    is_open = false;
    dispatch('close');

    try {
      $profile_store.image = await img_to_blob();
      await commit_profile([$profile_store]); //id like to not have to use an array.

   } catch(e) {
     logg("oopsie on the profile creation", e);
   } finally {
     reset_profile();
   }

  }
  //TODO: this seems to work but can I get the canvas from our slot
  function img_to_blob(){

    let cv = document.createElement("canvas");
    cv.width = img.width;
    cv.height = img.height;
    let ctx = cv.getContext("2d");
    ctx.putImageData(img, 0, 0);        // synchronous

    return new Promise((resolve) => {
      cv.toBlob(resolve,"image/jpeg"); // implied image/png format
    });
  }


  function cancel() {
    is_open = false;
    reset_profile();
    dispatch('close'); //why we need this?
  }

  //don't allow saving a new profile if they don't have a name.
  let save_disabled = true;
  $: save_disabled = ($profile_store.first.trim().length === 0 || $profile_store.last.trim().length === 0);
  $: console.log($error);
</script>

<Dialog  open={is_open}  class="fixed z-10 inset-0 overflow-y-auto" on:close={close}>
  <div transition:fade class="flex items-end justify-center min-h-screen pt-4 px-4  -mt-10 text-center sm:block sm:p-0">
      <DialogOverlay class="fixed top-0 left-0 inset-0 bg-gray-500 bg-opacity-75 transition-opacity" />

    <!-- we may not need this hack. svelte has a build in portal thing -->
    <span class="hidden sm:inline-block sm:align-middle sm:h-screen" aria-hidden="true">&#8203;</span>

      <div class="dialog-content">
        <div>

          <div class="">
            <DialogTitle as="h3" class="text-xl font-medium uppercase text-gray-900">
              Create Profile
            </DialogTitle>

            <hr class="mt-1"/>
            <div class="mt-4">
              <CreateProfileForm >
                <!-- pass the slot down down down -->
                  <slot name="image" slot="image"></slot>
              </CreateProfileForm>

              <div class="flex justify-end mt-4">
                <button type="button" on:click={cancel} class="cancel">Cancel</button>
                {#if $in_flight}
                  <button
                    disabled
                    type="submit"
                    class="save"
                    class:disabled={save_disabled}
                  >
                    Saving..
                  </button>
                {:else}
                    <button disabled="{save_disabled}" type="submit" on:click={save} class="save" class:disabled={save_disabled}>
                      Save
                    </button>
                  {/if}
              </div>

            </div>
          </div>
        </div>
      </div>
  </div>
</Dialog>

<style lang="postcss">
  .save {
      @apply ml-3 inline-flex justify-center py-2 px-4 border border-transparent shadow-sm text-sm font-medium rounded-md text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500;
  }
  .cancel {
      @apply bg-white py-2 px-4 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500;
  }
  .dialog-content {
      @apply inline-block align-bottom bg-white rounded-lg px-4 pt-5 pb-4 text-left overflow-hidden shadow-xl transform transition-all sm:my-8 sm:align-middle sm:max-w-4xl sm:w-full sm:p-6;
  }
  .disabled {
      @apply bg-gray-500;
  }
</style>



