<script>
	import {  ChevronRightIcon, UsersIcon } from '@rgossiaux/svelte-heroicons/outline'; // Assuming there's a Svelte version of heroicons
	import { Listbox, ListboxLabel, ListboxOptions, ListboxOption, Dialog, DialogOverlay, Transition, TransitionChild, TransitionRoot} from '@rgossiaux/svelte-headlessui'; // Assuming there's a Svelte version of headlessui
	import { appStore } from "$lib/app_store";
	import {onMount} from "svelte";
	let open = false;
	let cam_profiles  = [];
	let selected_idx = 0;
	let selected_profile = {};
	onMount(() => {
		cam_profiles = $appStore.cam_profiles;
		selected_profile = cam_profiles[selected_idx];
	});

</script>

<h1 class="text-3xl mt-24">The Grid is good</h1>
<button class="btn-enroll">hello</button>

<style>
	button:popover-open {
	width: 200px;
		height: 100px;
		position: absolute;
		inset: unset;
		bottom: 5px;
		right: 5px;
		margin: 0;
	}
</style>