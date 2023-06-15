<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import IconDelete from "virtual:icons/lucide/trash-2";

    const dispatch = createEventDispatcher();
    function change() {
        dispatch("change", value);
    }

    export let value: string[];
</script>

<table>
    <thead>
        <tr>
            <th>Url</th>
            <th />
        </tr>
    </thead>
    <tbody>
        {#each value as uri}
            <tr>
                <td>{uri}</td>
                <td>
                    <div class="flex justify-center">
                        <button
                            class="button-transparent-danger"
                            on:click={() => {
                                value = value.filter((v) => v != uri);
                                change()
                            }}
                        >
                            <IconDelete />
                        </button>
                    </div>
                </td>
            </tr>
        {/each}
    </tbody>
</table>
