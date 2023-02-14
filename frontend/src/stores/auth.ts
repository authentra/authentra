import type { PartialUser } from './../api/model.d';
import { ref, computed, type Ref } from 'vue';
import { defineStore } from 'pinia';
import { get_user_info } from '@/api/api';
export const useAuthStore = defineStore('auth', () => {
    const user: Ref<PartialUser | null> = ref(null);
    const is_authenticated = computed(() => user.value != null);
    async function check_authentication() {
        get_user_info().then((res) => {
            user.value = res.data.user
        }).catch((err) => {
            user.value = null
        })
    }
});