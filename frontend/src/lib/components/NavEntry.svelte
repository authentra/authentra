<script lang="ts">
    import { page } from "$app/stores";

    type RouteMatcher = (routeId: string | null, url: URL) => boolean;

    export let routeMatch: RouteMatcher | "page" = "page";
    export let target: string;

    $: matches =
        routeMatch === "page"
            ? $page.url.pathname === target
            : routeMatch($page.route.id, $page.url);
</script>

<a href={target} class="block text decoration-none link-button" class:button-active={matches}>
    <span>
        <slot />
    </span>
</a>
