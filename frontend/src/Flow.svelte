<script lang="ts">
    import Navbar from "./lib/Navbar.svelte";

    let dark_mode = true;

    const onclick = function () {
        alert("Click");
    };
</script>

<Navbar />

<div class="h-screen flex items-center justify-center">
    <main class="card flex flex-col items-top box-border px-2 pb-8">
        <h2>Sign in</h2>
        <form class="flex flex-col px-1">
            <label class="form-label" for="name">Username</label>
            <input
                class="form-item"
                type="text"
                id="name"
                autocomplete="username"
            />

            <label class="form-label" for="password">Password</label>
            <input
                class="form-item"
                type="password"
                id="password"
                autocomplete="current-password"
            />

            <label class="form-label" for="one-time-code">One Time Code</label>
            <input
                class="form-item"
                type=""
                id="one-time-code"
                autocomplete="current-password"
            />

            <button
                class="mt-4 bg-blue py-2 text-light"
                type="submit"
                id=""
                on:click={onclick}
            >
                Login
            </button>
        </form>

        <div class="sources flex flex-row" />
    </main>
</div>

<style>
    .container {
        height: 100%;
        align-items: center;
    }
    .card {
        width: 25rem;
        --at-apply: bg-background-200 rounded-3;
    }
    input {
        --at-apply: border-solid border-0 bg-background-100 p-2;
    }
    .form-item {
        --at-apply: mb-3;
    }
    .form-label {
        --at-apply: text-4;
        margin-left: 0.2rem;
        margin-bottom: 0.1rem;
    }
</style>
