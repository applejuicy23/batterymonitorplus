<script lang="ts">
  import { rateBatteryLevel } from "./format";

  /** Small horizontal battery filled to the charge, with a bolt when charging. */
  let { percent, charging = false }: { percent: number | null; charging?: boolean | null } =
    $props();

  let level = $derived(rateBatteryLevel(percent));
  let fill = $derived(percent === null ? 0 : Math.max(2, Math.min(100, percent)));
</script>

<span class="battery {level}" aria-hidden="true">
  <span class="body">
    <span class="fill" style="width: {fill}%"></span>
    {#if charging}
      <svg class="bolt" viewBox="0 0 10 14"><path d="M6 0 0 8h4l-1 6 7-9H6z" /></svg>
    {/if}
  </span>
  <span class="cap"></span>
</span>

<style>
  .battery {
    display: inline-flex;
    align-items: center;
    gap: 1px;
  }
  .body {
    position: relative;
    width: 22px;
    height: 11px;
    border: 1.4px solid currentColor;
    border-radius: 3px;
    padding: 1px;
    box-sizing: border-box;
    opacity: 0.9;
  }
  .fill {
    display: block;
    height: 100%;
    border-radius: 1.5px;
    background: var(--battery-ok);
  }
  .mid .fill {
    background: var(--battery-mid);
  }
  .low .fill {
    background: var(--battery-low);
  }
  .cap {
    width: 2px;
    height: 5px;
    border-radius: 0 1px 1px 0;
    background: currentColor;
    opacity: 0.9;
  }
  .bolt {
    position: absolute;
    inset: 0;
    margin: auto;
    width: 7px;
    height: 9px;
    fill: var(--text);
    stroke: var(--card);
    stroke-width: 1;
  }
</style>
