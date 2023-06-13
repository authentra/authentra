<script lang="ts">
    import { enhance } from "$app/forms";
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
    <form method="post" action="?/replace" use:enhance on:submit={() => dialog.close()}>
        <input name="id" hidden value={edit?.id}/>
        {#each possible_scopes as scope, i}
            <label>
                <input type="checkbox" name={scope} bind:checked={selected_scopes[i]}/>
                <span>{scope}</span>
            </label>
        {/each}
        <button type="submit">Submit</button>
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
