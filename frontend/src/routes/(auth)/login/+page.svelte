<script lang="ts">
    import IconLight from 'virtual:icons/lucide/sun'
    import type { ActionData, ActionsFailure } from './$types';
    import { enhance } from '$app/forms';

    export let form: ActionData;
</script>

<div class="flex h100% items-center justify-center">
    <main class="auth-card">
        <div class="header">
            <span>Login</span>
            <IconLight />
        </div>

        {#if form && !form.success}
        <div class="bg-red-3 text-black mb-3 mt-2">{form.message}</div>
        {/if}
        
       
        <form method="post" class="flex flex-col gap-6" use:enhance>
            <label class="field">
                <span>Username or Email</span>
                <input required name="user" autocomplete="username"/>
            </label>
            <label class="field">
                <span>Password</span>
                <input type="password" required name="password" autocomplete="current-password"/>
            </label>
            <button type="submit" class="mt-1">Login</button>
        </form>
    </main>
</div>