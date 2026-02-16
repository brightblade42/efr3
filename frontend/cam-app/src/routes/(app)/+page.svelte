<script lang="ts">
	import type { RecognizedResult, CameraInfo, CameraData } from '$lib/shared/types';
	import { appStore, uiStore } from '$lib/app_store';
	import sortable from '$lib/sortable';
	import Camera from '$lib/components/Camera.svelte';
	import FaceCard from '$lib/components/FaceCard.svelte';
	import WatchlistCard from '$lib/components/WatchlistCard.svelte';
	//import CamSettings from "$lib/components/CamSettings.svelte";
	import { Button } from 'flowbite-svelte';

	let cam_container;
	function connect_me(e: CustomEvent<CameraData>) {
		console.log('connect event handled:', e.detail);
		appStore.start_camera_fr(e.detail as CameraData);
	}
	function disconnect_me(e: CustomEvent<CameraData>) {
		console.log('disconnect event handled:', e.detail);
		appStore.stop_camera_fr(e.detail as CameraData);
	}

	function clear_watchlist() {
		appStore.clear_watchlist();
	}

	function on_sorted(items) {
		console.log('**** sorted ****', items);
		appStore.reorder_cameras(items);
	}

	function get_profile(name) {
		//console.log($appStore.cam_profiles.length);
		return $appStore.cam_profiles.find((p) => p.data.name === name);
	}

	function toggle_single_cam_view(cam_name) {
		console.log('toggle single cam view function called');
		appStore.select_profile(cam_name);
		uiStore.toggle_single_cam_view();
	}

	import { onMount, onDestroy } from 'svelte';
	import DotsVertical from '@rgossiaux/svelte-heroicons/outline/DotsVertical';

	function handleKeydown(event) {
		// Check for a specific key or keys, for example, the "Escape" key
		if (event.key === 'Escape') {
			console.log('Escape key was pressed!');
			if ($uiStore.is_cam_settings_open) {
				uiStore.toggle_cam_settings();
			}
			if ($uiStore.is_single_cam_view_open) {
				uiStore.toggle_single_cam_view();
			}
			// Your logic here
		}
	}

	// Reactive statement to check the last added object
	$: {
		if ($appStore.watch_list.length > 0) {
			$appStore.watch_list.slice(-1)[0] && hightlightCam($appStore.watch_list.slice(-1)[0]);
		} else {
			unhighlightCams();
		}
	}

	function unhighlightCams() {
		const cams = document.getElementsByClassName('fr-border');
		for (const element of cams) {
			element.classList.remove('fr-border');
		}
	}

	function hightlightCam(lastItem: RecognizedResult) {
		//would be cool to have a flash every time an FR hits.
		const cam_name = lastItem.location;
		const things = document.querySelectorAll('div#' + cam_name);
		things.forEach((div) => {
			if (!div.classList.contains('fr-border')) {
				div.classList.add('fr-border');
			}
		});
	}

	onMount(() => {
		// Add the event listener to the document when the component mounts
		document.addEventListener('keydown', handleKeydown);

		// Return the cleanup function
		return () => {
			// This function will be run when the component is destroyed
			document.removeEventListener('keydown', handleKeydown);
		};
	});

	//style="order: {profile.data.feed_position}"
</script>

<div class="relative mt-[4.1rem]">
	<div class="flex flex-col">
		{#if $appStore.cam_profiles.length === 0}
			<div class="text-7xl m-auto text-gray-200">Cameras</div>
		{:else}
			<div
				bind:this={cam_container}
				class="cam-strip"
				use:sortable={{ animation: 150, cb: on_sorted }}
			>
				{#each $appStore.cam_display_order as cam_name (cam_name)}
					<div id={cam_name} class="cam-box">
						<Camera
							profile={get_profile(cam_name)}
							on:dblclick={(e) => toggle_single_cam_view(cam_name)}
							on:connect={(e) => connect_me(e)}
							on:disconnect={(e) => disconnect_me(e)}
						/>
					</div>
				{/each}
			</div>
		{/if}
		{#if $appStore.identities.length === 0}
			<div
				class="transition md:text-4xl lg:text-7xl m-auto text-green-800 opacity-10 min-h-[300px]"
			>
				Matched Faces
			</div>
		{:else}
			<div
				class="flex overflow-x-scroll text-bgray-200 bg-bgray-100 pt-2 pb-6 mt-0 px-4 space-x-4 min-h-[300px]"
			>
				{#each $appStore.identities as rec (rec)}
					<FaceCard recognized_res={rec} />
				{/each}
			</div>
		{/if}
		{#if $appStore.watch_list.length === 0}
			<div
				class="transition md:text-4xl lg:text-7xl m-auto text-red-800 opacity-10 mt-10 min-h-[300px]"
			>
				Watch List
			</div>
		{:else}
			<div
				class="flex overflow-x-scroll text-wgray-200 bg-wgray-100 pt-0 pb-6 -mt-1 space-x-4 min-h-[300px]"
			>
				{#each $appStore.watch_list as rec (rec)}
					<WatchlistCard recognized_res={rec} />
				{/each}
			</div>
		{/if}
	</div>
	{#if $appStore.watch_list.length > 0}
		<div class="btn-container">
			<Button on:click={clear_watchlist} class="bg-red-600 hover:bg-red-800">Clear Watchlist</Button
			>
		</div>
	{/if}
</div>

<style>
	.cam-box {
		margin-right: 0.1rem;
	}
	.cam-strip {
		display: flex;
		resize: vertical;
		overflow-x: auto;
		overflow-y: hidden;
		max-height: 800px;
		height: 400px;
	}

	.btn-container {
		display: flex;
		justify-content: flex-end;
		padding-right: 1rem;
		width: 100%;
	}
	:global(.fr-border) {
		border: 4px solid red;
	}
</style>
