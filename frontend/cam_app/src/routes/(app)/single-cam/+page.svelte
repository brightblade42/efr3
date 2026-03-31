<script>
    let videoEl; //ref to video element
    //cebdfd81-e3c5-4cf2-8ccd-2c502f217a64 114
    let url = "http://192.168.3.48:8083/stream/cebdfd81-e3c5-4cf2-8ccd-2c502f217a64/channel/0/webrtc";
    //let url = "http://192.168.3.48:8083/stream/27aec28e-6181-4753-9acd-0456a75f0289/channel/0/webrtc";
    let available_cams = [
        {name: "office1", detect_frame_rate: 10}
        // {name: "Cam2", detect_frame_rate: 10},
        // {name: "Cam3", detect_frame_rate: 10},
        // {name: "Cam4", detect_frame_rate: 10},
        ];

    function start_camera() {
        console.log("start camera");
        const webrtc = new RTCPeerConnection({
            iceServers: [{
                urls: ['stun:stun.l.google.com:19302']
            }],
            sdpSemantics: 'unified-plan'
        });

        webrtc.ontrack = function (event) {
            console.log(event.streams.length + ' track is delivered')
            videoEl.srcObject = event.streams[0]
            videoEl.play()
        };

        webrtc.addTransceiver('video', { direction: 'sendrecv' });

        webrtc.onnegotiationneeded = async function handleNegotiationNeeded () {
            const offer = await webrtc.createOffer()
            await webrtc.setLocalDescription(offer)

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
    
     
</script>


<div class="mt-24">

<ul role="list" class="ml-4 grid grid-cols-2 gap-x-2 gap-y-2  sm:gap-x-6 lg:grid-cols-4 xl:gap-x-2">
    {#each available_cams as cam (cam.name)}
    <li class="relative">
      <div class="bg-slate-400 rounded-t-lg text-center">{cam.name}</div>
      <div class="group aspect-h-7 aspect-w-10 block w-full overflow-hidden  rounded-b-md  bg-gray-100 focus-within:ring-2 focus-within:ring-indigo-500 focus-within:ring-offset-2 focus-within:ring-offset-gray-100">
        <!-- 
        <img src="https://images.unsplash.com/photo-1582053433976-25c00369fc93?ixid=MXwxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHw%3D&ixlib=rb-1.2.1&auto=format&fit=crop&w=512&q=80" alt="" class="pointer-events-none object-cover group-hover:opacity-75" />
        -->
    <media-controller>
        <video
            bind:this={videoEl}
            src="https://stream.mux.com/DS00Spx1CV902MCtPj5WknGlR102V5HFkDe/high.mp4"
            slot="media"
            crossorigin
        >
            <track kind="captions" />
        </video>

        <media-control-bar slot="bottom-chrome">
            <media-play-button></media-play-button>
            <media-mute-button></media-mute-button>
            <media-volume-range></media-volume-range>
            <media-time-range></media-time-range>
            <media-pip-button></media-pip-button>
            <media-fullscreen-button></media-fullscreen-button>
        </media-control-bar>
        </media-controller>
      </div>
        <div >
            <button on:click={start_camera}>Start Camera?</button>
        </div>
      <p class="pointer-events-none block text-sm font-medium text-gray-500"> fps: {cam.detect_frame_rate}</p>
    </li>
    {/each}
 
    
    <!-- More files... -->
</ul>
</div>

<style>

</style>