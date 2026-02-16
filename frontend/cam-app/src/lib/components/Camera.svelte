<script lang="ts">
    import { onMount,afterUpdate, beforeUpdate, onDestroy } from 'svelte';
    import type {CameraData} from "$lib/shared/types.ts";
    import { createEventDispatcher} from "svelte";
    import type {AppState, CameraProfile, MediaData} from "$lib/app_store.ts";
	import {EyeIcon, EyeOffIcon } from "@rgossiaux/svelte-heroicons/solid";
	import ArrowPathIcon from "./icons/ArrowPathIcon.svelte";
	import {appStore} from "$lib/app_store.ts";
	//import { appStore } from "$lib/app_store";
    //import { VideoPlaceholder} from "flowbite-svelte";

    let webrtc: RTCPeerConnection;
    const dispatch = createEventDispatcher();

    let videoEl: HTMLVideoElement; //ref to video element
    export let name = "camera";

    let url  = "";
    export let profile: CameraProfile;
    let fps = 10;

	let eyecolor= "text-green-600";
	let is_recognizing  = false;
    onMount(() =>  {
        name = profile.data.name
        start_camera_display();
    });

	onDestroy(() => {
		//clean up resources.
		if (webrtc) {
			webrtc.close();
			webrtc = null;
			profile.media_data = undefined;
		}
	});

    $: {
        if (videoEl) {
            handleStateChange(profile.state.state);
        }
    }

	let in_flight = false;
    function handleStateChange(state)  {

        if (state === "Updated") {
            //reset webrtc connection.
            start_camera_display();
            //prevents our infinite update loop which I still don't understand
            profile.state.state = "Connected";
            // restoreVideo();
        }

		if (state === "FRConnected") {
			eyecolor= "text-green-600";
			is_recognizing = true;
		} else {
			eyecolor= "text-red-600";
			is_recognizing = false;
		}

		if (state === "FRConnecting" || state === "FRDisconnecting") {
			in_flight = true;
		} else {
			in_flight = false;
		}
    }


	function start_fr () {
		if (!is_recognizing) {
			console.log("CAN WE CLICK THIS on");
			appStore.start_camera_fr(profile.data);
		}
	}
	function stop_fr () {
		if (is_recognizing)
		{
			console.log("CAN WE CLICK THIS off");
			appStore.stop_camera_fr(profile.data);
		}
	}


	function generate_proxy_url(): string {
		const http_protocol = window.location.protocol === 'https:' ? 'https://' : 'http://';
		const base_url = window.location.hostname;
		let port = window.location.port ? `:${window.location.port}` : '';
		if (window.location.port === "5173") {
			return profile.data.proxy_url!;
			//port = ":3000"
		}
		return `${http_protocol}${base_url}${port}/cams`;

	}
    //rtsp connections are converted to a webrtc connection via a proxy server.
    function rtsp_to_webrtc_url() {
        const cam_data = profile.data;
        if (cam_data.proxy_url) {
            //let base_url = cam_data.proxy_url;
			let base_url = generate_proxy_url();
            const name = cam_data.rtsp_stream_info?.name;
            const chan = 0;
            if (!name) {
                console.log("no name for proxy stream. can't show display. must be named");
            } else {
                url = `${base_url}/stream/${name}/channel/${chan}/webrtc`;
            }

        } else {
            console.log("no address for proxy stream. can't show display")
        }
    }

    function start_camera_display() {

        //try to prevent memory leaks
        if (webrtc) {
            console.log("webrtc already exists. closing")
            webrtc.close();
            webrtc = null;
            profile.media_data = undefined;
        }


        webrtc = new RTCPeerConnection();

		const connTimeout = setTimeout(() => {
				if (webrtc.connectionState !== 'connected' && webrtc.connectionState !== 'completed'){
					console.log("failed to connect camera before timeout");
					webrtc.close();
					//alert("check rtsp");
					alert("Connection to " + profile.data.name + " failed. Check rtsp connection");
				}
		}, 10000);


        webrtc.onconnectionstatechange = function (event) {
            let name = profile.data.name;
            //don't know what to do with this yet.
            if (webrtc.connectionState === 'connected') {
                console.log(`${name} webrtc connected`);
            } else if (webrtc.connectionState === 'disconnected') {
                //console.log(`${name} weberc disconnected`);
            } else if (webrtc.connectionState === 'failed') {
				console.log("The webrtc connection failed. should we reconnect?");
            } else if  (webrtc.connectionState === 'closed') {
                console.log(`${name} webrtc closed`);
            } else if (webrtc.connectionState === 'new') {
                ;
            } else {
                ;
            }
        };

        webrtc.ontrack = function (event) {
            let stream = event.streams[0];
            videoEl.srcObject = stream;

            profile.media_data = {
                name: profile.data.name,
                remote_stream: stream,
                webrtc_url: undefined,//rtsp_to_webrtc_url(),
            };

			console.log("Stream stream stream");
        };

        webrtc.addTransceiver('video', { direction: 'sendrecv' });

        webrtc.onnegotiationneeded = async function handleNegotiationNeeded () {
            const offer = await webrtc.createOffer()
            await webrtc.setLocalDescription(offer)
            rtsp_to_webrtc_url();
            fetch(url, {
                method: 'POST',
                body: new URLSearchParams({ data: btoa(webrtc.localDescription.sdp) })
            })
            .then(response => response.text())
            .then(data => {
                try {
                    webrtc.setRemoteDescription(
                    new RTCSessionDescription({ type: 'answer', sdp: atob(data) })
                    )
                } catch (e) {
                    console.warn(e)
                }
            });
        };
    }


	//click stuff.. might make a good action
	let waiting = false;
	let clickType = '';
	let timeout = null;
	export let delay = 600;


	function handleClickType() {
		if (waiting) {
			clearTimeout(timeout);
			dispatch('dblclick');
			waiting= false;
			return;
		}

		waiting = true;
		timeout = setTimeout(() => {
			dispatch('sglclick');
			waiting = false;
		}, delay)
	}

</script>

   <div class="container">
    <div class="header">
		<div class="cam_name">{name}</div>
		{#if is_recognizing}
			<div on:click={stop_fr} class="header_icon" >
				 <EyeIcon class="w-5 h-5 ml-10 mt-1 {eyecolor}"/>
			</div>
		{:else}
			{#if in_flight}
				<div on:click={start_fr} class="header_icon " >
					 <ArrowPathIcon cclass="w-5 h-5 ml-10 -mt-1  animate-spin" />
				</div>
				{:else}

				<div on:click={start_fr} class="header_icon" >
					<EyeOffIcon class="w-5 h-5 ml-10 mt-1 {eyecolor}"/>
				</div>
				{/if}
		{/if}
	</div>
       <div class="place_holder">
           <div class="text-4xl">connecting display...</div>
       </div>

	   <video class="media-controller"
			on:click={handleClickType}
		   bind:this={videoEl}
		   autoplay
		   crossorigin
	   />

<!--    <media-controller  >-->
<!--        <video-->
<!--            bind:this={videoEl}-->
<!--            slot="media"-->
<!--            autoplay-->
<!--            crossorigin-->
<!--        >-->
<!--            <track kind="captions" />-->
<!--        </video>-->

<!--        <media-control-bar slot="center-chrome">-->
<!--            <media-play-button></media-play-button>-->
<!--            <media-mute-button></media-mute-button>-->
<!--            <media-volume-range></media-volume-range>-->
<!--            <media-time-range></media-time-range>-->
<!--            <media-pip-button></media-pip-button>-->
<!--            <media-fullscreen-button></media-fullscreen-button>-->
<!--        </media-control-bar>-->
<!--        </media-controller>-->
        <!--
      <p class="pointer-events-none text-sm text-blue-500 font-medium "> fps: {fps}</p>
    -->
       <!--
    <div  >
        <button class="bg-purple-200" on:click={connect_fr}>Connect Camera</button>
        <button class="bg-red-400" on:click={disconnect_fr}>Disconnect Camera</button>
        <div>{profile.state.state}</div>

    </div>
    -->
   </div>


<style>
    .container {
        position: relative;
        height: 90%;
        margin-right: .2rem;
        aspect-ratio: 16/9;
        min-width: 200px;
    }

    .header {
        @apply text-2xl bg-gray-300;
        text-align: center;
        border-top: 1px solid black;
		position: relative;
    }

	.cam_name {
		margin: 0 auto;
	}

	.header_icon {
		position: absolute;
		top: 0;
		right: 0;
		padding-right: 0.5rem;
		z-index: 10;
	}


    .place_holder {
        @apply text-center bg-blue-300 animate-pulse -mt-1;
        display: flex ;
        top: 0;
        left: 0;
        align-items: center;
        justify-content: center;
        width: 100%;
        height: 100%;
        z-index: -2;
    }

	.media-controller {
		position: absolute;
		left: 0;
		top: 1.8rem;
		aspect-ratio: 16/9;
        width: 100%;
	}


</style>