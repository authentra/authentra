<script lang="ts">
    import { querystring, replace } from "svelte-spa-router";
    import type { FlowData } from "../../api/model";

    export let flow_data: FlowData;
    export let submit_disabled: boolean;

    function is_form_fn(data: FlowData) {
        switch (data.component) {
            case 'identification': return true
            case 'password': return true
            default: return false
        }
    };

    let is_form = is_form_fn(flow_data);
    console.log("Form:" + is_form);

    if (flow_data.component == 'redirect') {
        replace(flow_data.to)
    }
</script>

{#if is_form}

<form on:submit|preventDefault enctype="application/x-www-form-urlencoded">
    {#if flow_data.component == "identification"}
        <label for="uid">
            {#if flow_data.user_fields.includes("email") && flow_data.user_fields.includes("name")}
                Email or Username
            {:else if flow_data.user_fields.includes("name")}
                Username
            {:else if flow_data.user_fields.includes("email")}
                Email
            {:else}
                Uid
            {/if}
        </label>
        <input id="uid" name="uid" />
    {:else if flow_data.component == "password"}
        <label for="password">Password</label>
        <input id="password" name="password" type="password" />
    {/if}

    <button type="submit" disabled={submit_disabled}>Submit</button>
</form>
{/if}

{#if flow_data.component == 'error'}
    Error: {flow_data.message}
    <br>
{/if}
