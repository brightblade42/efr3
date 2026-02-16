<script>
	import { onDestroy, onMount } from 'svelte';
	import eye from '$lib/images/eye_logo.png';
	import {appStore, uiStore} from '$lib/app_store';
	import { auth_store } from '$lib/auth_store';
	import { goto } from '$app/navigation';
	import { VideoCameraIcon, CogIcon } from '@rgossiaux/svelte-heroicons/solid';

	/**
	 * @type {{ subscribe: (arg0: (value: any) => void) => any; open: () => void; }}
	 */
	//export let settings_dialog;

	onMount(() => {
		console.log('mounting nav');
	});

	onDestroy(() => {
		console.log('destroying nav');
	});

	function toggle_settings() {
		console.log('toggle settings function called');
		uiStore.toggle_cam_settings();
		//settings_dialog.open();
	}

	function toggle_single_cam_view() {
		console.log('toggle single cam view function called');
		uiStore.toggle_single_cam_view();
	}

	function generate_url() {
		const http_protocol = window.location.protocol === 'https:' ? 'https://' : 'http://';
		const base_url = window.location.hostname;
		let port = window.location.port ? `:${window.location.port}` : '';
		return `${http_protocol}${base_url}${port}`;
	}
	async function logout() {
		//$auth_store.type = 'NotLoggedIn';
		appStore.disconnect_server();
		appStore.reset_state();
		let url = generate_url();
		let resp = await fetch(url + "/logout",
			{
				method: "GET",
			});
		location.reload();
		//goto('login');
	}

	function close_single_cam_view() {
		console.log('close single cam view function called');

		if ($uiStore.is_single_cam_view_open) {
			uiStore.toggle_single_cam_view();
		}
	}
</script>

<!--
  <div id="container" class="fixed inset-x-0 top-0 z-10  flex justify-between items-end  bg-blue-800 text-blue-300 p-1">
  -->
<div id="container">
	<div class="flex space-x-4 items-end">
		<img src={eye} class="inline-block w-[99px] h-[58px] opacity-100" alt="eyemetric" />
		<!-- first link -->
		<a href="/"
		   on:click={close_single_cam_view}
		   class="mt-0 btn-indigo ml-2 uppercase text-sm tracking-wide text-blue-50">
			<div class="flex space-x-2 items-end">
				<VideoCameraIcon class="h-6 w-6" />
				<span class="inline-block ml-2">Cameras</span>
			</div>
		</a>


		<button
				class="btn-indigo mt-0 ml-2 uppercase text-sm tracking-wide text-blue-50"
				on:click={toggle_single_cam_view}
		>
			<div class="flex space-x-2 items-end">
				<VideoCameraIcon class="h-6 w-6" />
				<span class="inline-block">Single Camera</span>
			</div>
		</button>
		<a href="/scratch" class="mt-0 btn-indigo ml-2 uppercase text-sm tracking-wide text-blue-50"
			>The Big Test</a
		>
	</div>

	<div class="flex mr-2">
		<button
			on:click={toggle_settings}
			class="btn-indigo mt-0 ml-2 uppercase text-sm tracking-wide text-blue-50"
		>
			<div class="flex space-x-2 items-end">
				<CogIcon class="h-6 w-6" />
				<span class="inline-block">Settings</span>
			</div>
		</button>
		<button
			class="btn-indigo mt-0 ml-2 uppercase text-sm tracking-wide text-blue-50"
			on:click={logout}>Logout</button
		>
	</div>
</div>

<style>
	#container {
		@apply fixed inset-x-0 top-0 z-10  flex justify-between items-end  bg-blue-800 text-blue-300 p-1;
	}
</style>

