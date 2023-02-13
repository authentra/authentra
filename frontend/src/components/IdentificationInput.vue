<script lang="ts" setup>
import type { FlowData } from '@/api/model';

const props = defineProps<{
    data: FlowData
}>();
</script>

<template>
    <template v-if="props.data.component == 'identification'">
        <label for="uid-input">
            <template v-if="props.data.user_fields.includes('email') && props.data.user_fields.includes('name')">
                Username or Email
            </template>
            <template v-else-if="props.data.user_fields.includes('name')">Username</template>
            <template v-else-if="props.data.user_fields.includes('email')">Email</template>
            <template v-else>Uid</template>
        </label>
        <input id="uid-input" name="uid" autocomplete="username" required />
    </template>
</template>