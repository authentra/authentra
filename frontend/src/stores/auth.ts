import { ref, computed, type Ref } from 'vue';
import { defineStore } from 'pinia';
export const useAuthStore = defineStore('auth', () => {
    const user_id: Ref<string | null> = ref(null);
    const is_authenticated = computed(() => user_id.value != null);
    async function check_authentication() {
        
    }
});