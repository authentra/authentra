<slot />

<style>
    :global(.auth-card) {
        --at-apply: flex flex-col shadow-2xl w25rem box-border p-4 rounded-xl;
    }

    :global(.auth-card .header) {
        --at-apply: flex justify-between items-center mb-4;
    }

    :global(.field) {
        --at-apply: flex flex-col;
    }
</style>
