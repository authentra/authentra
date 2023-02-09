<script lang="ts">
    import { querystring } from "svelte-spa-router";

    export let params = {};

    import { axios_instance, base_url, get_flow, post_flow } from "../../api/api";
    import Form from './Form.svelte';

    console.log(params);

    let query_value;
    querystring.subscribe(value => {query_value = value});

    $: ({ flow_slug } = params);
    $: flow_info = get_flow(flow_slug, query_value)

    let submit_disabled = false;

    async function handle_submit(e: SubmitEvent) {
        submit_disabled = true;
        const form_data = new FormData(e.target as HTMLFormElement);
        const search_params = new URLSearchParams(form_data);
        console.log(search_params);
        flow_info = post_flow(flow_slug, e.target as HTMLFormElement, query_value).finally(() => {submit_disabled = false});
        //flow_info = axios_instance.post("/flow/executor/"+flow_slug, search_params).finally(() => {submit_disabled = false});
    }
</script>

{#await flow_info}
Loading...
{:then response}
<Form flow_data={response.data} submit_disabled={submit_disabled} on:submit={handle_submit}/>
{response.data.flow.title}
{:catch error}
Fuck error
{/await}
