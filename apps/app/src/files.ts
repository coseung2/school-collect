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
