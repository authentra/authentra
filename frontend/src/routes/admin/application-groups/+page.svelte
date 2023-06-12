<script lang="ts">
    import { InternalScopes, type ApplicationGroup } from "$lib/api/admin";
    import type { PageData } from "./$types";
    import IconEdit from "virtual:icons/lucide/edit";
    import IconDelete from "virtual:icons/lucide/trash-2";

    const possible_scopes = InternalScopes;

    export let data: PageData;
    let dialog: HTMLDialogElement;
    let edit: ApplicationGroup | null = null;
    const selected_scopes: boolean[] = Array(5).fill(false);
    function editRow(row: ApplicationGroup) {
        edit = row;
        for (const [index, value] of possible_scopes.entries()) {
            selected_scopes[index] = edit.scopes.includes(value)
        }

        dialog.showModal();
    }
</script>

<svelte:head>
    <title>Admin Panel</title>
    <meta name="robots" content="noindex" />
</svelte:head>

<dialog bind:this={dialog} on:close={() => (edit = null)}>
    {JSON.stringify(edit)}
    <form>
        {#each possible_scopes as scope, i}
            <input type="checkbox" bind:checked={selected_scopes[i]}/> {scope}
        {/each}
        <form />
    </form>
</dialog>

<table class="w-full">
    <thead>
        <tr>
            <th>Id</th>
            <th>Scopes</th>
            <th>Actions</th>
        </tr>
    </thead>
    <tbody>
        {#each data.groups as group}
            <tr>
                <td>{group.id}</td>
                <td>{group.scopes}</td>
                <td>
                    <div class="flex justify-center">
                        <button
                            class="button-transparent"
                            on:click={() => editRow(group)}
                        >
                            <IconEdit />
                        </button>
                        <button
                        class="button-transparent-danger"
                        on:click={() => editRow(group)}
                    >
                        <IconDelete />
                    </button>
                    </div>
                </td>
            </tr>
        {/each}
    </tbody>
</table>

<style>
    table {
        border-collapse: collapse;
    }
    th,
    td {
        --at-apply: pl-2 py-1 border-2 border-solid text-left border-light-3 dark:border-dark-1;
        border-color:#dddddd;
    }
    tr:nth-child(even) {
        --at-apply: bg-light-5 dark:bg-dark-1/40 bg-blend-lighten
    }
</style>
