import { invoke } from "@tauri-apps/api/core";

/**
 * Writes an exported file into the user's Downloads folder and returns the path.
 *
 * The app never invents the content: it saves what the API already returned.
 */
export async function saveExportFile(
  fileName: string,
  content: string,
): Promise<string> {
  return invoke<string>("save_export_file", { fileName, content });
}

/**
 * Saves a downloaded attachment into the Downloads folder and returns the path.
 * An existing file is never overwritten; the native side adds a suffix.
 */
export async function saveDownloadedFile(
  fileName: string,
  content: Uint8Array,
): Promise<string> {
  return invoke<string>("save_downloaded_file", {
    fileName: safeFileName(fileName),
    content: Array.from(content),
  });
}

/** Keeps the original name readable while dropping characters the native side refuses. */
function safeFileName(name: string): string {
  const cleaned = name
    .replace(/[\\/:*?"<>|\t\r\n]/g, " ")
    .replace(/\.{2,}/g, ".")
    .replace(/\s+/g, " ")
    .trim()
    .replace(/^\.+/, "");
  return (cleaned || "attachment").slice(0, 120);
}

/** A file name safe to hand to the native save command. */
export function exportFileName(title: string, date: Date): string {
  const safe = title
    .replace(/[\\/:*?"<>|]/g, " ")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, 60);
  const stamp = date.toISOString().slice(0, 10);
  return `${safe || "collect"}-${stamp}.csv`;
}
