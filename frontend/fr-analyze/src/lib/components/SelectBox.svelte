<script lang="ts">
  import {
    Listbox,
    ListboxButton,
    ListboxOptions,
    ListboxOption
  } from '@rgossiaux/svelte-headlessui';
  import {fade} from 'svelte/transition';
  import { CheckIcon, SelectorIcon } from "@rgossiaux/svelte-heroicons/solid";
  import {createEventDispatcher} from "svelte";

  let dispatch = createEventDispatcher();
  export let options = [{ id: 0, name: ' ' }];
  export let direction = "down";

  let selected = options[0]; //default selected.
  export function set_default(idx) {
    selected = options[idx];
  }
  function on_selected(e) {
      selected = e.detail;
      dispatch('selected', {
        item: selected
      });
  }
</script>


<div class="listbox">
  <Listbox value={selected} on:change={on_selected}>

    <ListboxButton class="button2">
      <span class="block truncate">{selected.name}</span>
      <span class="absolute inset-y-0 right-0 flex items-center pr-2 pointer-events-none">
        <SelectorIcon class="w-5 h-5"/>
      </span>
    </ListboxButton>

    <ListboxOptions class="{direction === 'up' ? 'options-up' : 'options'}">
      {#each options as opt (opt.id)}
        <ListboxOption class="option2" value={opt} let:active let:selected>
          <span class:active class:selected>{opt.name}</span>
        </ListboxOption>
      {/each}
    </ListboxOptions>
  </Listbox>
</div>

<style lang="postcss">
    .listbox {
        @apply mt-1 relative w-full font-semibold;
    }

    .listbox :global(.button2) {
        @apply bg-white relative w-full border border-gray-300 rounded-md shadow-sm pl-3 pr-10 py-2 text-left cursor-default focus:outline-none focus:ring-1 focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm;
    }

    .listbox :global(.arrows2) {
        @apply absolute inset-y-0 right-0 flex items-center pr-2 pointer-events-none;
    }

    .listbox :global(.options) {
        @apply absolute z-20 w-full bg-white shadow-lg max-h-60 focus:outline-none rounded-md py-1 text-base ring-1 ring-black ring-opacity-5 overflow-auto sm:text-sm;

    }

    .listbox :global(.options-up) {
        @apply absolute z-20 w-full bg-white shadow-lg max-h-60 focus:outline-none rounded-md py-1 text-base ring-1 ring-black ring-opacity-5 overflow-auto sm:text-sm;
        @apply -mt-72
    }


    .listbox :global(.option2) {
      @apply w-full cursor-default select-none relative py-2 pl-3 pr-9 text-gray-900;
    }

    .listbox :global(.active) {
        @apply w-full text-white bg-indigo-600;
    }

    .listbox :global(.selected) {
        font-weight: 700;
    }

    .listbox :global(.open-up) {
        @apply -mt-72
    }
/*
    .listbox :global(.selected)::before {
        content: '⭐️ ';
    }
        .listbox :global(.active)::before {
            content: '👉️ ';
        }
    */
</style>
