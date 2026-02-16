<script lang="ts">
    import {  ChevronRightIcon, UsersIcon } from '@rgossiaux/svelte-heroicons/outline'; // Assuming there's a Svelte version of heroicons
    import {
        TransitionRoot,
        TransitionChild,
    } from "@rgossiaux/svelte-headlessui";
    import {
        RadioGroup,
        RadioGroupLabel,
        RadioGroupOption,
        RadioGroupDescription,
    } from '@rgossiaux/svelte-headlessui'; // Assuming there's a Svelte version of headlessui
    import sortable from '$lib/sortable';
    import { appStore,uiStore } from "$lib/app_store";
    import {onDestroy, onMount} from "svelte";
    import  type { CameraData } from "$lib/shared/types";
    import Clock from "$lib/components/Clock.svelte";
    import FacelistItem from "$lib/components/FacelistItem.svelte";
    import WatchlistItem from "$lib/components/WatchlistItem.svelte";
    let videoEl;
    let min_match;
    //example for possible search / filter of cam list.
    //$: filteredPeople = query === '' ? [] : people.filter(person => person.name.toLowerCase().includes(query.toLowerCase()));
    let cam_profiles  = [];
    let current_idx = 0;
    let current_profile = {};

    onMount(() => {
        console.log("setting dialog onMount");
    });

	$: {
		if ($appStore.selected_profile) {
			if (current_profile?.data?.name !== $appStore.selected_profile) {
				console.log("set current profile");
				set_current_profile($appStore.selected_profile);
			}
		}

		if (current_profile?.state?.state === "Updated") {
			console.log("current profile updated");

			if (videoEl) {
				setTimeout(() => {
					console.log("update video stream");
					videoEl.srcObject = null;
					videoEl.srcObject = current_profile.media_data.remote_stream;
				}, 1000);
			}
//				set_current_profile(current_profile.data.name);
		}
	}

    onDestroy(() => {
        console.log("setting dialog onDestroy");
        current_profile = {};
        current_idx = 0;
    });

    function on_sorted(items) {
        appStore.reorder_cameras(items);
    }


	function set_current_profile(name: string) {

		console.log("set the current profile to : ", name);
		current_profile = $appStore.cam_profiles.find((profile) => profile.data.name === name);
		current_idx = $appStore.cam_profiles.findIndex((profile) => profile.data.name === name);
		if (!current_profile?.media_data?.remote_stream) {
			console.log("set_current_profile no stream")
			return;
		}

		try {
			if (videoEl) {
				console.log("update video stream");
				videoEl.srcObject = null;
				videoEl.srcObject = current_profile.media_data.remote_stream;
			}
		} catch (e) {
			console.log("select_profile media stream boom", e);
		}

	}

    // function unselect_profile() {
    //     current_profile = {};
    //     current_idx = -1;
    //     if (videoEl) {
    //         videoEl.srcObject = null;
    //     }
	//
    // }

    function select_profile(name: string) {
		appStore.select_profile(name);
		console.log("selected new profile: ", name);
    }

	function select_next_profile(event) {
		console.log("select_next_profile");
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

    function filter_faces() {
        if (current_profile)
           return $appStore.watch_list.filter((rec) => rec.location === current_profile.data.name);
        else
            return [];
    }


</script>

<dialog id="my_dlg"
		on:keydown={select_next_profile}
		open={$uiStore.is_single_cam_view_open} class="rounded-lg ">
        <div class="container">

            <ul class="cam-list" use:sortable={{animation: 150, cb: on_sorted}} >
                <!-- {#each $appStore.cam_profiles as profile (profile.data.name)} -->
                {#each $appStore.cam_display_order as cam_name (cam_name)}
                    <li id="{cam_name}" class="flex justify-between text-lg w-full space-x-2 hover:bg-gray-200 cursor-pointer border-2 p-2"
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

            <div class="cam-view">
                    <video bind:this={videoEl} crossorigin autoplay />
                    <div class="clock-wrapper">
                        <Clock />
                    </div>
            </div>

            <div class="fr-settings ">
                {#each $appStore.all_identities as rec (rec)}
                    {#if rec.location === current_profile?.data?.name}
                        <FacelistItem recognized_res={rec } />
                    {/if}
                {/each}
            </div>
        </div>
</dialog>
<style>

	.selected {
		background-color: darkseagreen;
	}
    #btn1 {
        margin-top: 2rem;
    }

    dialog {
        z-index: 100;
        margin-top: 0.2rem;
        border: 0px solid green;
        margin: 0 auto;
        width: 100%;
    }
    dialog::backdrop {
        background-color: slategray;
    }


    .container {
        margin-top: 0.2rem;
        display: flex;
        border: 0px dashed blue;
        min-width: 100%;
    }

    .cam-list {
        margin-top: 0.2rem;
		flex-basis: 10%;
    }

    .cam-view {
        margin-top: 0.2rem;
        display: flex;
        flex-grow: 1;
        flex-direction: column;
        resize: vertical;
        aspect-ratio: 16/9;
        height: 450px;
        margin-right: 0.5rem;
        background-color: #9ca3af;
    }


    .clock-wrapper {
        margin: 1rem auto;
    }

    .fr-settings {
        flex-grow: 2;
        display: flex;
        flex-direction: column;
        height: 95vh;
        background-color: #f3f4f6;
        overflow-y: scroll;
        }



</style>
<!-- The template part remains largely the same, with Vue-specific syntax replaced by Svelte's syntax. -->
