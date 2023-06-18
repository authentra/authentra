<script lang="ts">
    import { enhance } from "$app/forms";
    import IconEdit from "virtual:icons/lucide/edit";
    import IconDelete from "virtual:icons/lucide/trash-2";
    import type { PageData } from "./$types";
    import { ApplicationKinds, type Application } from "$lib/api/developer";
    import UrlList from "$lib/components/UrlList.svelte";
    import { redirect } from "@sveltejs/kit";

    export let data: PageData;
    let edit_dialog: HTMLDialogElement;
    let create_dialog: HTMLDialogElement;
    let create: Application | null = null;
    let create_url_field: string = "";
    let edit_url_field: string = "";

    let edit: Application | null = null;

    function editRow(row: Application) {
        edit = structuredClone(row);
        edit_url_field = "";
        console.log(edit);
        edit_dialog.showModal();
    }

    function createRow() {
        create = {
            id: "",
            name: "",
            system_application: false,
            application_group: data.groups.length == 1 ? data.groups[0] : "",
            kind: "",
            client_id: "",
            redirect_uri: [],
        };
        create_url_field = "";
        create_dialog.showModal();
    }
    $: data.applications.sort(function (a, b) {
        if (a.id > b.id) {
            return -1;
        }
        if (b.id > a.id) {
            return 1;
        }
        return 0;
    });
</script>

<svelte:head>
    <title>Developer Panel</title>
    <meta name="robots" content="noindex" />
</svelte:head>

<dialog bind:this={edit_dialog}>
    {#if edit}
        <form
            method="post"
            action="?/edit"
            use:enhance
            on:submit={() => edit_dialog.close()}
        >
            <input name="id" bind:value={edit.id} hidden readonly />
            <label>
                <span>Name</span>
                <input name="name" bind:value={edit.name} />
            </label>
            {#if data.groups.length > 1 && data.is_admin}
                <label>
                    <span>Application Group</span>
                    <select bind:value={edit.application_group} disabled>
                        <option value={edit.application_group}
                            >{edit.application_group}</option
                        >
                    </select>
                </label>
            {:else}
                <input
                    name="application_group"
                    bind:value={edit.application_group}
                    hidden
                    readonly
                />
            {/if}
            <label>
                <span>Client Id</span>
                <input bind:value={edit.client_id} readonly />
            </label>
            <label>
                <span>Kind</span>
                <select bind:value={edit.kind} disabled>
                    <option value={edit.kind}>{edit.kind}</option>
                </select>
            </label>
            {#if data.is_admin}
                <label>
                    <input
                        type="checkbox"
                        bind:checked={edit.system_application}
                        disabled
                    />
                    <span>System Application</span>
                </label>
            {/if}
            {#each edit.redirect_uri as uri, i}
                <input name="uri={i}" value={uri} hidden readonly />
            {/each}
            <form
                on:submit|preventDefault={() => {
                    edit?.redirect_uri.push(edit_url_field);
                    edit.redirect_uri = edit.redirect_uri;
                    edit_url_field = "";
                }}
            >
                <input name="uri" type="url" bind:value={edit_url_field} />
                <button type="submit">Add</button>
            </form>
            <UrlList
                bind:value={edit.redirect_uri}
                on:change={(e) => (edit.redirect_uri = e.detail)}
            />
            <button type="button" on:click={() => edit_dialog.close()}>Cancel</button>
            <button type="submit">Submit</button>
        </form>
    {/if}
</dialog>

<dialog bind:this={create_dialog}>
    {#if create}
        <form
            method="post"
            action="?/create"
            use:enhance
            on:submit={() => create_dialog.close()}
        >
            <label>
                <span>Name</span>
                <input name="name" bind:value={create.name} required />
            </label>
            {#if data.groups.length > 1 && data.is_admin}
                <label>
                    <span>Application Group</span>
                    <select
                        name="application_group"
                        bind:value={create.application_group}
                        required
                    >
                        {#each data.groups as group}
                            <option value={group}>{group}</option>
                        {/each}
                    </select>
                </label>
            {:else}
                <input
                    name="application_group"
                    bind:value={create.application_group}
                    hidden
                    readonly
                />
            {/if}

            <label>
                <span>Kind</span>
                <select name="kind" bind:value={create.kind} required>
                    {#each ApplicationKinds as kind}
                        <option value={kind}>{kind}</option>
                    {/each}
                </select>
            </label>
            {#if data.is_admin}
                <label>
                    <input
                        type="checkbox"
                        bind:checked={create.system_application}
                    />
                    <span>System Application</span>
                </label>
            {/if}
            {#each create.redirect_uri as uri, i}
                <input name="uri={i}" bind:value={uri} hidden />
            {/each}
            <form
                on:submit|preventDefault={() => {
                    create?.redirect_uri.push(create_url_field);
                    create.redirect_uri = create.redirect_uri;
                    create_url_field = "";
                }}
            >
                <input name="uri" type="url" bind:value={create_url_field} />
                <button type="submit">Add</button>
            </form>
            <UrlList
                bind:value={create.redirect_uri}
                on:change={(e) => (create.redirect_uri = e.detail)}
            />
            <button type="button" on:click={() => create_dialog.close()}>Cancel</button>
            <button type="submit">Submit</button>
        </form>
    {/if}
</dialog>

<table class="w-full">
    <thead>
        <tr>
            <th>Id</th>
            <th>Name</th>
            <th />
        </tr>
    </thead>
    <tbody>
        {#each data.applications as application}
            <tr>
                <td>{application.id}</td>
                <td>{application.name}</td>
                <td>
                    <div class="flex justify-center">
                        <button
                            class="button-transparent"
                            on:click={() => editRow(application)}
                        >
                            <IconEdit />
                        </button>
                        <form method="post" action="?/delete" use:enhance>
                            <input name="id" value={application.id} hidden />
                            <button class="button-transparent-danger">
                                <IconDelete />
                            </button>
                        </form>
                    </div>
                </td>
            </tr>
        {/each}
    </tbody>
</table>
<button on:click={() => createRow()}>Create</button>
