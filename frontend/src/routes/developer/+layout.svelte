<script>
    import NavEntry from "$lib/components/NavEntry.svelte";
    import Shell from "$lib/components/Shell.svelte";
</script>

<Shell>
    <svelte:fragment slot="nav">
        <ul class="list-none">
            <li>
                <NavEntry target="/developer">Home</NavEntry>
            </li>
            <li>
                <NavEntry target="/developer/applications">Applications</NavEntry>
            </li>
        </ul>
    </svelte:fragment>
    <svelte:fragment slot="content">
        <slot />
    </svelte:fragment>
    
</Shell>

<style>
    :global(table) {
        border-collapse: collapse;
    }
    :global(th), :global(td) {
        --at-apply: pl-2 py-1 border-2 border-solid text-left border-light-3 dark:border-dark-1;
        border-color:#dddddd;
    }
    :global(tr:nth-child(even)) {
        --at-apply: bg-light-5 dark:bg-dark-1/40 bg-blend-lighten
    }
</style>
