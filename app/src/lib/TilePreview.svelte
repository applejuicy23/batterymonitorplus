<script lang="ts">
  import Tiles from "./Tiles.svelte";
  import type { Tile, TileLayout } from "./types";

  const LAYOUTS: { id: TileLayout; caption: string }[] = [
    { id: "pills", caption: "One tile per device" },
    { id: "combined", caption: "Combined tile" },
    { id: "full", caption: "Full" },
  ];

  /** Shows every layout on a fake taskbar; clicking one makes it the taskbar look. */
  let {
    tiles,
    layout,
    onPick,
  }: { tiles: Tile[]; layout: TileLayout; onPick: (layout: TileLayout) => void } = $props();
</script>

{#if tiles.length === 0}
  <p class="empty">No tiles yet — every device is hidden or has no battery.</p>
{:else}
  <div class="layouts">
    {#each LAYOUTS as option (option.id)}
      <label class="layout" class:picked={layout === option.id}>
        <span class="caption">
          <input type="radio" name="layout" checked={layout === option.id} onchange={() => onPick(option.id)} />
          {option.caption}
        </span>
        <div class="taskbar">
          <Tiles {tiles} layout={option.id} onclick={() => onPick(option.id)} />
        </div>
      </label>
    {/each}
  </div>
{/if}

<style>
  .layouts {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
  }
  .layout {
    display: flex;
    flex-direction: column;
    gap: 6px;
    cursor: pointer;
  }
  .caption {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .caption input {
    accent-color: var(--accent);
    margin: 0;
  }
  .picked .caption {
    color: var(--text);
    font-weight: 600;
  }
  .taskbar {
    display: flex;
    align-items: center;
    min-height: 48px;
    padding: 0 6px;
    border-radius: 8px;
    background: var(--taskbar);
    border: 1px solid var(--stroke);
  }
  .picked .taskbar {
    border-color: var(--accent);
  }
  .empty {
    color: var(--text-secondary);
    margin: 0;
  }
</style>
