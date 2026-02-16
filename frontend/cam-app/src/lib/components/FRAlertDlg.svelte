<script>
    import {fralert_store} from "$lib/fralert_store";
    import { Button, Modal, P } from 'flowbite-svelte';

	export let message = "Are you sure you want to send an FR alert?";
	export let title = "FR Alert"

    $: {
        console.log("Show ALERT: ", $fralert_store.is_alerting);
    }

    function send_alert() {
        //$fralert_store.send_alert();
        console.log("THE ALERT IS SENDING");
        fralert_store.send_alert();
    }

    function abort_alert() {
        fralert_store.abort_alert();
    }

</script>

<dialog open={$fralert_store.is_alerting}>
	<div class="relative w-full max-w-md max-h-full ">
		<div class="relative bg-red-200 dark:bg-gray-700">
			<div class="p-8 pt-6 pb-6 text-center">
				<!-- icon might be nice -->
				<!--
				<h2 class="pb-1" >{title}</h2>
				<svg class="mx-auto mb-4 text-gray-400 w-12 h-12 dark:text-gray-200" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 20 20">
					<path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 11V6m0 8h.01M19 10a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z"/>
				</svg>
				-->
				<h3 class="mb-5 text-xl font-normal text-red-800 dark:text-gray-400">{message}</h3>

				<button type="button"
						on:click={abort_alert}
						class="text-gray-500 bg-white hover:bg-gray-100 focus:ring-4
									focus:outline-none focus:ring-gray-200 rounded-lg border border-gray-200 text-sm font-medium px-5 py-2.5 hover:text-gray-900 focus:z-10 dark:bg-gray-700 dark:text-gray-300 dark:border-gray-500 dark:hover:text-white dark:hover:bg-gray-600 dark:focus:ring-gray-600">Cancel</button>
				<button
					type="button"
					on:click={send_alert}
					class="text-white bg-red-600 hover:bg-red-800 focus:ring-4 focus:outline-none focus:ring-red-300 dark:focus:ring-red-800 font-medium rounded-lg text-sm inline-flex items-center px-5 py-2.5 text-center mr-2">
					Yes
				</button>
			</div>
		</div>
	</div>
</dialog>

<style>

    dialog {
        @apply rounded-lg shadow-lg ;
        z-index: 1000;
        border: 4px solid grey ;
        margin: 0 auto;
        margin-top: 20rem;
        height: max-content;
        width: max-content;
    }
</style>