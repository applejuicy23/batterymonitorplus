import type { Device, Source } from "./types";

const SOURCE_NAMES: Record<Source, string> = {
  system: "This PC",
  bluetooth: "Bluetooth",
  razer: "Razer HID",
  yunzii: "2.4G dongle",
};

/** Turns a source id into a human name, e.g. "yunzii" → "2.4G dongle". */
export function describeSource(source: Source): string {
  return SOURCE_NAMES[source];
}

/** Turns a unix timestamp into "just now" / "5m ago" / "3h ago" / "2d ago". */
export function formatAge(unixSeconds: number): string {
  const age = Math.max(0, Date.now() / 1000 - unixSeconds);
  if (age < 60) return "just now";
  if (age < 3600) return `${Math.floor(age / 60)}m ago`;
  if (age < 86400) return `${Math.floor(age / 3600)}h ago`;
  return `${Math.floor(age / 86400)}d ago`;
}

/** Describes connection/charging in a few words, like `bmp-probe` does. */
export function describeState(device: Device): string {
  if (device.connected === false) {
    if (device.battery !== null && device.updated_at !== null) {
      return `disconnected, battery from ${formatAge(device.updated_at)}`;
    }
    return "disconnected";
  }
  if (device.charging === true) return "charging";
  if (device.charging === false) return "on battery";
  if (device.connected === true) return "connected";
  return "connection unknown";
}

/** Picks the battery color level: low (< 20 %), mid (< 40 %) or ok. */
export function rateBatteryLevel(percent: number | null): "low" | "mid" | "ok" | "none" {
  if (percent === null) return "none";
  if (percent < 20) return "low";
  if (percent < 40) return "mid";
  return "ok";
}
