<script lang="ts">
  import BatteryGlyph from "./BatteryGlyph.svelte";
  import KindIcon from "./KindIcon.svelte";
  import { describeSource, describeState } from "./format";
  import { ALL_KINDS, type DeviceChange, type DeviceKind, type DeviceRow } from "./types";

  /** One device in the settings list, with its show/name/kind/merge controls. */
  let {
    row,
    others,
    canMoveUp,
    canMoveDown,
    onChange,
    onMove,
  }: {
    row: DeviceRow;
    others: DeviceRow[];
    canMoveUp: boolean;
    canMoveDown: boolean;
    onChange: (id: string, change: DeviceChange) => void;
    onMove: (id: string, step: -1 | 1) => void;
  } = $props();

  let device = $derived(row.device);
  let explicitShow = $derived(row.settings.show);

  /** Saves the typed name when the field loses focus or Enter is pressed. */
  function saveName(event: Event) {
    const value = (event.target as HTMLInputElement).value.trim();
    const detected = row.settings.detected_name ?? row.id;
    if (value === row.name) return;
    onChange(row.id, { type: "name", value: value === "" || value === detected ? null : value });
  }

  /** Switches the device in or out of the tiles. */
  function toggleShown(event: Event) {
    onChange(row.id, { type: "show", value: (event.target as HTMLInputElement).checked });
  }

  /** Saves a kind override, or clears it when "auto" is picked. */
  function saveKind(event: Event) {
    const value = (event.target as HTMLSelectElement).value;
    onChange(row.id, { type: "kind", value: value === "" ? null : (value as DeviceKind) });
  }

  /** Merges this device into the picked one. */
  function mergeInto(event: Event) {
    const select = event.target as HTMLSelectElement;
    if (select.value) onChange(row.id, { type: "link", value: select.value });
    select.value = "";
  }
</script>

<div class="row" class:merged={row.merged_into} class:missing={!row.detected}>
  <div class="icon"><KindIcon kind={row.icon} size={22} /></div>

  <div class="main">
    {#if row.merged_into}
      <span class="title">{row.name}</span>
    {:else}
      <input
        class="title-input"
        value={row.name}
        onblur={saveName}
        onkeydown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
        spellcheck="false"
      />
    {/if}
    <span class="meta">
      {#if device}
        {describeSource(device.source)} · {describeState(device)}
      {:else}
        not found right now
      {/if}
      {#if row.merged_into}
        · part of <b>{row.merged_into}</b>
      {/if}
    </span>
    <span class="id">{row.id}</span>
  </div>

  <div class="battery">
    {#if device?.battery != null}
      <BatteryGlyph percent={device.battery} charging={device.charging} />
      <span class="percent">{device.battery}%</span>
    {:else}
      <span class="percent dim">–</span>
    {/if}
  </div>

  <div class="controls">
    {#if row.merged_into}
      <button onclick={() => onChange(row.id, { type: "unlink" })}>Unmerge</button>
    {:else}
      <select value={row.settings.kind ?? ""} onchange={saveKind} title="Device kind">
        <option value="">auto ({row.settings.detected_kind ?? row.kind})</option>
        {#each ALL_KINDS as kind}
          <option value={kind}>{kind}</option>
        {/each}
      </select>

      {#if others.length > 0}
        <select onchange={mergeInto} title="Same physical device as…">
          <option value="">Merge into…</option>
          {#each others as other (other.id)}
            <option value={other.id}>{other.name}</option>
          {/each}
        </select>
      {/if}

      <div class="order">
        <button class="icon-btn" disabled={!canMoveUp} onclick={() => onMove(row.id, -1)} title="Move up">↑</button>
        <button class="icon-btn" disabled={!canMoveDown} onclick={() => onMove(row.id, 1)} title="Move down">↓</button>
      </div>

      <label class="switch" title={explicitShow === undefined ? "Auto: shown while the battery is live" : ""}>
        <input type="checkbox" checked={row.shown} onchange={toggleShown} />
        <span class="track"><span class="thumb"></span></span>
        <span class="switch-label">{row.shown ? "On" : "Off"}{explicitShow === undefined ? " · auto" : ""}</span>
      </label>
    {/if}
    <button class="icon-btn" onclick={() => onChange(row.id, { type: "reset" })} title="Reset settings">⟲</button>
  </div>
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: 40px 1fr auto auto;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    background: var(--card);
    border: 1px solid var(--stroke);
    border-radius: 8px;
  }
  .row.merged,
  .row.missing {
    opacity: 0.6;
  }
  .icon {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: 6px;
    background: var(--subtle);
  }
  .main {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .title,
  .title-input {
    font: inherit;
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
  }
  .title-input {
    border: 1px solid transparent;
    border-radius: 4px;
    background: transparent;
    padding: 1px 4px;
    margin-left: -5px;
  }
  .title-input:hover {
    border-color: var(--stroke);
  }
  .title-input:focus {
    outline: none;
    background: var(--input);
    border-color: var(--stroke);
    border-bottom: 2px solid var(--accent);
  }
  .meta {
    font-size: 12px;
    color: var(--text-secondary);
  }
  .id {
    font-size: 11px;
    color: var(--text-tertiary);
    font-family: "Cascadia Mono", Consolas, monospace;
  }
  .battery {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 72px;
    justify-content: flex-end;
  }
  .percent {
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .dim {
    color: var(--text-tertiary);
  }
  .controls {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .order {
    display: flex;
    gap: 2px;
  }
  .switch {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    min-width: 92px;
  }
  .switch input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .track {
    position: relative;
    width: 40px;
    height: 20px;
    border-radius: 10px;
    border: 1px solid var(--text-secondary);
    box-sizing: border-box;
    transition: background 0.15s;
  }
  .thumb {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--text-secondary);
    transition: transform 0.15s;
  }
  .switch input:checked + .track {
    background: var(--accent);
    border-color: var(--accent);
  }
  .switch input:checked + .track .thumb {
    transform: translateX(20px);
    background: var(--on-accent);
  }
  .switch-label {
    font-size: 12px;
    color: var(--text-secondary);
  }
</style>
