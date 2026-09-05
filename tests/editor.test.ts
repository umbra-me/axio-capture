// @vitest-environment jsdom
import html from "../index.html?raw";
import { beforeAll, beforeEach, expect, test, vi } from "vitest";
import * as ipc from "../src/shared/ipc";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));
vi.mock("../src/shared/ipc", () => ({
  appInfo: vi.fn(), exportAction: vi.fn(), exportPng: vi.fn(), fetchBitmap: vi.fn(),
  getSettings: vi.fn(), namingTokens: vi.fn(), pickSaveDir: vi.fn(),
  previewFileName: vi.fn(), setSettings: vi.fn(), startCapture: vi.fn(),
}));
const settings: ipc.Settings = {
  after_capture: "edit", close_on_copy: true, close_on_save: false, save_dir: null,
  file_pattern: "capture-%datetime%", shortcut: "Alt+F9", launch_at_login: false,
  notify: false, dock_icon: "never", check_updates: true, skip_move_prompt: true,
  editor: { tool: "arrow", color: "#ff3b30", width: 4 },
};
const element = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const status = () => element("status-text").textContent;
const click = (id: string) => element(id).click();
beforeAll(async () => {
  document.documentElement.innerHTML = html;
  const context = { setTransform: vi.fn(), clearRect: vi.fn(), drawImage: vi.fn(), save: vi.fn(), restore: vi.fn() };
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(context as never);
  vi.spyOn(HTMLCanvasElement.prototype, "toBlob").mockImplementation(callback => callback(new Blob(["fixture png"], { type: "image/png" })));
  vi.mocked(ipc.fetchBitmap).mockResolvedValue({ width: 128, height: 128, close: vi.fn() } as unknown as ImageBitmap);
  vi.mocked(ipc.getSettings).mockResolvedValue(settings);
  vi.mocked(ipc.setSettings).mockResolvedValue(settings);
  vi.mocked(ipc.appInfo).mockResolvedValue({ version: "0.1.1", shortcut: "Alt+F9", save_dir: "/captures", default_save_dir: "/captures", settings_path: "/fixture/settings.json" });
  vi.mocked(ipc.namingTokens).mockResolvedValue([]);
  vi.mocked(ipc.previewFileName).mockResolvedValue("/captures/example.png");
  await import("../src/editor/main");
  await vi.waitFor(() => expect(status()).toBe("Captured"));
  await vi.waitFor(() => expect(ipc.setSettings).toHaveBeenCalled());
});
beforeEach(() => { vi.mocked(ipc.exportPng).mockReset(); vi.mocked(ipc.setSettings).mockReset().mockResolvedValue(settings); vi.mocked(ipc.exportAction).mockClear(); });

test("attachment result renders a literal path and reveal uses the native action", async () => {
  const path = '/captures/<img src=x onerror=alert(1)>.axio-capture.json';
  vi.mocked(ipc.exportPng).mockResolvedValue({ action: "handoff", path });
  click("handoff");
  await vi.waitFor(() => expect(status()).toContain("Attachment ready"));
  expect(ipc.exportPng).toHaveBeenCalledWith(expect.any(Blob), "handoff");
  expect(element("status-text").querySelector("img")).toBeNull();
  const link = element("status-text").querySelector("a")!;
  expect(link.textContent).toBe(` ${path}`); link.click();
  expect(ipc.exportAction).toHaveBeenCalledExactlyOnceWith("reveal");
});

test("an in-flight export suppresses duplicate actions and unlocks after failure", async () => {
  let reject!: (error: Error) => void;
  vi.mocked(ipc.exportPng).mockReturnValue(new Promise((_resolve, fail) => { reject = fail; }));
  click("save"); click("handoff");
  await vi.waitFor(() => expect(ipc.exportPng).toHaveBeenCalledTimes(1));
  reject(new Error("Disk full"));
  await vi.waitFor(() => expect(status()).toBe("Failed: Error: Disk full"));
  vi.mocked(ipc.exportPng).mockResolvedValue({ action: "copy", path: null }); click("copy");
  await vi.waitFor(() => expect(status()).toBe("Copied to clipboard"));
  expect(ipc.exportPng).toHaveBeenCalledTimes(2);
});

test("cancelled save does not present a success path", async () => {
  vi.mocked(ipc.exportPng).mockResolvedValue({ action: "cancelled", path: null }); click("save-as");
  await vi.waitFor(() => expect(status()).toBe("Save cancelled"));
  expect(element("status-text").querySelector("a")).toBeNull();
});

test("cancelled settings are not persisted; rejected settings stay editable", async () => {
  click("open-settings");
  const form = element<HTMLFormElement>("settings-form");
  const shortcut = form.elements.namedItem("shortcut") as HTMLInputElement;
  shortcut.value = "invalid"; click("settings-cancel");
  expect(ipc.setSettings).not.toHaveBeenCalled(); expect(element("settings").hidden).toBe(true);
  click("open-settings"); expect(shortcut.value).toBe("Alt+F9");
  vi.mocked(ipc.setSettings).mockRejectedValueOnce(new Error("Shortcut unavailable"));
  form.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
  await vi.waitFor(() => expect(element("settings-error").textContent).toContain("Shortcut unavailable"));
  expect(element("settings").hidden).toBe(false);
  vi.mocked(ipc.setSettings).mockResolvedValue(settings);
  form.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
  await vi.waitFor(() => expect(status()).toBe("Settings saved"));
  expect(element("settings").hidden).toBe(true);
  expect(ipc.setSettings).toHaveBeenLastCalledWith(expect.objectContaining({ skip_move_prompt: true, check_updates: true }));
});
