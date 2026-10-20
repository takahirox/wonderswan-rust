import init, { WebMachine } from "../pkg/ws_web.js";

const element = <T extends HTMLElement>(id: string) => document.querySelector<T>(`#${id}`)!;
const modelSelect = element<HTMLSelectElement>("model");
const resetButton = element<HTMLButtonElement>("reset");
const pauseButton = element<HTMLButtonElement>("pause");
const demoButton = element<HTMLButtonElement>("demo");
const loadButton = element<HTMLButtonElement>("load-rom");
const romInput = element<HTMLInputElement>("rom");
const romStatus = element<HTMLParagraphElement>("rom-status");
const errorStatus = element<HTMLParagraphElement>("error");
const canvas = element<HTMLCanvasElement>("screen");
const player = element<HTMLElement>("player");
const help = element<HTMLDialogElement>("help");
const settingsButton = element<HTMLButtonElement>("settings");
const debugToggle = element<HTMLInputElement>("debug-toggle");
const debugPanel = element<HTMLElement>("debug-panel");
const fullscreenButton = element<HTMLButtonElement>("fullscreen");
const retryButton = element<HTMLButtonElement>("retry");
const touchButtons = [...document.querySelectorAll<HTMLButtonElement>("[data-button]")];

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

async function main() {
  retryButton.hidden = true;
  romStatus.textContent = "Loading WebAssembly…";
  errorStatus.textContent = "";
  await init();
  const machine = new WebMachine();
  const context = canvas.getContext("2d");
  if (!context) throw new Error("This browser does not support a 2D canvas.");
  canvas.width = machine.frame_width();
  canvas.height = machine.frame_height();
  context.imageSmoothingEnabled = false;
  let loaded: { name: string; bytes: Uint8Array } | null = null;
  let readRevision = 0;
  let loading = false;
  let running = false;
  let faulted = false;
  let resumeAfterDialog = false;
  let demoAfterDialog = false;
  let visibilityRevision = 0;
  let returnFocus: HTMLElement | null = null;
  let animation: number | null = null;
  let lastTime: number | null = null;
  let elapsed = 0;
  const frameDuration = 1000 / machine.frame_rate();
  const pressed = new Set<string>();
  const pointers = new Map<number, HTMLButtonElement>();
  const bindings: Record<string, number> = {
    ArrowUp: 16, KeyW: 16, ArrowRight: 32, KeyD: 32,
    ArrowDown: 64, KeyS: 64, ArrowLeft: 128, KeyA: 128,
    Enter: 2, KeyZ: 4, KeyX: 8,
  };

  function updateButtons() {
    let mask = [...pressed].reduce((mask, code) => mask | bindings[code], 0);
    for (const button of pointers.values()) mask |= Number(button.dataset.button);
    machine.set_buttons(mask);
    for (const button of touchButtons) button.classList.toggle("is-pressed", [...pointers.values()].includes(button));
  }
  function clearKeys() {
    pressed.clear();
    pointers.clear();
    updateButtons();
  }
  function draw() {
    const pixels = new Uint8ClampedArray(machine.framebuffer());
    context!.putImageData(new ImageData(pixels, canvas.width, canvas.height), 0, 0);
  }
  function state() {
    return loading ? "Loading ROM…" : !loaded ? "Ready to play" : faulted ? "Stopped" : running ? "Running" : "Paused";
  }
  function updateDiagnostics() {
    if (debugPanel.hidden) return;
    element("debug-model").textContent = machine.model_name();
    element("debug-rom").textContent = loaded?.name ?? "None";
    element("debug-bytes").textContent = String(machine.rom_len());
    element("debug-state").textContent = state();
    element("debug-frames").textContent = String(machine.frame_number());
    element("debug-error").textContent = errorStatus.textContent || "None";
  }
  function refresh() {
    modelSelect.value = machine.model();
    element("rom-title").textContent = loaded?.name ?? "Your next game";
    romStatus.textContent = state();
    element("empty-state").hidden = loaded !== null;
    element("controls").setAttribute("aria-busy", String(loading));
    pauseButton.disabled = loaded === null || faulted || loading;
    resetButton.disabled = loaded === null;
    demoButton.disabled = loading;
    demoButton.textContent = loading ? "Loading…" : "Play demo";
    pauseButton.textContent = running ? "Pause" : "Resume";
    for (const button of touchButtons) button.disabled = !running;
    updateDiagnostics();
  }
  function setRunning(value: boolean) {
    running = value && loaded !== null && !faulted && !document.hidden && !help.open;
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
      errorStatus.textContent = `Execution stopped: ${message(error)} Try Restart or load another ROM.`;
      setRunning(false);
    }
  }
  function load(name: string, bytes: Uint8Array, autoplay = true) {
    machine.load_rom(bytes); // Validation errors preserve the previous machine.
    loaded = { name, bytes };
    loading = false;
    faulted = false;
    errorStatus.textContent = "";
    draw();
    // An asynchronous load must not run behind an open modal or steal its focus.
    resumeAfterDialog = autoplay && help.open && !document.hidden;
    setRunning(autoplay);
    if (!help.open) canvas.focus({ preventScroll: true });
  }
  function restart(model: string) {
    readRevision++; // Invalidate any in-flight file read or demo fetch.
    loading = false;
    setRunning(false);
    machine.reset(model);
    errorStatus.textContent = "";
    faulted = false;
    if (loaded) {
      try { load(loaded.name, loaded.bytes); }
      catch (error) {
        loaded = null;
        errorStatus.textContent = `Could not restart: ${message(error)} Load another ROM.`;
      }
    }
    draw();
    refresh();
  }
  async function readROM(name: string, read: () => Promise<Uint8Array>) {
    const revision = ++readRevision;
    const visibleRevision = visibilityRevision;
    const wasRunning = running;
    loading = true;
    errorStatus.textContent = "";
    setRunning(false);
    try {
      const bytes = await read();
      if (revision === readRevision) load(name, bytes, visibleRevision === visibilityRevision);
    } catch (error) {
      if (revision === readRevision) {
        loading = false;
        errorStatus.textContent = `Could not load “${name}”: ${message(error)} Choose Play demo or another ROM.`;
        setRunning(wasRunning && visibleRevision === visibilityRevision);
      }
    } finally {
      if (revision === readRevision) {
        loading = false;
        romInput.value = "";
        refresh();
      }
    }
  }
  function playDemo() {
    void readROM("Beacon Trail", async () => {
      const response = await fetch(`${import.meta.env.BASE_URL}demo/beacon-trail.ws`);
      if (!response.ok) throw new Error(`Demo request failed (${response.status}). Try again.`);
      return new Uint8Array(await response.arrayBuffer());
    });
  }

  modelSelect.addEventListener("change", () => restart(modelSelect.value));
  resetButton.addEventListener("click", () => restart(machine.model()));
  pauseButton.addEventListener("click", () => {
    setRunning(!running);
    if (running) canvas.focus({ preventScroll: true });
  });
  demoButton.addEventListener("click", playDemo);
  loadButton.addEventListener("click", () => { clearKeys(); romInput.click(); });
  romInput.addEventListener("change", () => {
    const file = romInput.files?.[0];
    if (!file) return;
    void readROM(file.name, async () => {
      if (file.size > 4 * 1024 * 1024) throw new Error("Supported ROM size is at most 4 MiB.");
      return new Uint8Array(await file.arrayBuffer());
    });
  });

  settingsButton.addEventListener("click", () => {
    returnFocus = document.activeElement as HTMLElement;
    resumeAfterDialog = running;
    setRunning(false);
    help.showModal();
  });
  element("help-close").addEventListener("click", () => help.close());
  help.addEventListener("close", () => {
    const resume = resumeAfterDialog;
    resumeAfterDialog = false;
    setRunning(resume);
    (running ? canvas : returnFocus ?? settingsButton).focus({ preventScroll: true });
    if (demoAfterDialog) {
      demoAfterDialog = false;
      playDemo();
    }
  });
  element("demo-again").addEventListener("click", () => {
    resumeAfterDialog = false;
    demoAfterDialog = true;
    help.close();
  });
  function toggleDebug(enabled: boolean) {
    debugToggle.checked = enabled;
    debugPanel.hidden = !enabled;
    updateDiagnostics();
  }
  debugToggle.addEventListener("change", () => toggleDebug(debugToggle.checked));
  element("debug-close").addEventListener("click", () => {
    toggleDebug(false);
    canvas.focus({ preventScroll: true });
  });
  // Read the actual core counter at most twice a second, outside frame execution.
  window.setInterval(() => {
    if (!document.hidden && debugPanel.querySelector("details")!.open) updateDiagnostics();
  }, 500);

  fullscreenButton.addEventListener("click", async () => {
    clearKeys();
    const notice = element("notice");
    notice.textContent = "";
    if (!document.fullscreenEnabled || !player.requestFullscreen) {
      notice.textContent = "Fullscreen is unavailable in this browser. You can keep playing here.";
      return;
    }
    try {
      if (document.fullscreenElement) await document.exitFullscreen();
      else await player.requestFullscreen();
      canvas.focus({ preventScroll: true });
    } catch {
      notice.textContent = "Fullscreen could not open. Try again, or keep playing here.";
    }
  });
  document.addEventListener("fullscreenchange", () => {
    fullscreenButton.textContent = document.fullscreenElement ? "Exit fullscreen" : "Fullscreen";
  });
  // Integer scaling where space allows; fractional nearest-neighbor scaling on narrow screens.
  function sizeScreen() {
    const display = element("display");
    const available = Math.min(display.clientWidth, document.fullscreenElement ? display.clientHeight * 224 / 144 : 896);
    const width = available >= 448 ? Math.floor(available / 224) * 224 : available;
    canvas.style.width = `${width}px`;
  }
  new ResizeObserver(sizeScreen).observe(element("display"));

  window.addEventListener("keydown", (event) => {
    if (!running || help.open || document.activeElement !== canvas || !(event.code in bindings)
      || event.ctrlKey || event.metaKey || event.altKey) return;
    event.preventDefault();
    pressed.add(event.code);
    updateButtons();
  });
  window.addEventListener("keyup", (event) => {
    if (!pressed.delete(event.code)) return;
    event.preventDefault();
    updateButtons();
  });
  canvas.addEventListener("blur", clearKeys);
  for (const button of touchButtons) {
    button.addEventListener("pointerdown", (event) => {
      if (!running || help.open || (event.pointerType === "mouse" && event.button !== 0)) return;
      event.preventDefault();
      button.setPointerCapture(event.pointerId);
      pointers.set(event.pointerId, button);
      updateButtons();
    });
    const release = (event: PointerEvent) => {
      if (pointers.delete(event.pointerId)) updateButtons();
    };
    button.addEventListener("pointerup", release);
    button.addEventListener("pointercancel", release);
    button.addEventListener("lostpointercapture", release);
    // Also make the on-screen controls usable with keyboard activation.
    button.addEventListener("keydown", (event) => {
      if (running && (event.code === "Space" || event.code === "Enter")) {
        event.preventDefault();
        pointers.set(-1, button);
        updateButtons();
      }
    });
    button.addEventListener("keyup", (event) => {
      if (event.code === "Space" || event.code === "Enter") {
        event.preventDefault();
        pointers.delete(-1);
        updateButtons();
      }
    });
    button.addEventListener("blur", clearKeys);
  }
  window.addEventListener("blur", clearKeys);
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) {
      visibilityRevision++;
      resumeAfterDialog = false;
      setRunning(false);
    }
  });
  draw();
  toggleDebug(false);
  for (const control of [modelSelect, demoButton, loadButton, settingsButton, fullscreenButton]) control.disabled = false;
  refresh();
}

function initialize() {
  void main().catch((error: unknown) => {
    romStatus.textContent = "Emulator unavailable";
    errorStatus.textContent = `Could not initialize the player: ${message(error)} Retry initialization or reload the page.`;
    retryButton.hidden = false;
  });
}
retryButton.addEventListener("click", initialize);
initialize();
