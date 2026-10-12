<script lang="ts">
  import BatteryGlyph from "./BatteryGlyph.svelte";
  import KindIcon from "./KindIcon.svelte";
  import { rateBatteryLevel } from "./format";
  import type { Tile } from "./types";

  /** Preview of how the taskbar tiles will look, in both layouts. */
  let { tiles }: { tiles: Tile[] } = $props();
</script>

{#if tiles.length === 0}
  <p class="empty">No tiles yet — every device is hidden or has no battery.</p>
{:else}
  <div class="layouts">
    <div class="layout">
      <span class="caption">One tile per device</span>
      <div class="taskbar">
        {#each tiles as tile (tile.id)}
          <div class="pill" class:stale={!tile.live} title="{tile.name}{tile.live ? '' : ' (old value)'}">
            <KindIcon kind={tile.icon} size={18} />
            <span class="percent {rateBatteryLevel(tile.battery)}">
              {tile.battery === null ? "–" : `${tile.battery}%`}
            </span>
          </div>
        {/each}
      </div>
    </div>

    <div class="layout">
      <span class="caption">Combined tile</span>
      <div class="taskbar">
        <div class="combined" style="--cols: {tiles.length > 2 ? 2 : 1}">
          {#each tiles.slice(0, 4) as tile (tile.id)}
            <div class="cell" class:stale={!tile.live}>
              <KindIcon kind={tile.icon} size={14} />
              <span class="percent {rateBatteryLevel(tile.battery)}">
                {tile.battery === null ? "–" : `${tile.battery}%`}
              </span>
            </div>
          {/each}
        </div>
      </div>
    </div>

    <div class="layout wide">
      <span class="caption">Full</span>
      <div class="taskbar">
        {#each tiles as tile (tile.id)}
          <div class="pill full" class:stale={!tile.live}>
            <KindIcon kind={tile.icon} size={18} />
            <span class="name">{tile.name}</span>
            <BatteryGlyph percent={tile.battery} charging={tile.charging} />
            <span class="percent {rateBatteryLevel(tile.battery)}">
              {tile.battery === null ? "–" : `${tile.battery}%`}
            </span>
          </div>
        {/each}
      </div>
    </div>
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
  }
  .layout.wide {
    flex-basis: 100%;
  }
  .caption {
    font-size: 12px;
    color: var(--text-secondary);
  }
  .taskbar {
    display: flex;
    align-items: center;
    gap: 4px;
    min-height: 48px;
    padding: 0 8px;
    border-radius: 8px;
    background: var(--taskbar);
    border: 1px solid var(--stroke);
  }
  .pill {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 40px;
    padding: 0 10px;
    border-radius: 6px;
    color: var(--text);
  }
  .pill:hover {
    background: var(--subtle-hover);
  }
  .pill.full {
    gap: 8px;
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
  .percent.low {
    color: var(--battery-low);
  }
  .combined {
    display: grid;
    grid-template-columns: repeat(var(--cols), auto);
    gap: 1px 12px;
    padding: 4px 10px;
    border-radius: 6px;
    color: var(--text);
  }
  .combined:hover {
    background: var(--subtle-hover);
  }
  .cell {
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .cell .percent {
    font-size: 11px;
  }
  .stale {
    opacity: 0.45;
  }
  .empty {
    color: var(--text-secondary);
    margin: 0;
  }
</style>
