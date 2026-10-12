<script lang="ts">
  import "../app.css";

  /** Wraps every page with the global Windows 11 styles. */
  let { children } = $props();
</script>

{@render children()}
