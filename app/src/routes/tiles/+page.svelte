<script lang="ts">
  import { onMount } from "svelte";

  import Tiles from "$lib/Tiles.svelte";
  import {
    fetchSnapshot,
    listenForSnapshots,
    moveTilesBy,
    openSettings,
    reportTilesWidth,
    saveTilesPosition,
  } from "$lib/api";
  import type { Snapshot } from "$lib/types";

  /** How far the pointer must travel before a press becomes a drag. */
  const DRAG_THRESHOLD = 4;

  let snapshot = $state<Snapshot | null>(null);
  let content: HTMLDivElement | undefined = $state();
  let dragging = $state(false);

  let pressX: number | null = null;
  let lastX = 0;
  let pendingDx = 0;
  let frameRequested = false;

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

  /** Remembers where the press started; it becomes a click or a drag later. */
  function startPress(event: PointerEvent) {
    if (event.button !== 0) return;
    pressX = event.screenX;
    lastX = event.screenX;
    dragging = false;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  /** Moves the window along the taskbar once the pointer has travelled far enough. */
  function followPointer(event: PointerEvent) {
    if (pressX === null) return;
    if (!dragging && Math.abs(event.screenX - pressX) < DRAG_THRESHOLD) return;
    dragging = true;
    pendingDx += event.screenX - lastX;
    lastX = event.screenX;
    if (!frameRequested) {
      frameRequested = true;
      requestAnimationFrame(sendPendingMove);
    }
  }

  /** Sends the movement collected during one frame, so Rust isn't flooded. */
  async function sendPendingMove() {
    frameRequested = false;
    const dx = pendingDx;
    pendingDx = 0;
    if (dx !== 0) await moveTilesBy(dx);
  }

  /** Ends the press: saves the spot after a drag, opens settings after a click. */
  async function endPress() {
    if (pressX === null) return;
    pressX = null;
    if (dragging) {
      dragging = false;
      await sendPendingMove();
      await saveTilesPosition();
    } else {
      openSettings();
    }
  }
</script>

<div class="dock" class:light={snapshot?.taskbar_light} class:dark={snapshot && !snapshot.taskbar_light}>
  <div
    class="content"
    class:dragging
    bind:this={content}
    role="presentation"
    onpointerdown={startPress}
    onpointermove={followPointer}
    onpointerup={endPress}
    onpointercancel={endPress}
  >
    {#if snapshot && snapshot.tiles.length > 0}
      <Tiles tiles={snapshot.tiles} layout={snapshot.tile_layout} />
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
    border-radius: 6px;
  }
  .content.dragging {
    outline: 1px dashed var(--text);
    outline-offset: -1px;
    cursor: ew-resize;
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
