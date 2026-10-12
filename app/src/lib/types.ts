/** Shapes sent by the Rust side; keep in sync with bmp-battery / bmp-config. */

export type DeviceKind = "laptop" | "mouse" | "keyboard" | "headset" | "gamepad" | "other";

export const ALL_KINDS: DeviceKind[] = ["laptop", "mouse", "keyboard", "headset", "gamepad", "other"];

export type Source = "system" | "bluetooth" | "razer" | "yunzii";

export interface Device {
  id: string;
  name: string;
  kind: DeviceKind;
  source: Source;
  battery: number | null;
  charging: boolean | null;
  connected: boolean | null;
  updated_at: number | null;
}

export interface Tile extends Device {
  icon: string;
  live: boolean;
}

export interface DeviceSettings {
  detected_name?: string;
  detected_kind?: DeviceKind;
  show?: boolean;
  name?: string;
  kind?: DeviceKind;
  icon?: string;
  order?: number;
  same_as?: string;
  keep_separate?: boolean;
}

export interface DeviceRow {
  id: string;
  name: string;
  kind: DeviceKind;
  icon: string;
  shown: boolean;
  detected: boolean;
  merged_into: string | null;
  device: Device | null;
  settings: DeviceSettings;
}

export interface Snapshot {
  tiles: Tile[];
  rows: DeviceRow[];
  poll_seconds: number;
  config_path: string;
  error: string | null;
}

export type DeviceChange =
  | { type: "show"; value: boolean | null }
  | { type: "name"; value: string | null }
  | { type: "kind"; value: DeviceKind | null }
  | { type: "order"; value: number | null }
  | { type: "link"; value: string }
  | { type: "unlink" }
  | { type: "reset" };
