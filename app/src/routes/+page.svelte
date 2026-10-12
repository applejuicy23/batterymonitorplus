<script lang="ts">
  import { onMount } from "svelte";

  import DeviceRowView from "$lib/DeviceRowView.svelte";
  import TilePreview from "$lib/TilePreview.svelte";
  import {
    fetchSnapshot,
    listenForSnapshots,
    refreshDevices,
    savePollSeconds,
    updateDevice,
  } from "$lib/api";
  import type { DeviceChange, Snapshot } from "$lib/types";

  const POLL_CHOICES = [15, 30, 60, 120, 300];

  let snapshot = $state<Snapshot | null>(null);
  let refreshing = $state(false);
  let actionError = $state<string | null>(null);

  /** Rows that are their own tile entry (not merged into another one). */
  let mainRows = $derived(snapshot?.rows.filter((r) => !r.merged_into) ?? []);
  /** Shown rows in tile order, used for the up/down buttons. */
  let orderedShown = $derived(
    snapshot ? snapshot.tiles.map((t) => t.id).filter((id) => mainRows.some((r) => r.id === id)) : [],
  );

  onMount(() => {
    fetchSnapshot().then((s) => (snapshot = s));
    const unlisten = listenForSnapshots((s) => (snapshot = s));
    return () => {
      unlisten.then((stop) => stop());
    };
  });

  /** Re-detects devices right away instead of waiting for the next poll. */
  async function refreshNow() {
    refreshing = true;
    try {
      snapshot = await refreshDevices();
    } finally {
      refreshing = false;
    }
  }

  /** Sends one settings change and shows the result (or the error). */
  async function changeDevice(id: string, change: DeviceChange) {
    try {
      snapshot = await updateDevice(id, change);
      actionError = null;
    } catch (e) {
      actionError = String(e);
    }
  }

  /** Moves a tile one place up or down by renumbering every shown tile. */
  async function moveTile(id: string, step: -1 | 1) {
    const order = [...orderedShown];
    const from = order.indexOf(id);
    const to = from + step;
    if (from < 0 || to < 0 || to >= order.length) return;
    [order[from], order[to]] = [order[to], order[from]];
    for (const [index, tileId] of order.entries()) {
      await changeDevice(tileId, { type: "order", value: index + 1 });
    }
  }

  /** Saves the picked poll interval. */
  async function changePollSeconds(event: Event) {
    const seconds = Number((event.target as HTMLSelectElement).value);
    snapshot = await savePollSeconds(seconds);
  }
</script>

<main>
  <header>
    <div>
      <h1>BatteryMonitor+</h1>
      <p class="subtitle">Battery tiles for your peripherals</p>
    </div>
    <div class="header-actions">
      {#if snapshot}
        <label class="poll">
          Update every
          <select value={snapshot.poll_seconds} onchange={changePollSeconds}>
            {#each POLL_CHOICES as seconds}
              <option value={seconds}>{seconds < 60 ? `${seconds} s` : `${seconds / 60} min`}</option>
            {/each}
          </select>
        </label>
      {/if}
      <button class="accent" onclick={refreshNow} disabled={refreshing}>
        {refreshing ? "Refreshing…" : "Refresh now"}
      </button>
    </div>
  </header>

  {#if !snapshot}
    <p class="loading">Looking for devices…</p>
  {:else}
    <section>
      <h2>Tiles preview</h2>
      <div class="card">
        <TilePreview tiles={snapshot.tiles} />
      </div>
    </section>

    <section>
      <h2>Devices</h2>
      <div class="rows">
        {#each snapshot.rows as row (row.id)}
          <DeviceRowView
            {row}
            others={mainRows.filter((r) => r.id !== row.id)}
            canMoveUp={row.shown && orderedShown.indexOf(row.id) > 0}
            canMoveDown={row.shown && orderedShown.indexOf(row.id) < orderedShown.length - 1}
            onChange={changeDevice}
            onMove={moveTile}
          />
        {:else}
          <p class="loading">No devices found yet.</p>
        {/each}
      </div>
    </section>

    {#if snapshot.error || actionError}
      <p class="error">{actionError ?? snapshot.error}</p>
    {/if}

    <footer>Settings file: <code>{snapshot.config_path}</code></footer>
  {/if}
</main>

<style>
  main {
    max-width: 980px;
    margin: 0 auto;
    padding: 24px 28px 32px;
    display: flex;
    flex-direction: column;
    gap: 24px;
  }
  header {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
  }
  h1 {
    margin: 0;
    font-family: "Segoe UI Variable Display", "Segoe UI", sans-serif;
    font-size: 28px;
    font-weight: 600;
  }
  .subtitle {
    margin: 2px 0 0;
    color: var(--text-secondary);
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .poll {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-secondary);
    font-size: 13px;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }
  .card {
    padding: 16px;
    background: var(--card);
    border: 1px solid var(--stroke);
    border-radius: 8px;
  }
  .rows {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .loading {
    color: var(--text-secondary);
  }
  .error {
    margin: 0;
    padding: 10px 14px;
    border-radius: 8px;
    background: rgba(196, 43, 28, 0.12);
    color: var(--battery-low);
  }
  footer {
    font-size: 12px;
    color: var(--text-tertiary);
  }
  code {
    font-family: "Cascadia Mono", Consolas, monospace;
  }
</style>
