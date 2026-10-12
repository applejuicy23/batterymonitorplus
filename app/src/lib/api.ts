import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { DeviceChange, Snapshot, TileLayout } from "./types";

/** Asks Rust for the latest snapshot without detecting devices again. */
export function fetchSnapshot(): Promise<Snapshot> {
  return invoke<Snapshot>("get_snapshot");
}

/** Makes Rust detect devices right now and returns the fresh snapshot. */
export function refreshDevices(): Promise<Snapshot> {
  return invoke<Snapshot>("refresh_devices");
}

/** Saves one settings change for a device and returns the updated snapshot. */
export function updateDevice(id: string, change: DeviceChange): Promise<Snapshot> {
  return invoke<Snapshot>("update_device", { id, change });
}

/** Saves how often batteries are re-read. */
export function savePollSeconds(seconds: number): Promise<Snapshot> {
  return invoke<Snapshot>("set_poll_seconds", { seconds });
}

/** Saves how the taskbar tiles are drawn. */
export function saveTileLayout(layout: TileLayout): Promise<Snapshot> {
  return invoke<Snapshot>("set_tile_layout", { layout });
}

/** Tells Rust how wide the tiles are, so it can dock the window next to the tray. */
export function reportTilesWidth(width: number): Promise<void> {
  return invoke("set_tiles_width", { width });
}

/** Shifts the tiles sideways while they're dragged (`dx` in px, positive = right). */
export function moveTilesBy(dx: number): Promise<void> {
  return invoke("move_tiles_by", { dx });
}

/** Saves where the tiles were dropped. */
export function saveTilesPosition(): Promise<Snapshot> {
  return invoke<Snapshot>("save_tiles_position");
}

/** Puts the tiles back next to the tray. */
export function resetTilesPosition(): Promise<Snapshot> {
  return invoke<Snapshot>("reset_tiles_position");
}

/** Switches the tiles between living inside the taskbar and floating over it. */
export function saveTileEmbed(embed: boolean): Promise<Snapshot> {
  return invoke<Snapshot>("set_tile_embed", { embed });
}

/** Opens the settings window. */
export function openSettings(): Promise<void> {
  return invoke("open_settings");
}

/** Calls `onSnapshot` every time the background poll produces new data. */
export function listenForSnapshots(onSnapshot: (s: Snapshot) => void): Promise<UnlistenFn> {
  return listen<Snapshot>("snapshot", (event) => onSnapshot(event.payload));
}
