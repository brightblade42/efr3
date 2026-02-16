<script lang="ts">
	import Nav from '$lib/components/Nav.svelte';
	import { auth_store } from '$lib/auth_store.ts';
	import {appStore, uiStore} from '$lib/app_store.ts';
	import { goto } from '$app/navigation';
	import CamSettings from "$lib/components/CamSettings.svelte";
	import SingleCamView from "$lib/components/SingleCamView.svelte";
	import FRAlertDlg from "$lib/components/FRAlertDlg.svelte";
	import Notifications from "$lib/components/Notifications.svelte";
	import DeleteDlg from "$lib/components/DeleteDlg.svelte";
	//import { createDialog } from '@rgossiaux/svelte-headlessui'; //'svelte-headlessui'
	//import Transition from 'svelte-transition';

	//const dialog = createDialog({ label: 'Payment Success' });
 $:{
	 console.log("This is where we start")
	 appStore.connect_server();

 }
	/*
	if ($auth_store.type !== 'LoggedIn') {
		console.log('go to login page now');
		setTimeout(() => {
			//goto('login');
			goto('login');
		}, 1000)
	}

	 */
</script>

<!--
{#if $auth_store.type === 'LoggedIn'}
-->
	<div id="app-root" class="relative">
		<!--<Nav settings_dialog={dialog} /> -->
		<Nav />

		<!--
		<CamSettings open="{$uiStore.is_cam_settings_open}"  />
		-->
		<Notifications />
		<FRAlertDlg />
		<CamSettings/>
		<SingleCamView/>
		<div class="mt-10">
			<slot />
		</div>
	</div>
<!--
{/if}
-->
