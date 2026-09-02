// The annotation editor. Rust hands over the cropped PNG; everything drawn
// here stays in `shapes` and is re-rendered from the bitmap on every change,
// so undo is a matter of dropping the last snapshot.

import { listen } from "@tauri-apps/api/event";
import {
  appInfo,
  exportAction,
  exportPng,
  fetchBitmap,
  getSettings,
  namingTokens,
  pickSaveDir,
  previewFileName,
  setSettings,
  startCapture,
  type ExportAction,
  type Settings,
} from "../shared/ipc";

type Tool = "arrow" | "line" | "rect" | "ellipse" | "pen" | "highlight" | "text" | "step" | "blur";

interface Stroke {
  color: string;
  width: number;
}

interface BoxShape extends Stroke {
  kind: "arrow" | "line" | "rect" | "ellipse" | "blur";
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

interface PathShape extends Stroke {
  kind: "pen" | "highlight";
  points: number[];
}

interface TextShape extends Stroke {
  kind: "text";
  x: number;
  y: number;
  text: string;
}

interface StepShape extends Stroke {
  kind: "step";
  x: number;
  y: number;
  n: number;
}

type Shape = BoxShape | PathShape | TextShape | StepShape;

function isBox(shape: Shape): shape is BoxShape {
  return (
    shape.kind === "arrow" ||
    shape.kind === "line" ||
    shape.kind === "rect" ||
    shape.kind === "ellipse" ||
    shape.kind === "blur"
  );
}

const TOOL_KEYS: Record<string, Tool> = {
  a: "arrow",
  l: "line",
  r: "rect",
  e: "ellipse",
  p: "pen",
  h: "highlight",
  t: "text",
  n: "step",
  b: "blur",
};

const $ = <T extends HTMLElement>(id: string): T => {
  const element = document.getElementById(id);
  if (!element) throw new Error(`missing #${id}`);
  return element as T;
};

const stage = $<HTMLElement>("stage");
const empty = $<HTMLDivElement>("empty");
const wrap = $<HTMLDivElement>("canvas-wrap");
const canvas = $<HTMLCanvasElement>("canvas");
const textInput = $<HTMLTextAreaElement>("text-input");
const statusText = $<HTMLSpanElement>("status-text");
const dimensions = $<HTMLSpanElement>("dimensions");
const widthInput = $<HTMLInputElement>("width");
const customColor = $<HTMLInputElement>("custom-color");
const ctx = canvas.getContext("2d")!;
const offscreen = document.createElement("canvas");

let bitmap: ImageBitmap | null = null;
let shapes: Shape[] = [];
let history: Shape[][] = [];
let future: Shape[][] = [];
let draft: Shape | null = null;
let tool: Tool = "arrow";
let color = "#ff3b30";
let strokeWidth = 4;
/** Image pixels per nominal point, so strokes look the same on a Retina crop. */
let unit = 1;
let textAnchor: { x: number; y: number } | null = null;
let busy = false;
let settings: Settings | null = null;
let prefsTimer: number | undefined;

// ---------- geometry ----------

function normalize(shape: BoxShape): { x: number; y: number; w: number; h: number } {
  const x = Math.min(shape.x1, shape.x2);
  const y = Math.min(shape.y1, shape.y2);
  return { x, y, w: Math.abs(shape.x2 - shape.x1), h: Math.abs(shape.y2 - shape.y1) };
}

function toImage(event: PointerEvent | MouseEvent): { x: number; y: number } {
  const rect = canvas.getBoundingClientRect();
  const scale = canvas.width / rect.width;
  return {
    x: Math.max(0, Math.min(canvas.width, (event.clientX - rect.left) * scale)),
    y: Math.max(0, Math.min(canvas.height, (event.clientY - rect.top) * scale)),
  };
}

function fontSize(width: number): number {
  return (10 + width * 3) * unit;
}

function isLight(hex: string): boolean {
  const value = hex.replace("#", "");
  const r = parseInt(value.slice(0, 2), 16);
  const g = parseInt(value.slice(2, 4), 16);
  const b = parseInt(value.slice(4, 6), 16);
  return (r * 299 + g * 587 + b * 114) / 1000 > 150;
}

// ---------- rendering ----------

function drawShape(shape: Shape): void {
  ctx.save();
  ctx.strokeStyle = shape.color;
  ctx.fillStyle = shape.color;
  ctx.lineWidth = shape.width * unit;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";

  switch (shape.kind) {
    case "rect": {
      const { x, y, w, h } = normalize(shape);
      ctx.strokeRect(x, y, w, h);
      break;
    }
    case "ellipse": {
      const { x, y, w, h } = normalize(shape);
      ctx.beginPath();
      ctx.ellipse(x + w / 2, y + h / 2, w / 2, h / 2, 0, 0, Math.PI * 2);
      ctx.stroke();
      break;
    }
    case "line": {
      ctx.beginPath();
      ctx.moveTo(shape.x1, shape.y1);
      ctx.lineTo(shape.x2, shape.y2);
      ctx.stroke();
      break;
    }
    case "arrow": {
      const dx = shape.x2 - shape.x1;
      const dy = shape.y2 - shape.y1;
      const length = Math.hypot(dx, dy);
      if (length < 1) break;
      const angle = Math.atan2(dy, dx);
      const head = Math.max(12 * unit, ctx.lineWidth * 3.5);
      const bodyEnd = length - head * 0.7;
      ctx.beginPath();
      ctx.moveTo(shape.x1, shape.y1);
      ctx.lineTo(shape.x1 + Math.cos(angle) * bodyEnd, shape.y1 + Math.sin(angle) * bodyEnd);
      ctx.stroke();
      ctx.beginPath();
      ctx.moveTo(shape.x2, shape.y2);
      ctx.lineTo(
        shape.x2 - head * Math.cos(angle - Math.PI / 6),
        shape.y2 - head * Math.sin(angle - Math.PI / 6),
      );
      ctx.lineTo(
        shape.x2 - head * Math.cos(angle + Math.PI / 6),
        shape.y2 - head * Math.sin(angle + Math.PI / 6),
      );
      ctx.closePath();
      ctx.fill();
      break;
    }
    case "pen":
    case "highlight": {
      if (shape.kind === "highlight") {
        ctx.lineWidth = shape.width * unit * 3;
        ctx.globalAlpha = 0.4;
        ctx.globalCompositeOperation = "multiply";
        ctx.lineCap = "butt";
      }
      const points = shape.points;
      if (points.length < 4) {
        ctx.beginPath();
        ctx.arc(points[0] ?? 0, points[1] ?? 0, ctx.lineWidth / 2, 0, Math.PI * 2);
        ctx.fill();
        break;
      }
      ctx.beginPath();
      ctx.moveTo(points[0] ?? 0, points[1] ?? 0);
      for (let i = 2; i < points.length; i += 2) {
        ctx.lineTo(points[i] ?? 0, points[i + 1] ?? 0);
      }
      ctx.stroke();
      break;
    }
    case "text": {
      const size = fontSize(shape.width);
      ctx.font = `700 ${size}px system-ui, -apple-system, "Segoe UI", sans-serif`;
      ctx.textBaseline = "top";
      ctx.lineJoin = "round";
      ctx.lineWidth = Math.max(2, size / 7);
      ctx.strokeStyle = isLight(shape.color) ? "rgba(0,0,0,0.75)" : "rgba(255,255,255,0.85)";
      const lines = shape.text.split("\n");
      lines.forEach((line, i) => {
        const y = shape.y + i * size * 1.2;
        ctx.strokeText(line, shape.x, y);
        ctx.fillText(line, shape.x, y);
      });
      break;
    }
    case "step": {
      const radius = (10 + shape.width) * unit;
      ctx.beginPath();
      ctx.arc(shape.x, shape.y, radius, 0, Math.PI * 2);
      ctx.fill();
      ctx.lineWidth = Math.max(1.5 * unit, radius / 8);
      ctx.strokeStyle = isLight(shape.color) ? "#111" : "#fff";
      ctx.stroke();
      ctx.fillStyle = isLight(shape.color) ? "#111" : "#fff";
      ctx.font = `700 ${radius * 1.15}px system-ui, -apple-system, "Segoe UI", sans-serif`;
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText(String(shape.n), shape.x, shape.y + radius * 0.05);
      break;
    }
    case "blur": {
      const { x, y, w, h } = normalize(shape);
      if (w < 1 || h < 1) {
        break;
      }
      const block = Math.max(4 * unit, Math.min(40 * unit, Math.min(w, h) / 6));
      const sw = Math.max(1, Math.ceil(w / block));
      const sh = Math.max(1, Math.ceil(h / block));
      offscreen.width = sw;
      offscreen.height = sh;
      const off = offscreen.getContext("2d")!;
      off.imageSmoothingEnabled = true;
      off.clearRect(0, 0, sw, sh);
      // Pixelate whatever is already composited under the box, image and
      // earlier annotations alike, so ordering behaves like a real editor.
      off.drawImage(canvas, x, y, w, h, 0, 0, sw, sh);
      ctx.imageSmoothingEnabled = false;
      ctx.drawImage(offscreen, 0, 0, sw, sh, x, y, w, h);
      ctx.imageSmoothingEnabled = true;
      if (draft === shape) {
        ctx.strokeStyle = "rgba(255,255,255,0.8)";
        ctx.lineWidth = unit;
        ctx.setLineDash([4 * unit, 4 * unit]);
        ctx.strokeRect(x, y, w, h);
      }
      break;
    }
  }
  ctx.restore();
}

function render(): void {
  if (!bitmap) return;
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.globalAlpha = 1;
  ctx.globalCompositeOperation = "source-over";
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.drawImage(bitmap, 0, 0);
  for (const shape of shapes) drawShape(shape);
  if (draft) drawShape(draft);
}

/** Size the canvas element to fit the stage without exceeding 1:1 pixels. */
function fit(): void {
  if (!bitmap) return;
  const dpr = window.devicePixelRatio || 1;
  const naturalW = bitmap.width / dpr;
  const naturalH = bitmap.height / dpr;
  const availW = stage.clientWidth - 32;
  const availH = stage.clientHeight - 32;
  const scale = Math.min(1, availW / naturalW, availH / naturalH);
  canvas.style.width = `${Math.max(1, Math.floor(naturalW * scale))}px`;
  canvas.style.height = `${Math.max(1, Math.floor(naturalH * scale))}px`;
}

// ---------- history ----------

function snapshot(): void {
  history.push(shapes.slice());
  future = [];
  if (history.length > 200) history.shift();
  updateHistoryButtons();
}

function undo(): void {
  const previous = history.pop();
  if (!previous) return;
  future.push(shapes);
  shapes = previous;
  render();
  updateHistoryButtons();
}

function redo(): void {
  const next = future.pop();
  if (!next) return;
  history.push(shapes);
  shapes = next;
  render();
  updateHistoryButtons();
}

function updateHistoryButtons(): void {
  $<HTMLButtonElement>("undo").disabled = history.length === 0;
  $<HTMLButtonElement>("redo").disabled = future.length === 0;
}

function commit(shape: Shape): void {
  snapshot();
  shapes = [...shapes, shape];
  render();
}

// ---------- tools ----------

/** Persist the last used tool, colour and width, debounced. */
function rememberPrefs(): void {
  if (!settings) return;
  settings.editor = { tool, color, width: strokeWidth };
  window.clearTimeout(prefsTimer);
  const snapshotSettings = { ...settings, editor: { ...settings.editor } };
  prefsTimer = window.setTimeout(() => {
    setSettings(snapshotSettings).catch(() => {
      // A failed preference write is not worth interrupting the user.
    });
  }, 400);
}

function setTool(next: Tool): void {
  finishText();
  tool = next;
  document.querySelectorAll<HTMLButtonElement>("#tools button").forEach((button) => {
    button.classList.toggle("active", button.dataset.tool === next);
  });
  canvas.className = `tool-${next}`;
  rememberPrefs();
}

function setColor(next: string): void {
  color = next;
  customColor.value = next;
  document.querySelectorAll<HTMLButtonElement>(".swatch").forEach((button) => {
    button.classList.toggle("active", button.dataset.color === next);
  });
  if (textAnchor) textInput.style.color = next;
  rememberPrefs();
}

function nextStep(): number {
  return shapes.reduce((max, s) => (s.kind === "step" ? Math.max(max, s.n) : max), 0) + 1;
}

function constrain(shape: BoxShape, shift: boolean): void {
  if (!shift) return;
  const dx = shape.x2 - shape.x1;
  const dy = shape.y2 - shape.y1;
  if (shape.kind === "rect" || shape.kind === "ellipse" || shape.kind === "blur") {
    const side = Math.max(Math.abs(dx), Math.abs(dy));
    shape.x2 = shape.x1 + Math.sign(dx || 1) * side;
    shape.y2 = shape.y1 + Math.sign(dy || 1) * side;
  } else {
    const angle = Math.round(Math.atan2(dy, dx) / (Math.PI / 4)) * (Math.PI / 4);
    const length = Math.hypot(dx, dy);
    shape.x2 = shape.x1 + Math.cos(angle) * length;
    shape.y2 = shape.y1 + Math.sin(angle) * length;
  }
}

canvas.addEventListener("pointerdown", (event) => {
  if (!bitmap || event.button !== 0) return;
  const p = toImage(event);
  if (tool === "text") {
    if (textAnchor) {
      finishText();
      return;
    }
    beginText(p, event);
    return;
  }
  finishText();
  if (tool === "step") {
    commit({ kind: "step", x: p.x, y: p.y, n: nextStep(), color, width: strokeWidth });
    return;
  }
  canvas.setPointerCapture(event.pointerId);
  if (tool === "pen" || tool === "highlight") {
    draft = { kind: tool, points: [p.x, p.y], color, width: strokeWidth };
  } else {
    draft = { kind: tool, x1: p.x, y1: p.y, x2: p.x, y2: p.y, color, width: strokeWidth };
  }
  render();
});

canvas.addEventListener("pointermove", (event) => {
  const current = draft;
  if (!current) return;
  const p = toImage(event);
  switch (current.kind) {
    case "pen":
    case "highlight":
      current.points.push(p.x, p.y);
      break;
    case "arrow":
    case "line":
    case "rect":
    case "ellipse":
    case "blur":
      current.x2 = p.x;
      current.y2 = p.y;
      constrain(current, event.shiftKey);
      break;
    default:
      return;
  }
  render();
});

function endDraft(): void {
  if (!draft) return;
  const finished = draft;
  draft = null;
  if (finished.kind === "pen" || finished.kind === "highlight") {
    commit(finished);
    return;
  }
  if (!isBox(finished)) return;
  const { w, h } = normalize(finished);
  const isLine = finished.kind === "line" || finished.kind === "arrow";
  if ((isLine && Math.hypot(w, h) >= 3) || (!isLine && w >= 3 && h >= 3)) {
    commit(finished);
  } else {
    render();
  }
}

canvas.addEventListener("pointerup", (event) => {
  if (event.button !== 0) return;
  endDraft();
});

canvas.addEventListener("pointercancel", () => {
  draft = null;
  render();
});

// ---------- text ----------

function beginText(p: { x: number; y: number }, event: PointerEvent): void {
  textAnchor = p;
  const rect = canvas.getBoundingClientRect();
  const cssScale = rect.width / canvas.width;
  textInput.hidden = false;
  textInput.value = "";
  textInput.style.left = `${event.clientX - rect.left + canvas.offsetLeft}px`;
  textInput.style.top = `${event.clientY - rect.top + canvas.offsetTop}px`;
  textInput.style.fontSize = `${fontSize(strokeWidth) * cssScale}px`;
  textInput.style.color = color;
  autosizeText();
  textInput.focus();
}

function autosizeText(): void {
  textInput.rows = Math.max(1, textInput.value.split("\n").length);
  textInput.style.width = "auto";
  textInput.style.width = `${textInput.scrollWidth + 8}px`;
}

function finishText(): void {
  if (!textAnchor) return;
  const text = textInput.value.replace(/\s+$/, "");
  const anchor = textAnchor;
  textAnchor = null;
  textInput.hidden = true;
  textInput.value = "";
  if (text.trim()) {
    commit({ kind: "text", x: anchor.x, y: anchor.y, text, color, width: strokeWidth });
  }
}

function cancelText(): void {
  textAnchor = null;
  textInput.hidden = true;
  textInput.value = "";
}

textInput.addEventListener("input", autosizeText);
textInput.addEventListener("keydown", (event) => {
  if (event.key === "Enter" && !event.shiftKey) {
    event.preventDefault();
    finishText();
  } else if (event.key === "Escape") {
    event.preventDefault();
    cancelText();
  }
  event.stopPropagation();
});
textInput.addEventListener("blur", () => finishText());

// ---------- export ----------

function setStatus(text: string, path?: string | null): void {
  statusText.textContent = text;
  if (path) {
    const link = document.createElement("a");
    link.href = "#";
    link.textContent = ` ${path}`;
    link.title = "Show in folder";
    link.addEventListener("click", (event) => {
      event.preventDefault();
      void exportAction("reveal");
    });
    statusText.append(link);
  }
}

async function toPng(): Promise<Blob> {
  finishText();
  draft = null;
  render();
  return new Promise((resolve, reject) => {
    canvas.toBlob((blob) => (blob ? resolve(blob) : reject(new Error("encode failed"))), "image/png");
  });
}

async function doExport(action: ExportAction): Promise<void> {
  if (!bitmap || busy) return;
  busy = true;
  setStatus(action === "copy" ? "Copying…" : "Saving…");
  try {
    const result = await exportPng(await toPng(), action);
    switch (result.action) {
      case "copy":
        setStatus("Copied to clipboard");
        break;
      case "save":
      case "save-as":
        setStatus("Saved", result.path);
        break;
      case "cancelled":
        setStatus("Save cancelled");
        break;
      default:
        setStatus("Done");
    }
  } catch (error) {
    setStatus(`Failed: ${String(error)}`);
  } finally {
    busy = false;
  }
}

// ---------- loading ----------

async function load(): Promise<void> {
  try {
    const next = await fetchBitmap("editor_image");
    bitmap?.close();
    bitmap = next;
  } catch {
    empty.hidden = false;
    wrap.hidden = true;
    return;
  }
  resetForBitmap();
  setStatus("Captured");
}

function resetForBitmap(): void {
  if (!bitmap) return;
  shapes = [];
  history = [];
  future = [];
  draft = null;
  cancelText();
  updateHistoryButtons();
  canvas.width = bitmap.width;
  canvas.height = bitmap.height;
  unit = Math.max(1, Math.min(3, Math.round(bitmap.width / 1400)));
  empty.hidden = true;
  wrap.hidden = false;
  fit();
  render();
  dimensions.textContent = `${bitmap.width} × ${bitmap.height}`;
  setStatus("Loaded");
}

// ---------- wiring ----------

document.querySelectorAll<HTMLButtonElement>("#tools button").forEach((button) => {
  button.addEventListener("click", () => setTool(button.dataset.tool as Tool));
});
document.querySelectorAll<HTMLButtonElement>(".swatch").forEach((button) => {
  button.addEventListener("click", () => setColor(button.dataset.color ?? color));
});
customColor.addEventListener("input", () => setColor(customColor.value));
widthInput.addEventListener("input", () => {
  strokeWidth = Number(widthInput.value);
  rememberPrefs();
  if (textAnchor) {
    const rect = canvas.getBoundingClientRect();
    textInput.style.fontSize = `${fontSize(strokeWidth) * (rect.width / canvas.width)}px`;
    autosizeText();
  }
});
$("undo").addEventListener("click", undo);
$("redo").addEventListener("click", redo);
$("clear").addEventListener("click", () => {
  finishText();
  if (shapes.length === 0) return;
  snapshot();
  shapes = [];
  render();
});
$("copy").addEventListener("click", () => void doExport("copy"));
$("save").addEventListener("click", () => void doExport("save"));
$("save-as").addEventListener("click", () => void doExport("save-as"));
$("new-capture").addEventListener("click", () => void startCapture());

window.addEventListener("keydown", (event) => {
  if (event.target === textInput) return;
  const mod = event.metaKey || event.ctrlKey;
  const key = event.key.toLowerCase();
  if (mod && key === "z") {
    event.preventDefault();
    event.shiftKey ? redo() : undo();
  } else if (mod && key === "y") {
    event.preventDefault();
    redo();
  } else if (mod && key === "c") {
    event.preventDefault();
    void doExport("copy");
  } else if (mod && key === "s") {
    event.preventDefault();
    void doExport(event.shiftKey ? "save-as" : "save");
  } else if (mod && key === "n") {
    event.preventDefault();
    void startCapture();
  } else if (event.key === "Escape") {
    if (draft) {
      draft = null;
      render();
    }
  } else if (!mod && !event.altKey) {
    const next = TOOL_KEYS[key];
    if (next) {
      event.preventDefault();
      setTool(next);
    }
  }
});

window.addEventListener("resize", fit);
window.addEventListener("dragover", (event) => event.preventDefault());
window.addEventListener("drop", (event) => {
  // Only reaches here outside Tauri (a plain browser during development);
  // inside the app the window's native drag-drop handler opens the file.
  event.preventDefault();
  const file = event.dataTransfer?.files[0];
  if (!file || !file.type.startsWith("image/")) return;
  void createImageBitmap(file).then((next) => {
    bitmap?.close();
    bitmap = next;
    resetForBitmap();
  });
});
window.addEventListener("contextmenu", (event) => event.preventDefault());

// ---------- settings panel ----------

const panel = $<HTMLElement>("settings");
const form = $<HTMLFormElement>("settings-form");
const settingsError = $<HTMLParagraphElement>("settings-error");

function field<T extends HTMLElement>(name: string): T {
  const element = form.elements.namedItem(name);
  if (!element) throw new Error(`missing settings field ${name}`);
  return element as T;
}

function openSettings(): void {
  finishText();
  const current = settings;
  if (!current) {
    setStatus("Settings are unavailable outside the app");
    return;
  }
  field<HTMLSelectElement>("after_capture").value = current.after_capture;
  field<HTMLInputElement>("close_on_copy").checked = current.close_on_copy;
  field<HTMLInputElement>("close_on_save").checked = current.close_on_save;
  field<HTMLInputElement>("save_dir").value = current.save_dir ?? "";
  field<HTMLInputElement>("file_pattern").value = current.file_pattern;
  field<HTMLInputElement>("shortcut").value = current.shortcut;
  field<HTMLInputElement>("launch_at_login").checked = current.launch_at_login;
  field<HTMLInputElement>("notify").checked = current.notify;
  field<HTMLSelectElement>("dock_icon").value = current.dock_icon;
  field<HTMLInputElement>("check_updates").checked = current.check_updates;
  $("dock-row").hidden = !navigator.platform.toLowerCase().includes("mac");
  void updatePatternPreview();
  settingsError.hidden = true;
  panel.hidden = false;
  field<HTMLSelectElement>("after_capture").focus();
}

function closeSettings(): void {
  panel.hidden = true;
}

function readForm(): Settings {
  const base = settings ?? {
    after_capture: "edit",
    close_on_copy: true,
    close_on_save: false,
    save_dir: null,
    file_pattern: "capture-%datetime%",
    shortcut: "CmdOrCtrl+Shift+2",
    launch_at_login: false,
    notify: true,
    dock_icon: "never",
    check_updates: true,
    skip_move_prompt: false,
    editor: { tool, color, width: strokeWidth },
  };
  const dir = field<HTMLInputElement>("save_dir").value.trim();
  return {
    ...base,
    after_capture: field<HTMLSelectElement>("after_capture").value as Settings["after_capture"],
    close_on_copy: field<HTMLInputElement>("close_on_copy").checked,
    close_on_save: field<HTMLInputElement>("close_on_save").checked,
    save_dir: dir === "" ? null : dir,
    file_pattern: field<HTMLInputElement>("file_pattern").value.trim(),
    shortcut: field<HTMLInputElement>("shortcut").value.trim(),
    launch_at_login: field<HTMLInputElement>("launch_at_login").checked,
    notify: field<HTMLInputElement>("notify").checked,
    dock_icon: field<HTMLSelectElement>("dock_icon").value as Settings["dock_icon"],
    check_updates: field<HTMLInputElement>("check_updates").checked,
    editor: { tool, color, width: strokeWidth },
  };
}

const patternPreview = $<HTMLParagraphElement>("pattern-preview");
let previewTimer: number | undefined;

async function updatePatternPreview(): Promise<void> {
  const pattern = field<HTMLInputElement>("file_pattern").value.trim();
  try {
    const path = await previewFileName(pattern);
    patternPreview.textContent = `Example: ${path}`;
    patternPreview.classList.remove("invalid");
  } catch (error) {
    patternPreview.textContent = String(error);
    patternPreview.classList.add("invalid");
  }
}

field<HTMLInputElement>("file_pattern").addEventListener("input", () => {
  window.clearTimeout(previewTimer);
  previewTimer = window.setTimeout(() => void updatePatternPreview(), 150);
});

async function fillTokenList(): Promise<void> {
  const list = $<HTMLDListElement>("token-list");
  list.replaceChildren();
  for (const [token, help] of await namingTokens()) {
    const dt = document.createElement("dt");
    dt.textContent = token;
    const dd = document.createElement("dd");
    dd.textContent = help;
    list.append(dt, dd);
  }
}

form.addEventListener("submit", (event) => {
  event.preventDefault();
  void (async () => {
    try {
      settings = await setSettings(readForm());
      closeSettings();
      applyShortcutHint(settings.shortcut);
      setStatus("Settings saved");
    } catch (error) {
      settingsError.textContent = String(error);
      settingsError.hidden = false;
    }
  })();
});
$("settings-cancel").addEventListener("click", closeSettings);
$("open-settings").addEventListener("click", () => (panel.hidden ? openSettings() : closeSettings()));
$("pick-dir").addEventListener("click", () => {
  void pickSaveDir().then((dir) => {
    if (dir) field<HTMLInputElement>("save_dir").value = dir;
  });
});
$("reset-dir").addEventListener("click", () => {
  field<HTMLInputElement>("save_dir").value = "";
});
form.addEventListener("keydown", (event) => {
  // Keep tool shortcuts and Escape-to-cancel-draft out of the form.
  event.stopPropagation();
  if (event.key === "Escape") closeSettings();
});

function applyShortcutHint(shortcut: string): void {
  const mac = navigator.platform.toLowerCase().includes("mac");
  $("shortcut").textContent = mac
    ? shortcut.replace("CmdOrCtrl", "⌘").replace("Shift", "⇧").replace(/\+/g, "")
    : shortcut.replace("CmdOrCtrl", "Ctrl");
}

async function main(): Promise<void> {
  updateHistoryButtons();
  let initialTool: Tool = "arrow";
  let initialColor = "#ff3b30";
  try {
    settings = await getSettings();
    const prefs = settings.editor;
    if (prefs.tool in TOOL_KEYS || Object.values(TOOL_KEYS).includes(prefs.tool as Tool)) {
      initialTool = prefs.tool as Tool;
    }
    if (/^#[0-9a-f]{6}$/i.test(prefs.color)) initialColor = prefs.color;
    strokeWidth = Math.min(12, Math.max(1, prefs.width));
    widthInput.value = String(strokeWidth);
    const info = await appInfo();
    applyShortcutHint(info.shortcut);
    $("settings-path").textContent = `Stored at ${info.settings_path}`;
    field<HTMLInputElement>("save_dir").placeholder = info.default_save_dir;
    await fillTokenList();
  } catch {
    // Outside Tauri (plain-browser development) there are no settings.
  }
  setTool(initialTool);
  setColor(initialColor);
  try {
    await listen("capture-new", () => void load());
    await listen("open-settings", () => openSettings());
  } catch {
    // Outside Tauri there is no event bridge.
  }
  await load();
}

void main();
