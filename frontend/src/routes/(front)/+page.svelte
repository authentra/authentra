<script lang="ts">
    import { handleMeta } from "$lib/utils";
    import type { PageData } from "./$types";

    export let data: PageData
    handleMeta(data.meta)
</script>
{ JSON.stringify(data) }
<h1>Welcome to SvelteKit</h1>
<p>Visit <a href="https://kit.svelte.dev">kit.svelte.dev</a> to read the documentation</p>
