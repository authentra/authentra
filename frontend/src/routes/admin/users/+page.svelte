<script lang="ts">
    import IconEdit from "virtual:icons/lucide/edit";
    import IconDelete from "virtual:icons/lucide/trash-2";
    import { enhance } from "$app/forms";
    import type { AdminUser } from "$lib/server/apis/user";
    import type { PageData } from "./$types";
    import { UserRoles } from "$lib/api/types";

    export let data: PageData;
    let create_dialog: HTMLDialogElement;
    let create: { name: string; password: string; customer: boolean } = {
        name: "",
        password: "",
        customer: false,
    };
    const selected_roles: boolean[] = Array(UserRoles.length).fill(false);

    function createRow() {
        create = { name: "", password: "", customer: false };
        selected_roles.fill(false);
        create_dialog.showModal();
    }
</script>

<dialog bind:this={create_dialog}>
    <form method="post" action="?/create" use:enhance on:submit={() => create_dialog.close()}>
        <label>
            <span>Name</span>
            <input name="name" bind:value={create.name} required />
        </label>
        <label>
            <span>Password</span>
            <input
                name="password"
                type="password"
                bind:value={create.password}
                required
            />
        </label>
        <label>
            <input
                name="customer"
                type="checkbox"
                bind:checked={create.customer}
            />
            <span>Customer</span>
        </label>
        <div>
            <span>Roles</span>
            <div>
                {#each UserRoles as role, i}
                    <label>
                        <input
                            type="checkbox"
                            name="role:{role}"
                            bind:checked={selected_roles[i]}
                        />
                        <span>{role}</span>
                    </label>
                {/each}
            </div>
        </div>
        <div>
            <button on:click={() => create_dialog.close()}>Cancel</button>
            <button type="submit">Submit</button>
        </div>
    </form>
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
                    </div>
                </td>
            </tr>
        {/each}
    </tbody>
</table>
<button on:click={() => createRow()}>Create</button>
