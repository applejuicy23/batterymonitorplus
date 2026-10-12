import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { DeviceChange, Snapshot } from "./types";

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

/** Calls `onSnapshot` every time the background poll produces new data. */
export function listenForSnapshots(onSnapshot: (s: Snapshot) => void): Promise<UnlistenFn> {
  return listen<Snapshot>("snapshot", (event) => onSnapshot(event.payload));
}
