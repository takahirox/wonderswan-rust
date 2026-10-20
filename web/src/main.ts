import init, { WebMachine } from "../pkg/ws_web.js";

const controls = document.querySelector<HTMLFieldSetElement>("#controls")!;
const modelSelect = document.querySelector<HTMLSelectElement>("#model")!;
const resetButton = document.querySelector<HTMLButtonElement>("#reset")!;
const pauseButton = document.querySelector<HTMLButtonElement>("#pause")!;
const demoButton = document.querySelector<HTMLButtonElement>("#demo")!;
const romInput = document.querySelector<HTMLInputElement>("#rom")!;
const activeModel = document.querySelector<HTMLParagraphElement>("#active-model")!;
const romStatus = document.querySelector<HTMLParagraphElement>("#rom-status")!;
const errorStatus = document.querySelector<HTMLParagraphElement>("#error")!;
const canvas = document.querySelector<HTMLCanvasElement>("#screen")!;

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

async function main() {
  await init();
  const machine = new WebMachine();
  const context = canvas.getContext("2d");
  if (!context) throw new Error("This browser does not support a 2D canvas.");
  canvas.width = machine.frame_width();
  canvas.height = machine.frame_height();
  context.imageSmoothingEnabled = false;
  let loaded: { name: string; bytes: Uint8Array } | null = null;
  let readRevision = 0;
  let running = false;
  let faulted = false;
  let animation: number | null = null;
  let lastTime: number | null = null;
  let elapsed = 0;
  const frameDuration = 1000 / machine.frame_rate();
  const pressed = new Set<string>();
  const bindings: Record<string, number> = {
    ArrowUp: 16, KeyW: 16, ArrowRight: 32, KeyD: 32,
    ArrowDown: 64, KeyS: 64, ArrowLeft: 128, KeyA: 128,
    Enter: 2, KeyZ: 4, KeyX: 8,
  };

  function clearKeys() {
    pressed.clear();
    machine.set_buttons(0);
  }
  function draw() {
    const pixels = new Uint8ClampedArray(machine.framebuffer());
    context!.putImageData(new ImageData(pixels, canvas.width, canvas.height), 0, 0);
  }
  function refresh() {
    modelSelect.value = machine.model();
    activeModel.textContent = `Active model: ${machine.model_name()}`;
    romStatus.textContent = loaded
      ? `${faulted ? "Stopped" : running ? "Running" : "Paused"}: ${loaded.name} · ${machine.rom_len()} bytes.`
      : "Choose Play demo or a local WonderSwan ROM.";
    pauseButton.disabled = loaded === null || faulted;
    resetButton.disabled = loaded === null;
    pauseButton.textContent = running ? "Pause" : "Resume";
  }
  function setRunning(value: boolean) {
    running = value && loaded !== null && !document.hidden;
    if (animation !== null) cancelAnimationFrame(animation);
    animation = null;
    lastTime = null;
    elapsed = 0;
    clearKeys();
    if (running) animation = requestAnimationFrame(tick);
    refresh();
  }
  function tick(time: number) {
    animation = null;
    if (!running) return;
    if (lastTime !== null) elapsed += Math.min(time - lastTime, 100);
    lastTime = time;
    try {
      // Bounded catch-up. Never run an unbounded loop after suspension.
      let frames = 0;
      while (elapsed >= frameDuration && frames < 4) {
        machine.run_frame();
        elapsed -= frameDuration;
        frames++;
      }
      elapsed = Math.min(elapsed, frameDuration);
      if (frames) draw();
      animation = requestAnimationFrame(tick);
    } catch (error) {
      faulted = true;
      setRunning(false);
      errorStatus.textContent = `Execution stopped: ${message(error)}`;
    }
  }
  function load(name: string, bytes: Uint8Array) {
    machine.load_rom(bytes); // Validation errors preserve the previous machine.
    loaded = { name, bytes };
    faulted = false;
    errorStatus.textContent = "";
    draw();
    setRunning(true);
    canvas.focus();
  }
  function restart(model: string) {
    readRevision++; // Invalidate any in-flight file read or demo fetch.
    setRunning(false);
    machine.reset(model);
    errorStatus.textContent = "";
    if (loaded) {
      try { load(loaded.name, loaded.bytes); }
      catch (error) {
        loaded = null;
        errorStatus.textContent = message(error);
      }
    }
    draw();
    refresh();
  }

  modelSelect.addEventListener("change", () => restart(modelSelect.value));
  resetButton.addEventListener("click", () => restart(machine.model()));
  pauseButton.addEventListener("click", () => {
    setRunning(!running);
    if (running) canvas.focus();
  });
  demoButton.addEventListener("click", async () => {
    const revision = ++readRevision;
    errorStatus.textContent = "";
    try {
      const response = await fetch(`${import.meta.env.BASE_URL}demo/beacon-trail.ws`);
      if (!response.ok) throw new Error(`Demo request failed (${response.status}).`);
      const bytes = new Uint8Array(await response.arrayBuffer());
      if (revision === readRevision) load("Beacon Trail (CC0)", bytes);
    } catch (error) {
      if (revision === readRevision) errorStatus.textContent = `Could not load demo: ${message(error)}`;
    }
  });
  romInput.addEventListener("change", async () => {
    const file = romInput.files?.[0];
    if (!file) return;
    const revision = ++readRevision;
    errorStatus.textContent = "";
    try {
      if (file.size > 4 * 1024 * 1024) throw new Error("Supported ROM size is at most 4 MiB.");
      const bytes = new Uint8Array(await file.arrayBuffer());
      if (revision === readRevision) load(file.name, bytes);
    } catch (error) {
      if (revision === readRevision) errorStatus.textContent = `Could not load “${file.name}”: ${message(error)}`;
    } finally {
      if (revision === readRevision) romInput.value = "";
    }
  });
  window.addEventListener("keydown", (event) => {
    if (!running || !(event.code in bindings) || event.ctrlKey || event.metaKey || event.altKey) return;
    const target = event.target as HTMLElement;
    if (target.closest("input, select, button, textarea, [contenteditable]")) return;
    event.preventDefault();
    pressed.add(event.code);
    machine.set_buttons([...pressed].reduce((mask, code) => mask | bindings[code], 0));
  });
  window.addEventListener("keyup", (event) => {
    if (!pressed.delete(event.code)) return;
    event.preventDefault();
    machine.set_buttons([...pressed].reduce((mask, code) => mask | bindings[code], 0));
  });
  window.addEventListener("blur", clearKeys);
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) setRunning(false);
  });
  draw();
  refresh();
  controls.disabled = false;
}

main().catch((error: unknown) => {
  activeModel.textContent = "Emulator unavailable.";
  errorStatus.textContent = `Could not initialize WebAssembly: ${message(error)}`;
});
