<script lang="ts">
    import { enhance } from "$app/forms";
    import { page } from "$app/stores";
    import { UserRoles } from "$lib/api/types";
    import type { AdminUser } from "$lib/server/apis/user";
    import type { PageData } from "./$types";

    export let data: PageData;
    const selected_roles: boolean[] = Array(UserRoles.length).fill(false);
    let delete_dialog: HTMLDialogElement;
    let params = $page.params;
    let initial: AdminUser;
    let user: AdminUser;
    function assignUser(u: AdminUser) {
        if (u != initial) {
            initial = structuredClone(data.user);
            user = structuredClone(data.user);
            for (const [index, value] of UserRoles.entries()) {
                selected_roles[index] = user.roles.includes(value);
            }
        }
    }
    assignUser(data.user);
    $: assignUser(data.user)
</script>

<dialog bind:this={delete_dialog}>
    <form method="post" action="?/delete" use:enhance>
        <h3>Do you really want to delete this user?</h3>
        <button on:click={() => delete_dialog.close()}>Cancel</button>
        <button type="submit">Delete</button>
    </form>
</dialog>

<form class="flex flex-col" method="post" action="?/edit" use:enhance>
    <label>
        <span>Id</span>
        <input name="id" bind:value={user.id} readonly />
    </label>
    <label>
        <span>Name</span>
        <input name="name" bind:value={user.name} />
    </label>
    <label>
        <span>Email</span>
        <input name="email" type="email" bind:value={user.email} />
    </label>
    <label>
        <input name="active" type="checkbox" bind:checked={user.active} />
        <span>Active</span>
    </label>
    <label>
        <input name="customer" type="checkbox" bind:checked={user.customer} />
        <span>Customer</span>
    </label>
    <label>
        <input name="require_password_reset" type="checkbox" bind:checked={user.require_password_reset} />
        <span>Require Password reset</span>
    </label>
    <div>
        <span>Roles</span>
        <div>
            {#each UserRoles as role, i}
                <label>
                    <input type="checkbox" name="role:{role}" bind:checked={selected_roles[i]} />
                    <span>{role}</span>
                </label>
            {/each}
        </div>
    </div>
    <div>
        <button on:click={() => delete_dialog.showModal()}>Delete</button>
        <button type="submit">Save</button>
    </div>
</form>
