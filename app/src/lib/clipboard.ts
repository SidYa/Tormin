/** Clipboard text, or "" when it is unavailable (permission, empty, non-text). */
export async function readText(): Promise<string> {
  try {
    return (await navigator.clipboard.readText()) ?? "";
  } catch {
    return "";
  }
}
