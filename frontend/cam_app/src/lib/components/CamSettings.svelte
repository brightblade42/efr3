<script lang="ts">
	import { ChevronRightIcon, UsersIcon } from '@rgossiaux/svelte-heroicons/outline'; // Assuming there's a Svelte version of heroicons

	import { Button } from 'flowbite-svelte';
	import { TransitionRoot, TransitionChild } from '@rgossiaux/svelte-headlessui';
	import {
		RadioGroup,
		RadioGroupLabel,
		RadioGroupOption,
		RadioGroupDescription
	} from '@rgossiaux/svelte-headlessui'; // Assuming there's a Svelte version of headlessui
	import { appStore, uiStore } from '$lib/app_store';
	import { onDestroy, onMount, tick } from 'svelte';
	import type { CameraData } from '$lib/shared/types';
	import {
		PlusIcon,
		VideoCameraIcon,
		EyeIcon,
		EyeOffIcon
	} from '@rgossiaux/svelte-heroicons/solid';
	import ArrowPathIcon from '$lib/components/icons/ArrowPathIcon.svelte';
	import sortable from '$lib/sortable';
	import DeleteDlg from '$lib/components/DeleteDlg.svelte';

	let videoEl;
	let camname;
	let rtsp_url;
	let entrance_pos;
	let exit_pos;
	let room_pos;
	let none_pos;
	let min_match;
	let detect_mask;
	let current_idx = -1;
	let current_profile = {};

	let active = false;
	let checked = false;
	let status = 'Unknown';
	let delete_msg = '';
	let can_edit = true;
	onDestroy(() => {
		current_profile = {};
		current_idx = 0;
		can_edit = true;
	});

	$: {
		console.log("hello from dollar block");
		if ($appStore.selected_profile) {
			if (current_profile?.data?.name !== $appStore.selected_profile) {
				console.log('set current profile');
				set_current_profile($appStore.selected_profile)
					.then((e) => {
						console.log('promise voodoo');
					})
					.catch((e) => {
						console.log('promise voodoo error', e);
					});
			}

			if (current_profile?.state?.state === 'Updated') {
				console.log('current profile updated');

				if (videoEl) {
					setTimeout(() => {
						console.log('update video stream');
						videoEl.srcObject = null;
						videoEl.srcObject = current_profile.media_data.remote_stream;
					}, 1000);
				}
			}

			can_edit = !current_profile?.state?.state.endsWith('ing');
		}
	}

	function unselect_profile() {
		current_profile = undefined;
		current_idx = -1;
		appStore.deselect_profile();

		if (videoEl) {
			videoEl.srcObject = null;
		}

		camname.value = '';
		rtsp_url.value = '';
		let dir = 1;
		entrance_pos.checked = false;
		exit_pos.checked = false;
		room_pos.checked = false;
		if (dir === 0) {
			exit_pos.checked = true;
		} else if (dir === 1) {
			entrance_pos.checked = true;
		} else {
			room_pos.checked = true;
		}

		min_match.value = 90; //remember to div by 100
		detect_mask.checked = true;
	}

	async function set_current_profile(name: string) {
		console.log('set the current profile to : ', name);
		current_profile = $appStore.cam_profiles.find((profile) => profile.data.name === name);
		current_idx = $appStore.cam_profiles.findIndex((profile) => profile.data.name === name);

		await tick();

		if (!current_profile?.media_data?.remote_stream) {
			console.log('set_current_profile no stream');
			videoEl.srcObject = null;
			populate_form();
			return;
		}

		try {
			if (videoEl) {
				console.log('update video stream');
				videoEl.srcObject = null;
				videoEl.srcObject = current_profile.media_data.remote_stream;
			}
		} catch (e) {
			console.log('select_profile media stream boom', e);
		}

		populate_form();
	}

	function select_next_profile(event) {
		console.log('select_next_profile');

		if (current_profile) {
			let current_order = $appStore.cam_display_order;
			let current_idx = current_order.findIndex((name) => name === current_profile.data.name);
			let next_idx = -1;

			if (event.key === 'ArrowDown') {
				console.log('Down arrow key was pressed!');
				next_idx = current_idx + 1;
				if (next_idx >= current_order.length) {
					next_idx = 0;
				}
				// add your logic for the down arrow key
			} else if (event.key === 'ArrowUp') {
				console.log('Up arrow key was pressed!');
				next_idx = current_idx - 1;
				if (next_idx < 0) {
					next_idx = current_order.length - 1;
				}
				// add your logic for the up arrow key
			}

			let next_name = current_order[next_idx];
			select_profile(next_name);
		}
	}

	function populate_form() {
		//populate form
		camname.value = current_profile.data.name;
		rtsp_url.value = current_profile.data.rtsp_url;
		let dir = current_profile.data.direction;
		entrance_pos.checked = false;
		exit_pos.checked = false;
		room_pos.checked = false;
		if (dir === undefined) {
			none_pos.checked = true;
		} else if (dir === 0) {
			exit_pos.checked = true;
		} else if (dir === 1) {
			entrance_pos.checked = true;
		} else {
			room_pos.checked = true;
		}

		min_match.value = current_profile.data.min_match * 100;
		detect_mask.checked = current_profile.data.fr_stream_settings.detect_mask;
	}

	function select_profile(name: string) {
		appStore.select_profile(name);
	}

	function get_cam_position() {
		let dir = 0;
		if (entrance_pos.checked) {
			dir = 1;
		} else if (exit_pos.checked) {
			dir = 0;
		} else if (room_pos.checked) {
			dir = 2;
		} else {
			dir = 0;
		}

		return dir;
	}

	function update_camera() {
		let cam_data: CameraData = { ...current_profile.data };

		cam_data.fr_stream_settings.detect_mask = true; //detect_mask.checked;

		cam_data.name = camname.value;
		cam_data.rtsp_url = rtsp_url.value;
		cam_data.direction = get_cam_position();
		cam_data.min_match = min_match.value / 100;
		// cam_data.fr_stream_settings.detect_mask= detect_mask.checked
		cam_data.fr_stream_settings.name = camname.value;
		cam_data.fr_stream_settings.source = rtsp_url.value;

		appStore.update_camera(cam_data);
	}

	//clears the form, ready for a new camera
	function add_camera() {}

	function save_new_camera() {
		let cam_data: CameraData = {
			name: camname.value,
			rtsp_url: rtsp_url.value,
			direction: get_cam_position(),
			min_match: min_match.value / 100,
			fr_stream_settings: {
				detect_mask: true
			}
		};
		//name: camname.value,
		//   source: rtsp_url.value,
		appStore.add_camera(cam_data);
	}

	function start_fr() {
		appStore.start_camera_fr(current_profile.data);
	}
	function stop_fr() {
		appStore.stop_camera_fr(current_profile.data);
	}

	let is_delete_dlg_open = false;
	let delete_title = 'Delete Camera';
	function confirm_delete() {
		is_delete_dlg_open = true;
		delete_msg = `Are you sure you want to delete ${current_profile.data.name}?`;
	}
	function cancel_delete() {
		is_delete_dlg_open = false;
	}
	function delete_camera() {
		//confirm first.
		is_delete_dlg_open = false;
		appStore.delete_camera(current_profile.data);
		unselect_profile();
	}
	function on_sorted(items) {
		appStore.reorder_cameras(items);
	}
</script>

<dialog id="my_dlg" on:keydown={select_next_profile} open={$uiStore.is_cam_settings_open}>
	<div id="header" class="flex p-2 bg-slate-200 justify-between h-12 align-middle">
		<div class="text-center">Camera Name</div>
		<Button
			class=" bg-blue-500 hover:bg-blue-800"
			on:click={(e) => {
				unselect_profile();
			}}
		>
			<PlusIcon class="w-4 h-4 mr-2 " />Add New
		</Button>
	</div>

	<DeleteDlg
		open={is_delete_dlg_open}
		title={delete_title}
		on:cancel={cancel_delete}
		on:confirm={delete_camera}
		message={delete_msg}
	/>

	<form method="dialog" id="container1">
		<div class="cam-container divide-x">
			<ul class="cam-list" use:sortable={{ animation: 150, cb: on_sorted }}>
				{#each $appStore.cam_display_order as cam_name (cam_name)}
					<li
						id={cam_name}
						class="flex justify-between text-lg w-full space-x-2 hover:bg-gray-200 cursor-pointer border-2 p-1"
						on:click={() => select_profile(cam_name)}
						class:selected={current_profile?.data?.name === cam_name}
					>
						<span>{cam_name}</span>
						<div class:hidden={current_profile?.data?.name !== cam_name}>
							<ChevronRightIcon class="w-5 h-5 p-0 mt-1" />
						</div>
					</li>
				{/each}
			</ul>

			<div class="details">
				<div class="cam-details">
					<div class="cam-info">
						<div>
							<label for="camname" class="block text-sm font-medium leading-6 text-gray-900"
								>Camera name</label
							>
							<div class="mt-2">
								<div
									class="flex rounded-md shadow-sm ring-1 ring-inset ring-gray-300 focus-within:ring-2 focus-within:ring-inset focus-within:ring-indigo-600 sm:max-w-md"
								>
									<input
										type="text"
										name="camname"
										bind:this={camname}
										autocomplete="camname"
										class="block flex-1 border-0 bg-transparent py-1.5 pl-1 text-gray-900 placeholder:text-gray-400 focus:ring-0 sm:text-sm sm:leading-6"
										placeholder="camera_name"
									/>
								</div>
							</div>
						</div>

						<div>
							<label for="rtsp" class="block mt-2 text-sm font-medium leading-6 text-gray-900"
								>RTSP</label
							>
							<div class="mt-2">
								<div
									class="flex rounded-md shadow-sm ring-1 ring-inset ring-gray-300 focus-within:ring-2 focus-within:ring-inset focus-within:ring-indigo-600 sm:max-w-md"
								>
									<!--
                                    <span class="flex select-none items-center pl-3 text-gray-500 sm:text-sm">rtsp://</span>
                                    -->
									<input
										type="text"
										name="rtsp"
										bind:this={rtsp_url}
										autocomplete="rtsp"
										class="block flex-1 border-0 bg-transparent py-1.5 pl-1 text-gray-900 placeholder:text-gray-400 focus:ring-0 sm:text-sm sm:leading-6"
										placeholder="192.168.3.106/axis/media.amp"
									/>
								</div>
							</div>
						</div>

						<div class="mt-2">
							<label class="text-sm font-semibold text-gray-900">Position</label>
							<fieldset class="mt-2">
								<legend class="sr-only">Position Option</legend>
								<div class="space-y-2 sm:flex sm:items-center sm:space-x-10 sm:space-y-0">
									<div class="flex items-center">
										<input
											bind:this={none_pos}
											name="cam-position"
											type="radio"
											class="h-4 w-4 border-gray-300 text-indigo-600 focus:ring-indigo-600"
										/>
										<label for="none" class="ml-3 block text-sm font-medium leading-6 text-gray-900"
											>None</label
										>
									</div>
									<div class="flex items-center">
										<input
											bind:this={entrance_pos}
											name="cam-position"
											type="radio"
											class="h-4 w-4 border-gray-300 text-indigo-600 focus:ring-indigo-600"
										/>
										<label
											for="entrance"
											class="ml-3 block text-sm font-medium leading-6 text-gray-900">Entrance</label
										>
									</div>
									<div class="flex items-center">
										<input
											bind:this={exit_pos}
											name="cam-position"
											type="radio"
											class="h-4 w-4 border-gray-300 text-indigo-600 focus:ring-indigo-600"
										/>
										<label for="exit" class="ml-3 block text-sm font-medium leading-6 text-gray-900"
											>Exit</label
										>
									</div>
									<div class="flex items-center">
										<input
											bind:this={room_pos}
											name="cam-position"
											type="radio"
											class="h-4 w-4 border-gray-300 text-indigo-600 focus:ring-indigo-600"
										/>
										<label for="room" class="ml-3 block text-sm font-medium leading-6 text-gray-900"
											>Room</label
										>
									</div>
								</div>
							</fieldset>
						</div>

						<div class="mt-4 flex flex-column space-x-8">
							<div class="mr-4 relative flex items-start mt-8">
								<div class="mr-3 text-sm leading-6">
									<label for="match" class="font-medium text-gray-900">Match</label>
								</div>
								<div class="flex h-6 items-center">
									<input
										bind:this={min_match}
										name="match"
										type="number"
										min="50"
										max="99"
										step="5"
										class="h-8 w-20
                                               rounded border-gray-300 text-indigo-600 focus:ring-indigo-600"
									/>
								</div>
							</div>

							<!--
							<div class="relative flex items-start mt-8">
								<div class="mr-3 text-sm leading-6">
									<label for="mask" class="font-medium text-gray-900">Detect Mask</label>
								</div>
								<div class="flex h-6 items-center">
									<input
										bind:this={detect_mask}
										name="mask"
										type="checkbox"
										class="h-4 w-4 rounded border-gray-300 text-indigo-600 focus:ring-indigo-600"
									/>
								</div>
							</div>
						-->
						</div>

					</div>

					{#if current_profile}
						<div class="vid-container">
							<video bind:this={videoEl} crossorigin autoplay />
						</div>
					{:else}
						<div id="vid_placeholder">
							<VideoCameraIcon class="h-32 w-38 text-gray-300" />
							{#if current_profile?.state?.state === 'Updating'}
								<div>Updating...</div>
							{:else if current_profile?.state?.state === 'Saving'}
								<div>Updating...</div>
							{/if}
						</div>
					{/if}
				</div>
				<div class="mt-2 flex">
					{#if current_profile?.state?.state === 'FRConnected'}
						<Button
							class="ml-2 bg-red-500"
							on:click={(e) => {
								stop_fr();
							}}
						>
							<EyeOffIcon class="mr-2 w-6 h-6" /> Stop FR</Button
						>
					{:else if current_profile?.state?.state === 'FRDisconnected'}
						<Button
							class="ml-2 bg-green-500 hover:bg-green-700"
							on:click={(e) => {
								start_fr();
							}}
						>
							<EyeIcon class="w-6 h-6 mr-2" /> Start FR
						</Button>
					{:else if current_profile?.state?.state === 'FRConnecting'}
						<Button disabled class="ml-2  bg-slate-400">
							<ArrowPathIcon cclass="w-6 h-6 mr-2 animate-spin" /> Starting
						</Button>
					{:else if current_profile?.state?.state === 'FRDisconnecting'}
						<Button disabled class="ml-2  bg-slate-400">
							<ArrowPathIcon cclass="w-6 h-6 mr-2 animate-spin" /> Stopping
						</Button>
					{:else}
						<Button
							class="ml-2 bg-green-500 hover:bg-green-700"
							on:click={(e) => {
								start_fr();
							}}
						>
							<EyeIcon class="w-6 h-6 mr-2" /> Start FR
						</Button>
					{/if}

					<!--						 <div class="ml-4">{status}</div>-->
				</div>
			</div>
		</div>
	</form>
	<div id="footer" class="flex justify-end bg-bgray-300 mb-4">
		{#if current_idx !== -1}
			<Button
				class="mr-4 bg-red-500"
				on:click={(e) => {
					confirm_delete();
				}}>Delete</Button
			>
		{:else}
			<Button
				disabled
				class="mr-4 bg-red-500"
				on:click={(e) => {
					confirm_delete();
				}}>Delete</Button
			>
		{/if}
		<!--
        <Button class="mr-2 bg-gray-500" on:click={(e) => { console.log("cancel op")}}>Cancel</Button>
        -->
		{#if current_idx === -1}
			<Button
				class="mr-2 bg-blue-500 hover:bg-blue-800"
				on:click={(e) => {
					save_new_camera();
				}}>Save</Button
			>
		{:else}
			<Button
				disabled={!can_edit}
				class="mr-2 bg-blue-400 hover:bg-blue-800"
				on:click={(e) => {
					update_camera();
				}}>Update</Button
			>
		{/if}
	</div>
</dialog>

<!-- The template part remains largely the same, with Vue-specific syntax replaced by Svelte's syntax. -->

<style>
	.selected {
		background-color: rgb(118, 169, 250);
	}

	dialog {
		@apply rounded-lg shadow-lg;
		z-index: 1000;
		border: 4px solid grey;
		margin: 0 auto;
		margin-top: 20rem;
		height: max-content;
		width: 1200px;
	}

	.confirm {
		margin: 0 auto;
	}

	form {
		display: flex;
		flex-direction: column;
		height: 100%;
	}

	.vid-container {
		position: relative;
	}

	#vid_backdrop {
		position: absolute;
		display: flex;
		margin: 0 auto;
		top: 20%;
		width: 100%;
		z-index: -1;
	}

	#vicon {
		margin: 0 auto;
	}

	video {
		z-index: 200;
	}

	.cam-container {
		margin: 0.1rem;
		height: 100%;
		display: flex;
	}

	.cam-list {
		border: 0px solid #6b7280;
		flex-basis: 15%;
	}
	.details {
		margin: 0.2rem;
	}
	.cam-details {
		display: flex;
		flex-direction: row-reverse;
	}
	.cam-info {
		border: 0px solid green;
		margin: 0 8px;
		padding: 4px;
	}

	.fr-settings {
		display: flex;
		margin: 0.5rem;
		border-top: 1px solid #3b7280;
		max-height: 60%;
	}

	video {
		border: 1px solid #3b7280;
		aspect-ratio: 16/9;
		height: 300px;
	}

	#vid_placeholder {
		@apply flex justify-center items-center;
		border: 1px solid #3b7280;
		aspect-ratio: 16/9;
		min-height: 300px;
	}
	.checked {
		background-color: rgb(191 219 254);
	}
</style>
