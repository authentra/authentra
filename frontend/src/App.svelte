<script lang="ts">
    import Flow from "./Flow.svelte";
    import Router from 'svelte-spa-router';
    import { routes } from "./main";
    import { get_user_id } from "./api/api";
    import { location } from 'svelte-spa-router';

    let user_id;
    let locationValue;
    location.subscribe(value => {
        locationValue = value;
    });
    $: locationValue, user_id = get_user_id();
</script>

<div id="content-wrap" style="margin-top: 0;">
    {#await user_id}
	    <p>...waiting</p>
    {:then response}
	    <p>The user id is: {response.data.uid}</p>
    {:catch error}
	    <p style="color: red">{error.message}</p>
    {/await}
    <Router {routes}></Router>
</div>
<div id="footer" style="background-color: red;"></div>

