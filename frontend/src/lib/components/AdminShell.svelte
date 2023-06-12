<script lang="ts">
    import ThemeToggle from "./ThemeToggle.svelte";
    import IconUser from "virtual:icons/lucide/user";
    import IconMenu from "virtual:icons/lucide/menu";
    import NavEntry from "./NavEntry.svelte";

    let open = false;
    function handleCaret(e: any) {}
</script>

<div class="flex flex-row h-full">
    <div
        class="h-full background-body w-64 border-r border-r-solid hidden fixed lg:flex lg:static z99999"
        class:flex={open}
    >
        <ul class="list-none">
            <li>
                <NavEntry target="/admin">Home</NavEntry>
            </li>
            <li>
                <NavEntry target="/admin/applications">Applications</NavEntry>
            </li>
            <li>
                <NavEntry target="/admin/application-groups">Application Groups</NavEntry>
            </li>
            
        </ul>
    </div>
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div
        class="hidden fixed h-screen w-screen top-0 left-0 blocker"
        class:!block={open}
        class:blocker-open={open}
        on:click={() => (open = false)}
    />
    <div class="block w-screen h-screen wrapper">
        <div class="flex justify-between items-center p2 border-b border-b-solid">
            <button
                class="button-transparent hidden <lg:inline"
                on:click={() => (open = true)}
            >
                <IconMenu />
            </button>
            <div class="lg:inline" />
            <div class="gap-2">
                <ThemeToggle />
                <button class="button-transparent">
                    <IconUser />
                </button>
            </div>
        </div>
        <div class="block pl-2 pt-2"><slot /></div>
        
    </div>
</div>

<style>
    .blocker {
        background-color: rgba(0, 0, 0, 0.5);
    }

    .blocker-open + .wrapper {
        z-index: -1;
    }
</style>
