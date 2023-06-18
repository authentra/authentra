<script lang="ts">
    import IconEdit from "virtual:icons/lucide/edit";
    import IconDelete from "virtual:icons/lucide/trash-2";
    import { enhance } from "$app/forms";
    import type { AdminUser } from "$lib/server/apis/user";
    import type { PageData } from "./$types";
    import { UserRoles } from "$lib/api/types";

    export let data: PageData;
    let edit_dialog: HTMLDialogElement;
    let edit: AdminUser | null;
    const selected_roles: boolean[] = Array(UserRoles.length).fill(
        false
    );

    function editRow(user: AdminUser) {
        edit = structuredClone(user);
        for (const [index, value] of UserRoles.entries()) {
            selected_roles[index] = edit.roles.includes(value);
        }
        edit_dialog.showModal();
    }
</script>

<dialog bind:this={edit_dialog}>
    {#if edit}
        <form>
            <label>
                <span>Id</span>
                <input name="id" bind:value={edit.name} readonly />
            </label>
            <label>
                <span>Name</span>
                <input name="name" bind:value={edit.name} />
            </label>
            <label>
                <span>Email</span>
                <input name="name" type="email" bind:value={edit.email} />
            </label>
            <label>
                <input name="name" type="checkbox" bind:checked={edit.active} />
                <span>Active</span>
            </label>
            <div>
                <span>Roles</span>
                <div>
                    {#each UserRoles as role,i}
                    <label>
                        <input type="checkbox" bind:checked={selected_roles[i]}/>
                        <span>{role}</span>
                    </label>
                        
                    {/each}
                </div>
            </div>
            <div>
                <button on:click={() => edit_dialog.close()}>Cancel</button>
                <button type="submit">Submit</button>
            </div>

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
        {#each data.users as user}
            <tr>
                <td>{user.id}</td>
                <td>{user.name}</td>
                <td>
                    <div class="flex justify-center">
                        <a
                            class="button-transparent"
                            href="/admin/users/{user.id}"
                        >
                            <IconEdit />
                    </a>
                        <form method="post" action="?/delete" use:enhance>
                            <input name="id" value={user.id} hidden />
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
