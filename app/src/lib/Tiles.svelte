<script lang="ts">
  import BatteryGlyph from "./BatteryGlyph.svelte";
  import KindIcon from "./KindIcon.svelte";
  import { rateBatteryLevel } from "./format";
  import type { Tile, TileLayout } from "./types";

  /** The tiles themselves in one layout; used on the taskbar and in the preview. */
  let {
    tiles,
    layout,
    onclick,
  }: { tiles: Tile[]; layout: TileLayout; onclick?: () => void } = $props();

  /** Formats the charge as "80%", or a dash when it's unknown. */
  function formatPercent(tile: Tile): string {
    return tile.battery === null ? "–" : `${tile.battery}%`;
  }

  /** Builds the hover text: name, charge and whether the value is old. */
  function describeTile(tile: Tile): string {
    const stale = tile.live ? "" : " (old value)";
    return `${tile.name} ${formatPercent(tile)}${stale}`;
  }
</script>

<div class="tiles">
  {#if layout === "combined"}
    <button class="tile combined" style="--cols: {tiles.length > 2 ? 2 : 1}" {onclick}
      title={tiles.map(describeTile).join("\n")}>
      {#each tiles.slice(0, 4) as tile (tile.id)}
        <span class="cell" class:stale={!tile.live}>
          <KindIcon kind={tile.icon} size={14} />
          <span class="percent small {rateBatteryLevel(tile.battery)}">{formatPercent(tile)}</span>
        </span>
      {/each}
    </button>
  {:else}
    {#each tiles as tile (tile.id)}
      <button class="tile pill" class:stale={!tile.live} {onclick} title={describeTile(tile)}>
        <KindIcon kind={tile.icon} size={18} />
        {#if layout === "full"}
          <span class="name">{tile.name}</span>
          <BatteryGlyph percent={tile.battery} charging={tile.charging} />
        {/if}
        <span class="percent {rateBatteryLevel(tile.battery)}">{formatPercent(tile)}</span>
      </button>
    {/each}
  {/if}
</div>

<style>
  .tiles {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }
  .tile {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 40px;
    min-height: 0;
    padding: 0 10px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text);
    cursor: default;
  }
  .tile:hover {
    background: var(--subtle-hover);
  }
  .name {
    font-size: 12px;
    max-width: 160px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .percent {
    font-size: 13px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .percent.small {
    font-size: 11px;
  }
  .percent.low {
    color: var(--battery-low);
  }
  .combined {
    display: grid;
    grid-template-columns: repeat(var(--cols), auto);
    gap: 0 12px;
    padding: 2px 10px;
  }
  .cell {
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .stale {
    opacity: 0.45;
  }
</style>
