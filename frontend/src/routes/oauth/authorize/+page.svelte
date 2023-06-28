<script lang="ts">
    import type { PageData } from "./$types";
    import IconError from "virtual:icons/lucide/x-octagon";
    import IconCheck from "virtual:icons/lucide/check-circle-2";
    import IconApplication from "virtual:icons/lucide/package";
    import IconDisabled from "virtual:icons/lucide/circle";
    import { page } from "$app/stores";
    import { enhance } from "$app/forms";
    import ThemeToggle from "$lib/components/ThemeToggle.svelte";

    export let data: PageData;
    $: {
        const searchParams = $page.url.searchParams;
    }
    const makeParams = (action: string) => {
        const searchParams = $page.url.searchParams;
        searchParams.delete("/authorize_next");
        return "?/" + action + "&" + searchParams.toString();
    };
</script>

<svelte:head>
    <title>Authorize Application</title>
    <meta name="robots" content="noindex" />
</svelte:head>

<div class="flex h100% items-center justify-center">
    <main class="card">
        {#if data.check.success == true}
            <div class="header">
                <span>Authorize Application</span>
                <ThemeToggle />
            </div>
            <div class="flex flex-col justify-center items-center gap-3 mb-3">
                <IconApplication class="w12 h12" />
                <span>{data.check.app_name}</span>
            </div>
            <div>
                <form
                    method="post"
                    action={makeParams("authorize_next")}
                    class="flex flex-col"
                    use:enhance
                >
                    {#each Array.from($page.url.searchParams.entries()) as [name, value], i}
                        <input {name} bind:value hidden readonly />
                    {/each}
                    <div class="flex flex-col mb-3 self-center">
                        {#each data.check.scopes as scope}
                            <!-- svelte-ignore a11y-click-events-have-key-events -->
                            <label class="flex gap-1">
                                <input
                                    name="enable-scope:{scope.name}"
                                    type="checkbox"
                                    class="scope-box"
                                    checked
                                    hidden
                                />
                                <IconCheck class="text-green selected hidden" />
                                <IconDisabled class="disabled hidden" />
                                <span>{scope.description}</span>
                            </label>
                        {/each}
                    </div>
                    {#each data.check.invalid_scopes as scope}
                        <div>
                            <IconError class="text-red" />
                            <span>{scope}</span>
                        </div>
                    {/each}
                    <div class="flex gap-2 justify-end">
                        <form
                            method="post"
                            action={makeParams("deny")}
                            use:enhance
                        >
                            {#each Array.from($page.url.searchParams.entries()) as [name, value], i}
                                <input {name} bind:value hidden readonly />
                            {/each}
                            <button>Cancel</button>
                        </form>
                        <button type="submit">Authorize</button>
                    </div>
                </form>
            </div>
        {:else if data.check.success == false}
            <div class="flex flex-col">
                <span>
                    {data.check.error}
                </span>
                <span>
                    {data.check.error_description}
                </span>
            </div>
        {/if}
    </main>
</div>
{#if data.check.success == true}
    <main class="flex flex-col">
        <span>Authorize Application</span>
        <span>{data.check.app_name}</span>
        <div />
    </main>
{/if}

<style>
    :global(.scope-box:checked + .selected) {
        display: inline;
    }
    :global(.scope-box:not(:checked) + .selected + .disabled) {
        display: inline;
    }

    .card {
        --at-apply: flex flex-col shadow-2xl w25rem box-border p-4 rounded-xl;
    }

    .card .header {
        --at-apply: flex justify-between items-center mb-3;
    }

    .field {
        --at-apply: flex flex-col;
    }
</style>
