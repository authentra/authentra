<script lang="ts" setup>
import { execute_flow, execute_flow_post } from '@/api/api';
import type { FlowData } from '@/api/model';
import IdentificationInput from '@/components/IdentificationInput.vue';
import PasswordInput from '@/components/PasswordInput.vue';
import type { AxiosResponse } from 'axios';
import { watch, ref, type Ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

const route = useRoute();
const router = useRouter();
const flow_slug: Ref<string> = ref(route.params.flow_slug as string);
const data: Ref<FlowData | null> = ref(null);
const data_error: Ref<FlowData | null> = ref(null);
const error: Ref<any | null> = ref(null);

function fetch_flow(slug: string) {
    handle_promise(execute_flow(slug))
}

watch(() => route.params.flow_slug, (newValue, oldValue) => {
    flow_slug.value = newValue as string
}, { deep: true });
watch(flow_slug, (newValue, oldValue) => {
    fetch_flow(newValue)
})
fetch_flow(flow_slug.value)

function submit(e: Event) {
    const target = e.target as HTMLFormElement
    handle_promise(execute_flow_post(flow_slug.value, target))
}

function handle_promise(promise: Promise<AxiosResponse<FlowData>>) {
    promise.then((res) => {
        data.value = res.data
        error.value = null
        if (data.value.component == 'redirect') {
            router.push(data.value.to)
        }
    }).catch((err) => {
        error.value = err
        data.value = null
    });
}

</script>
<template>
    <form @submit.prevent="submit">
        <IdentificationInput v-if="data != null && data.component == 'identification'" v-bind:data="data" />
        <PasswordInput
        v-if="data != null && (data.component == 'password' || (data.component == 'identification' && data.password != null))"/>
        <button type="submit">Login</button>
    </form>

</template>
