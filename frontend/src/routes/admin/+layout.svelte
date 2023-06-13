<script>
    import AdminShell from "$lib/components/AdminShell.svelte";
</script>

<AdminShell>
    <slot />
</AdminShell>

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
