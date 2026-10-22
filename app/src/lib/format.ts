const UNITS = ["B", "KB", "MB", "GB", "TB"];

export function formatSize(bytes: number): string {
  let size = bytes;
  for (const unit of UNITS) {
    if (size < 1024) return unit === "B" ? `${Math.round(size)} ${unit}` : `${size.toFixed(1)} ${unit}`;
    size /= 1024;
  }
  return `${size.toFixed(1)} PB`;
}

export function formatSpeed(bytesPerSec: number): string {
  if (bytesPerSec > 1024 * 1024) return `${(bytesPerSec / 1024 / 1024).toFixed(1)} MB/s`;
  if (bytesPerSec > 1024) return `${(bytesPerSec / 1024).toFixed(1)} KB/s`;
  return `${bytesPerSec} B/s`;
}

export function formatEta(seconds: number): string {
  if (!isFinite(seconds) || seconds <= 0) return "";
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m ${s}s`;
  return `${s}s`;
}

/** Display name from a magnet link's dn= parameter. */
export function magnetName(uri: string): string {
  const query = uri.slice(uri.indexOf("?") + 1);
  for (const part of query.split("&")) {
    if (part.toLowerCase().startsWith("dn=")) {
      try {
        return decodeURIComponent(part.slice(3).replace(/\+/g, " "));
      } catch {
        return part.slice(3);
      }
    }
  }
  return "Magnet Link";
}

export const isMagnet = (s: string) => s.trim().toLowerCase().startsWith("magnet:");
