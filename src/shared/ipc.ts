import { invoke } from "@tauri-apps/api/core";

export interface AppInfo {
  version: string;
  shortcut: string;
  save_dir: string;
  default_save_dir: string;
  settings_path: string;
}

export type AfterCapture = "edit" | "copy" | "save" | "copy-save";

export interface EditorPrefs {
  tool: string;
  color: string;
  width: number;
}

export interface Settings {
  after_capture: AfterCapture;
  close_on_copy: boolean;
  close_on_save: boolean;
  save_dir: string | null;
  file_pattern: string;
  shortcut: string;
  launch_at_login: boolean;
  notify: boolean;
  dock_icon: "always" | "while-editor-open" | "never";
  check_updates: boolean;
  skip_move_prompt: boolean;
  editor: EditorPrefs;
}

export interface ExportResult {
  action: "copy" | "save" | "save-as" | "handoff" | "reveal" | "cancelled";
  path: string | null;
}

export type ExportAction = "copy" | "save" | "save-as" | "handoff" | "reveal";

/** Fetch PNG bytes from a command that answers with a raw body. */
export async function fetchBitmap(
  command: string,
  args?: Record<string, unknown>,
): Promise<ImageBitmap> {
  const bytes = await invoke<ArrayBuffer>(command, args);
  const blob = new Blob([bytes], { type: "image/png" });
  return createImageBitmap(blob);
}

export function appInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("app_info");
}

export function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

/** Rejects with a human-readable message when a value is invalid. */
export function setSettings(settings: Settings): Promise<Settings> {
  return invoke<Settings>("set_settings", { settings });
}

/** Rejects with a message when the pattern is invalid. */
export function previewFileName(pattern: string): Promise<string> {
  return invoke<string>("preview_file_name", { pattern });
}

export function namingTokens(): Promise<[string, string][]> {
  return invoke<[string, string][]>("naming_tokens");
}

export function pickSaveDir(): Promise<string | null> {
  return invoke<string | null>("pick_save_dir");
}

export function startCapture(): Promise<void> {
  return invoke("start_capture");
}

export function overlayShow(): Promise<void> {
  return invoke("overlay_show");
}

export function overlayCancel(): Promise<void> {
  return invoke("overlay_cancel");
}

export function overlayConfirm(selection: {
  monitor: number;
  x: number;
  y: number;
  width: number;
  height: number;
  viewWidth: number;
  viewHeight: number;
}): Promise<void> {
  return invoke("overlay_confirm", selection);
}

/** Send the editor's PNG to Rust with the action in a header. */
export async function exportPng(png: Blob, action: ExportAction): Promise<ExportResult> {
  const bytes = new Uint8Array(await png.arrayBuffer());
  return invoke<ExportResult>("export_png", bytes, {
    headers: { "x-axio-action": action },
  });
}

/** Actions without a body reuse the same command with an empty marker byte. */
export function exportAction(action: ExportAction): Promise<ExportResult> {
  return invoke<ExportResult>("export_png", new Uint8Array([0]), {
    headers: { "x-axio-action": action },
  });
}
