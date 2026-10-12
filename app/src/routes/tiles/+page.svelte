<script lang="ts">
  import { onMount } from "svelte";

  import Tiles from "$lib/Tiles.svelte";
  import { fetchSnapshot, listenForSnapshots, openSettings, reportTilesWidth } from "$lib/api";
  import type { Snapshot } from "$lib/types";

  let snapshot = $state<Snapshot | null>(null);
  let content: HTMLDivElement | undefined = $state();

  onMount(() => {
    fetchSnapshot().then((s) => (snapshot = s));
    const unlisten = listenForSnapshots((s) => (snapshot = s));
    const observer = new ResizeObserver(() => reportContentWidth());
    if (content) observer.observe(content);
    return () => {
      observer.disconnect();
      unlisten.then((stop) => stop());
    };
  });

  /** Sends the tiles' width to Rust (0 hides the window when there's nothing to show). */
  function reportContentWidth() {
    const hasTiles = (snapshot?.tiles.length ?? 0) > 0;
    const width = hasTiles && content ? Math.ceil(content.getBoundingClientRect().width) : 0;
    reportTilesWidth(width);
  }
</script>

<div class="dock" class:light={snapshot?.taskbar_light} class:dark={snapshot && !snapshot.taskbar_light}>
  <div class="content" bind:this={content}>
    {#if snapshot && snapshot.tiles.length > 0}
      <Tiles tiles={snapshot.tiles} layout={snapshot.tile_layout} onclick={openSettings} />
    {/if}
  </div>
</div>

<style>
  :global(html),
  :global(body) {
    background: transparent !important;
    overflow: hidden;
    user-select: none;
  }
  .dock {
    display: flex;
    align-items: center;
    height: 100vh;
  }
  .content {
    display: inline-flex;
    flex: none;
    width: max-content;
  }
  .light {
    --text: #1b1b1b;
    --subtle-hover: rgba(0, 0, 0, 0.06);
    --battery-ok: #0f7b0f;
    --battery-mid: #c48400;
    --battery-low: #c42b1c;
    --card: #f3f3f3;
  }
  .dark {
    --text: #ffffff;
    --subtle-hover: rgba(255, 255, 255, 0.08);
    --battery-ok: #6ccb5f;
    --battery-mid: #fce100;
    --battery-low: #ff99a4;
    --card: #202020;
  }
</style>
