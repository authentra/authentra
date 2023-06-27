<script lang="ts">
    import type { PageData } from "./$types";
    import IconError from "virtual:icons/lucide/x-octagon";
    import IconCheck from "virtual:icons/lucide/check-circle-2";
    import IconDisabled from "virtual:icons/lucide/circle";
    import { page } from "$app/stores";
    import { enhance } from "$app/forms";
    import { json } from "@sveltejs/kit";

    export let data: PageData;
    $: {
        const searchParams = $page.url.searchParams;
    }
    const makeParams = (action: string) => {
        const searchParams = $page.url.searchParams;
        ;
        searchParams.delete("/authorize_next");
        return "?/" + action + "&" + searchParams.toString();
    };
</script>

<svelte:head>
    <title>Authorize Application</title>
    <meta name="robots" content="noindex">
</svelte:head>

{#if data.check.success == false}
<div class="flex flex-col">
    <span>
        {data.check.error}
    </span>
    <span>
        {data.check.error_description}
    </span>
</div>

{/if}
{#if data.check.success == true}
<main class="flex flex-col">
    <span>Authorize Application</span>
    <span>{data.check.app_name}</span>
    <div>
        <form method="post" action={makeParams('authorize_next')} use:enhance>
            {#each Array.from($page.url.searchParams.entries()) as [name, value], i}
            <input name={name} bind:value={value} hidden readonly/>
            {/each}
            {#each data.check.scopes as scope}
                <!-- svelte-ignore a11y-click-events-have-key-events -->
                <label>
                    <input
                        name="enable-scope:{scope}"
                        type="checkbox"
                        class="scope-box"
                        checked
                        hidden
                    />
                    <IconCheck class="text-green selected hidden" />
                    <IconDisabled class="disabled hidden" />
                    <span>{scope}</span>
                </label>
            {/each}
            {#each data.check.invalid_scopes as scope}
                <div>
                    <IconError class="text-red" />
                    <span>{scope}</span>
                </div>
            {/each}
            <div class="flex gap-2">
                <form method="post" action={makeParams('deny')} use:enhance>
                    {#each Array.from($page.url.searchParams.entries()) as [name, value], i}
                    <input name={name} bind:value={value} hidden readonly/>
                    {/each}
                    <button>Cancel</button>
                </form>
                <button type="submit">Authorize</button>
            </div>
        </form>
    </div>
</main> 
{/if}



<style>
    :global(.scope-box:checked + .selected) {
        display: inline;
    }
    :global(.scope-box:not(:checked) + .selected + .disabled) {
        display: inline;
    }
</style>
