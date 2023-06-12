<script lang="ts">
  import "../app.scss";
  //import "normalize.css";
</script>

<slot />

<style>
  :global(body) {
    --at-apply: background-body text-color;
  }
  :global(button) {
    --at-apply: border-none;
  }
  :global([role="button"], button) {
    cursor: pointer;
  }
</style>
