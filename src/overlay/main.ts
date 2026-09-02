// Region selection over one frozen monitor. Each monitor gets its own
// overlay window and page; they share nothing except the Rust state.

import { fetchBitmap, overlayCancel, overlayConfirm, overlayShow } from "../shared/ipc";

const monitor = Number(new URLSearchParams(location.search).get("monitor") ?? "0");
const canvas = document.getElementById("screen") as HTMLCanvasElement;
const hint = document.getElementById("hint") as HTMLDivElement;
const ctx = canvas.getContext("2d")!;

interface Point {
  x: number;
  y: number;
}

let bitmap: ImageBitmap | null = null;
let dpr = window.devicePixelRatio || 1;
let anchor: Point | null = null;
let cursor: Point | null = null;
let done = false;

function selection(): { x: number; y: number; width: number; height: number } | null {
  if (!anchor || !cursor) return null;
  const x = Math.min(anchor.x, cursor.x);
  const y = Math.min(anchor.y, cursor.y);
  return { x, y, width: Math.abs(cursor.x - anchor.x), height: Math.abs(cursor.y - anchor.y) };
}

function resize(): void {
  dpr = window.devicePixelRatio || 1;
  canvas.width = Math.round(innerWidth * dpr);
  canvas.height = Math.round(innerHeight * dpr);
  canvas.style.width = `${innerWidth}px`;
  canvas.style.height = `${innerHeight}px`;
  draw();
}

function draw(): void {
  if (!bitmap) return;
  const w = innerWidth;
  const h = innerHeight;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.drawImage(bitmap, 0, 0, w, h);
  ctx.fillStyle = "rgba(0, 0, 0, 0.45)";
  ctx.fillRect(0, 0, w, h);

  const sel = selection();
  if (sel && sel.width > 0 && sel.height > 0) {
    ctx.save();
    ctx.beginPath();
    ctx.rect(sel.x, sel.y, sel.width, sel.height);
    ctx.clip();
    ctx.drawImage(bitmap, 0, 0, w, h);
    ctx.restore();

    ctx.strokeStyle = "#7c5cff";
    ctx.lineWidth = 1;
    ctx.strokeRect(sel.x + 0.5, sel.y + 0.5, sel.width - 1, sel.height - 1);

    const label = `${Math.round(sel.width * dpr)} × ${Math.round(sel.height * dpr)}`;
    ctx.font = "12px system-ui, -apple-system, 'Segoe UI', sans-serif";
    const padding = 6;
    const textWidth = ctx.measureText(label).width;
    let lx = sel.x;
    let ly = sel.y - 24;
    if (ly < 4) ly = sel.y + sel.height + 4;
    if (lx + textWidth + padding * 2 > w) lx = w - textWidth - padding * 2;
    ctx.fillStyle = "rgba(17, 19, 24, 0.9)";
    ctx.fillRect(lx, ly, textWidth + padding * 2, 20);
    ctx.fillStyle = "#f4f4f5";
    ctx.textBaseline = "middle";
    ctx.fillText(label, lx + padding, ly + 10);
  } else if (cursor) {
    ctx.strokeStyle = "rgba(255, 255, 255, 0.35)";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(cursor.x + 0.5, 0);
    ctx.lineTo(cursor.x + 0.5, h);
    ctx.moveTo(0, cursor.y + 0.5);
    ctx.lineTo(w, cursor.y + 0.5);
    ctx.stroke();
  }
}

function point(event: PointerEvent): Point {
  return { x: event.clientX, y: event.clientY };
}

async function cancel(): Promise<void> {
  if (done) return;
  done = true;
  await overlayCancel();
}

async function confirm(): Promise<void> {
  const sel = selection();
  anchor = null;
  if (!sel || sel.width < 3 || sel.height < 3) {
    draw();
    return;
  }
  if (done) return;
  done = true;
  try {
    await overlayConfirm({
      monitor,
      x: sel.x,
      y: sel.y,
      width: sel.width,
      height: sel.height,
      viewWidth: innerWidth,
      viewHeight: innerHeight,
    });
  } catch (error) {
    console.error(error);
    done = false;
  }
}

canvas.addEventListener("pointerdown", (event) => {
  if (event.button !== 0) return;
  canvas.setPointerCapture(event.pointerId);
  anchor = point(event);
  cursor = anchor;
  hint.classList.add("hidden");
  draw();
});

canvas.addEventListener("pointermove", (event) => {
  cursor = point(event);
  draw();
});

canvas.addEventListener("pointerup", (event) => {
  if (event.button !== 0 || !anchor) return;
  cursor = point(event);
  void confirm();
});

canvas.addEventListener("contextmenu", (event) => {
  event.preventDefault();
  void cancel();
});

window.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    event.preventDefault();
    void cancel();
  }
});

window.addEventListener("resize", resize);
window.addEventListener("blur", () => {
  // Losing focus mid-drag would leave a stuck selection; reset instead.
  anchor = null;
  draw();
});

async function main(): Promise<void> {
  try {
    bitmap = await fetchBitmap("overlay_image", { monitor });
    resize();
    await overlayShow();
  } catch (error) {
    console.error(error);
    await cancel();
  }
}

void main();
