<script>
  import Nav from '$lib/components/Nav.svelte';
  import {auth_store} from "$lib/auth_store.ts";
  import { goto } from "$app/navigation";
  if ($auth_store.type !== "LoggedIn") {
    console.log("go to login page now");
    setTimeout(()=> {
      //goto('login');
      goto('login');
    }, 1000)
  }
</script>

{#if $auth_store.type === "LoggedIn"}
<Nav />
<slot></slot>
{/if}