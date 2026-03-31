<script lang="ts">
 import { onMount } from "svelte";
 let current_time;
 //let time_sub;

 onMount(() => {
   const interval = setInterval(() => {
     current_time = new Date().toLocaleTimeString();
   }, 1000);

   return () => clearInterval(interval);
 });

</script>

<div class="flex w-[600px] h-[92.4px] border rounded-2xl
       font-bold text-6xl text-white">
    <span class="m-auto pt-2 pb-2">{current_time}</span>
</div>

<style>
    div {
        background: rgb(0,0, 71)
    }

</style>
