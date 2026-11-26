import { Saves, romIdentity, consoleKey, RtcRecords, rtcKey, orientationKey } from "./saves";
import { Sound } from "./audio";
import { Display, stateThumbnail, type DisplayEffect } from "./display";
import { isRomIdentity, maxRomBytes, readCartridge, type CartridgeEntry } from "./cartridge-library";
import { cartridgePicker } from "./cartridge-picker";
import { statePicker } from "./state-picker";
import init, { max_snapshot_bytes, WebMachine } from "../pkg/ws_web.js";

const element = <T extends HTMLElement>(id: string) => document.querySelector<T>(`#${id}`)!;
const modelSelect = element<HTMLSelectElement>("model");
const resetButton = element<HTMLButtonElement>("reset");
const pauseButton = element<HTMLButtonElement>("pause");
const loadButton = element<HTMLButtonElement>("load-rom");
const romInput = element<HTMLInputElement>("rom");
const romStatus = element<HTMLParagraphElement>("rom-status");
const errorStatus = element<HTMLParagraphElement>("error");
const canvas = element<HTMLCanvasElement>("screen");
const player = element<HTMLElement>("player");
const toolbar = element("controls");
const game = element("game");
const screenFrame = element("display");
const orientationSelect = element<HTMLSelectElement>("orientation");
const help = element<HTMLDialogElement>("help");
const settingsButton = element<HTMLButtonElement>("settings");
const debugToggle = element<HTMLInputElement>("debug-toggle");
const debugPanel = element<HTMLElement>("debug-panel");
const fullscreenButton = element<HTMLButtonElement>("fullscreen");
const exitFullscreenButton = element<HTMLButtonElement>("exit-fullscreen");
const retryButton = element<HTMLButtonElement>("retry");
const touchButtons = [...document.querySelectorAll<HTMLButtonElement>("[data-button]")];

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

async function main() {
  retryButton.hidden = true;
  romStatus.textContent = "Loading WebAssembly…";
  romStatus.classList.remove("visually-hidden");
  errorStatus.textContent = "";
  await init();
  const machine = new WebMachine();
  canvas.width = machine.frame_width();
  canvas.height = machine.frame_height();
  const display = new Display(canvas, element<HTMLCanvasElement>("screen-effect"),
    message => { element("display-notice").textContent = message; });
  const effectSelect = element<HTMLSelectElement>("display-effect");
  let loaded: { name: string; key: string } | null = null;
  let orientation: "horizontal" | "vertical" = "horizontal";
  effectSelect.value = display.effect;
  effectSelect.addEventListener("change", () => display.select(effectSelect.value as DisplayEffect));
  effectSelect.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && !event.ctrlKey && !event.metaKey && !event.altKey && !event.shiftKey && loaded) {
      canvas.focus();
    }
  });
  const report = (message: string) => { element("notice").textContent = message; };
  const saves = new Saves(report);
  const rtcMode = element<HTMLSelectElement>("rtc-mode");
  const clocks = new RtcRecords(message => { element("rtc-notice").textContent = message; });
  function syncClock() { if (loaded) machine.sync_rtc(Date.now()); }
  function restoreClock() {
    if (!loaded || !machine.rtc_data().length) return;
    clocks.restore(rtcKey(loaded.key), data => machine.load_rtc_data(data));
    syncClock();
  }
  const consoles = new Saves(message => { element("console-notice").textContent = `${machine.model_name()}: ${message}`; }, true);
  const restoredConsoles = new Set<string>();
  function restoreConsole() {
    const model = machine.model();
    if (restoredConsoles.has(model)) return;
    const restored = consoles.restore(consoleKey(model), machine.console_data().length);
    if (restored) machine.load_console_data(restored);
    restoredConsoles.add(model);
  }
  restoreConsole();
  const soundButton = element<HTMLButtonElement>("sound");
  const volume = element<HTMLInputElement>("volume");
  const enableAudio = element<HTMLButtonElement>("enable-audio");
  const sound = new Sound(refreshSound);
  function refreshSound() {
    soundButton.setAttribute("aria-pressed", String(sound.muted));
    volume.value = String(sound.volume);
    element("volume-value").textContent = `${sound.volume}%`;
    const status = element("sound-status");
    status.textContent = sound.status;
    status.classList.toggle("visually-hidden", !sound.hasWarning);
    enableAudio.hidden = !sound.needsUnlock;
  }
  function persist() {
    syncClock();
    if (loaded) {
      saves.store(loaded.key, machine.save_data());
      clocks.store(rtcKey(loaded.key), machine.rtc_data());
    }
    consoles.store(consoleKey(machine.model()), machine.console_data());
  }
  let readRevision = 0;
  let loading = false;
  let wantsPlayback = false;
  let running = false;
  let faulted = false;
  let pageAway = false;
  let visibilityRevision = 0;
  let animation: number | null = null;
  let production: number | null = null;
  let lastTime = 0;
  let elapsed = 0;
  let dirty = false;
  let framePending = false;
  const pressed = new Set<string>();
  const pointers = new Map<number, HTMLButtonElement>();
  const activated = new Map<string, HTMLButtonElement>();
  const bindings: Record<string, number> = {
    ArrowUp: 16, KeyW: 16, ArrowRight: 32, KeyD: 32,
    ArrowDown: 64, KeyS: 64, ArrowLeft: 128, KeyA: 128,
    KeyI: 256, KeyL: 512, KeyK: 1024, KeyJ: 2048,
    Enter: 2, KeyZ: 4, KeyX: 8,
  };

  function updateButtons() {
    let mask = [...pressed].reduce((mask, code) => mask | bindings[code], 0);
    for (const button of pointers.values()) mask |= Number(button.dataset.button);
    for (const button of activated.values()) mask |= Number(button.dataset.button);
    machine.set_buttons(mask);
    for (const button of touchButtons) button.classList.toggle("is-pressed", (mask & Number(button.dataset.button)) !== 0);
  }
  function clearKeys() {
    pressed.clear();
    for (const [id, button] of pointers) {
      if (button.hasPointerCapture(id)) button.releasePointerCapture(id);
    }
    pointers.clear();
    activated.clear();
    updateButtons();
  }
  function captureFrame() {
    framePending = false;
    const pixels = machine.framebuffer();
    const rgba = pixels.buffer instanceof ArrayBuffer
      ? new Uint8ClampedArray(pixels.buffer, pixels.byteOffset, pixels.byteLength)
      : new Uint8ClampedArray(pixels);
    display.advance(rgba, machine.model());
  }
  function draw() {
    captureFrame();
    display.present();
  }
  function state() {
    return loading ? "Loading ROM…" : !loaded ? "No ROM loaded" : faulted ? "Stopped" : running ? "Running" : "Paused";
  }
  function updateDiagnostics() {
    if (debugPanel.hidden) return;
    element("debug-model").textContent = machine.model_name();
    element("debug-rom").textContent = loaded?.name ?? "None";
    element("debug-bytes").textContent = String(machine.rom_len());
    element("debug-state").textContent = state();
    element("debug-frames").textContent = String(machine.frame_number());
    element("debug-error").textContent = errorStatus.textContent || element("notice").textContent || "None";
  }
  function refresh() {
    orientationSelect.disabled = !loaded || loading || states.busy;
    modelSelect.disabled = states.busy;
    loadButton.disabled = states.busy;
    rtcMode.disabled = states.busy;
    modelSelect.value = machine.model();
    const title = element("rom-title");
    title.textContent = loaded?.name ?? "No ROM loaded";
    title.title = loaded?.name ?? "";
    romStatus.textContent = state();
    romStatus.classList.toggle("visually-hidden", !loading);
    element("empty-state").hidden = loaded !== null;
    element("controls").setAttribute("aria-busy", String(loading));
    pauseButton.disabled = loaded === null || faulted || loading;
    resetButton.disabled = loaded === null || states.busy;
    pauseButton.textContent = running ? "Pause" : loaded ? "Resume" : "Play";
    for (const button of touchButtons) button.disabled = !running;
    states.refresh();
    updateDiagnostics();
    sizeScreen();
  }
  function syncPlayback() {
    const canRun = wantsPlayback && loaded !== null && !loading && !faulted
      && !pageAway && !document.hidden && !help.open;
    if (running === canRun) {
      refresh();
      return;
    }
    if (framePending) captureFrame();
    running = canRun;
    if (animation !== null) cancelAnimationFrame(animation);
    if (production !== null) clearTimeout(production);
    animation = null;
    production = null;
    lastTime = performance.now();
    elapsed = 0;
    dirty = false;
    clearKeys();
    if (running) {
      display.resetHistory();
      sound.resume();
      production = window.setTimeout(tick, 0);
      animation = requestAnimationFrame(present);
    }
    else { sound.pause(); persist(); }
    refresh();
  }
  function failed(error: unknown) {
    framePending = false;
    faulted = true;
    wantsPlayback = false;
    errorStatus.textContent = `Execution stopped: ${message(error)} Try Reset or load another ROM.`;
    syncPlayback();
  }
  function present() {
    animation = null;
    if (!running) return;
    try {
      if (dirty) {
        if (framePending) captureFrame();
        display.present();
        dirty = false;
      }
      animation = requestAnimationFrame(present);
    } catch (error) { failed(error); }
  }
  function tick() {
    production = null;
    if (!running) return;
    const time = performance.now();
    // Presentation cadence must not determine how much PCM is produced. Bound
    // both backlog and work per turn after stalls; a suspended page is paused.
    elapsed = Math.min(elapsed + time - lastTime, sound.catchUpMs);
    lastTime = time;
    syncClock();
    try {
      let frames = 0;
      while (frames < 8 && performance.now() - time < 8) {
        const frameDuration = 1000 / machine.frame_rate();
        if (elapsed < frameDuration) break;
        machine.run_frame();
        sound.push(machine.audio_samples(), machine.sample_rate());
        // LCD mixes each emulated frame into its history. Clean and CRT only
        // need the latest frame when the browser is about to present it.
        if (display.tracksHistory) captureFrame();
        else framePending = true;
        elapsed -= frameDuration;
        frames++;
      }
      dirty ||= frames > 0;
      production = window.setTimeout(tick, Math.max(0, Math.min(8,
        1000 / machine.frame_rate() - elapsed - (performance.now() - time))));
    } catch (error) { failed(error); }
  }
  function focusGame() {
    canvas.focus({ preventScroll: true });
    const bounds = game.getBoundingClientRect();
    const toolbarTop = toolbar.getBoundingClientRect().top;
    const scrollMargins = getComputedStyle(game);
    const safeTop = parseFloat(scrollMargins.scrollMarginTop);
    const height = window.innerHeight - safeTop - parseFloat(scrollMargins.scrollMarginBottom);
    // Match scrollIntoView's safe-area margins when deciding what fits together.
    const revealToolbar = toolbarTop < safeTop && bounds.bottom - toolbarTop <= height;
    const target = bounds.height <= height ? game : screenFrame;
    target.scrollIntoView({ block: revealToolbar ? "end" : "nearest", inline: "nearest" });
  }
  function applyOrientation() {
    const preference = orientationSelect.value;
    const next = preference === "horizontal" || preference === "vertical"
      ? preference : machine.orientation() === "vertical" ? "vertical" : "horizontal";
    if (orientation === next) return;
    clearKeys();
    orientation = next;
    game.dataset.orientation = orientation;
  }
  function load(name: string, bytes: Uint8Array, key: string) {
    persist();
    machine.load_rom(bytes); // Validation errors preserve the previous machine.
    display.resetHistory();
    machine.configure_rtc(rtcMode.value);
    loaded = { name, key };
    let savedOrientation: string | null = null;
    try { savedOrientation = localStorage.getItem(orientationKey(key)); } catch { /* Use Auto. */ }
    orientationSelect.value = savedOrientation === "horizontal" || savedOrientation === "vertical"
      ? savedOrientation : "auto";
    applyOrientation();
    element("rtc-notice").textContent = "";
    restoreClock();
    element("notice").textContent = "";
    const restored = saves.restore(key, machine.save_data().length);
    if (restored) machine.load_save(restored);
    faulted = false;
    errorStatus.textContent = "";
    draw();
    wantsPlayback = true;
  }
  function restart(model: string) {
    if (states.busy) return;
    readRevision++; // Invalidate any in-flight file read.
    loading = false;
    wantsPlayback = false;
    if (!running) persist();
    syncPlayback();
    errorStatus.textContent = "";
    try {
      machine.change_model(model);
      applyOrientation();
      display.resetHistory();
      draw();
      restoreConsole();
      faulted = false;
      wantsPlayback = loaded !== null;
      syncPlayback();
      if (running && !picker.open) focusGame();
    } catch (error) {
      errorStatus.textContent = `Could not reset: ${message(error)} Choose a compatible model or load another ROM.`;
    }
    display.present();
    refresh();
  }
  async function readROM(name: string, read: () => Promise<Uint8Array>, entry?: CartridgeEntry) {
    if (states.busy) return;
    const revision = ++readRevision;
    const visibleRevision = visibilityRevision;
    const sourceFocus = document.activeElement;
    let verified = !entry;
    clearKeys();
    loading = true;
    if (!faulted) errorStatus.textContent = "";
    syncPlayback();
    try {
      const bytes = await read();
      if (revision !== readRevision) return;
      const key = await romIdentity(bytes);
      if (revision !== readRevision) return;
      if (entry && key !== entry.key) throw new Error("Saved ROM identity mismatch.");
      verified = true;
      // An identical cartridge keeps its frame, playback state, and inserted name.
      if (key !== loaded?.key) load(name, bytes, key);
      if (entry) picker.clearProblem(entry.key);
      else void picker.remember(key, name, bytes);
      if (visibleRevision === visibilityRevision && canFocusGame() && document.activeElement === sourceFocus) focusGame();
    } catch (error) {
      if (revision === readRevision) {
        if (entry && !verified) picker.readFailed(entry.key);
        else {
          errorStatus.textContent = `Could not load “${name}”: ${message(error)} Choose another ROM.`;
          if (visibleRevision === visibilityRevision && canFocusGame() && document.activeElement === loadButton) {
            loadButton.scrollIntoView({ block: "nearest", inline: "nearest" });
          }
        }
      }
    } finally {
      if (revision === readRevision) {
        loading = false;
        syncPlayback();
      }
    }
  }
  soundButton.addEventListener("click", () => {
    sound.setMuted(!sound.muted);
    if (!sound.muted) sound.unlock();
  });
  enableAudio.addEventListener("click", () => sound.unlock());
  volume.addEventListener("input", () => {
    sound.setVolume(Number(volume.value));
    // Raising the level does not override an explicit mute preference.
    if (sound.volume > 0 && sound.needsUnlock) sound.unlock();
  });
  rtcMode.addEventListener("change", () => {
    if (!loaded || states.busy) return;
    persist();
    machine.configure_rtc(rtcMode.value);
    restoreClock();
  });
  orientationSelect.addEventListener("change", () => {
    if (!loaded || loading || states.busy) return;
    const key = orientationKey(loaded.key);
    try {
      if (orientationSelect.value === "auto") localStorage.removeItem(key);
      else localStorage.setItem(key, orientationSelect.value);
    } catch { /* Keep the live choice. */ }
    applyOrientation();
    if (framePending) captureFrame();
    sizeScreen();
  });
  window.setInterval(() => { if (!pageAway && !document.hidden) persist(); }, 1000);
  function suspendForDeparture() {
    visibilityRevision++;
    if (!running) {
      clearKeys();
      persist();
    }
    syncPlayback();
  }
  window.addEventListener("pagehide", event => {
    pageAway = true;
    if (!event.persisted) {
      readRevision++;
      loading = false;
      wantsPlayback = false;
    }
    suspendForDeparture();
  });
  window.addEventListener("pageshow", event => {
    if (!event.persisted || !pageAway) return;
    pageAway = false;
    syncPlayback();
    restoreWindowedView();
  });
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) suspendForDeparture();
    else {
      syncPlayback();
      restoreWindowedView();
    }
  });
  modelSelect.addEventListener("change", () => restart(modelSelect.value));
  resetButton.addEventListener("click", () => restart(machine.model()));
  pauseButton.addEventListener("click", () => {
    wantsPlayback = !wantsPlayback;
    if (wantsPlayback) sound.unlock();
    syncPlayback();
    if (running) canvas.focus({ preventScroll: true });
  });
  const picker = cartridgePicker(loadButton, () => {
    clearKeys();
    if (loading) {
      readRevision++;
      loading = false;
      syncPlayback();
    }
  }, () => {
    sound.unlock();
    romInput.click();
  }, entry => {
    if (isRomIdentity(entry.key)) sound.unlock();
    void readROM(entry.name, () => readCartridge(entry.key), entry);
  });
  const states = statePicker({
    scope: () => loaded && !loading ? `${loaded.key}:${machine.model()}` : null,
    canSave: () => !faulted,
    onOpen: clearKeys,
    onBusy: refresh,
    maxBytes: max_snapshot_bytes(),
    async capture() {
      syncClock();
      const timestamp = Date.now();
      const bytes = machine.capture_snapshot();
      const thumbnail = await stateThumbnail(machine.framebuffer(), orientation === "vertical");
      return { bytes, thumbnail, timestamp };
    },
    async inspect(bytes) {
      const candidate = machine.prepare_snapshot(bytes);
      try {
        return await stateThumbnail(candidate.framebuffer(), orientation === "vertical");
      } finally {
        candidate.free();
      }
    },
    restore(bytes) {
      // Prepare and apply in the same turn, before updating the player or scheduler.
      const candidate = machine.prepare_snapshot(bytes);
      machine.apply_snapshot(candidate, Date.now());
      clearKeys();
      sound.flush();
      display.resetHistory();
      faulted = false;
      errorStatus.textContent = "";
      dirty = false;
      draw();
      if (production !== null) clearTimeout(production);
      production = null;
      lastTime = performance.now();
      elapsed = 0;
      syncPlayback();
      if (running && production === null) production = window.setTimeout(tick, 0);
      if (running && canFocusGame()) canvas.focus({ preventScroll: true });
    },
  });
  romInput.addEventListener("change", () => {
    const file = romInput.files?.[0];
    romInput.value = "";
    if (!file) return;
    void readROM(file.name, async () => {
      if (file.size > maxRomBytes) throw new Error("Supported ROM size is at most 64 MiB (mapper dependent).");
      return new Uint8Array(await file.arrayBuffer());
    });
  });

  function canFocusGame() {
    return !pageAway && !document.hidden && !help.open && !picker.open && !states.open;
  }
  settingsButton.addEventListener("click", () => {
    help.showModal();
    syncPlayback();
  });
  element("help-close").addEventListener("click", () => help.close());
  help.addEventListener("close", () => {
    syncPlayback();
    if (canFocusGame()) (running ? canvas : settingsButton).focus({ preventScroll: true });
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

  let windowedScroll: ScrollToOptions | undefined;
  let fullscreenExitPending = false;
  function restoreWindowedView() {
    if (!fullscreenExitPending || pageAway || document.hidden) return;
    fullscreenExitPending = false;
    if (canFocusGame()) fullscreenButton.focus({ preventScroll: true });
    if (windowedScroll) window.scrollTo(windowedScroll);
    windowedScroll = undefined;
  }
  fullscreenButton.addEventListener("click", async () => {
    clearKeys();
    const notice = element("fullscreen-status");
    notice.textContent = "";
    if (!document.fullscreenEnabled || !game.requestFullscreen) {
      notice.textContent = "Fullscreen is unavailable.";
      return;
    }
    windowedScroll = { left: window.scrollX, top: window.scrollY };
    try {
      await game.requestFullscreen();
    } catch {
      windowedScroll = undefined;
      notice.textContent = "Could not enter fullscreen.";
    }
  });
  function exitFullscreen() {
    if (document.fullscreenElement === game) {
      void document.exitFullscreen().catch(() => {});
    }
  }
  exitFullscreenButton.addEventListener("click", exitFullscreen);
  const playerChrome = [...player.querySelectorAll<HTMLElement>(":scope > :not(#game)"), debugPanel];
  let gameplayFullscreen = false;
  document.addEventListener("fullscreenchange", () => {
    const active = document.fullscreenElement === game;
    if (active === gameplayFullscreen) return;
    gameplayFullscreen = active;
    const modalOpen = help.open || picker.open || states.open;
    // A dialog opened during the request takes precedence and must remain usable.
    if (active && modalOpen) exitFullscreen();
    for (const item of playerChrome) item.inert = active && !modalOpen;
    clearKeys();
    if (framePending) captureFrame();
    sizeScreen();
    if (active) {
      if (canFocusGame()) canvas.focus({ preventScroll: true });
    } else {
      fullscreenExitPending = true;
      restoreWindowedView();
    }
  });
  // Integer scaling where space allows; fractional nearest-neighbor scaling on narrow screens.
  function sizeScreen() {
    const vertical = orientation === "vertical";
    const width = vertical ? 144 : 224;
    const height = vertical ? 224 : 144;
    const bounds = screenFrame.getBoundingClientRect();
    const left = Math.ceil(bounds.left);
    const top = Math.ceil(bounds.top);
    const spaceWidth = Math.max(0, Math.floor(bounds.right) - left);
    const spaceHeight = Math.max(0, Math.floor(bounds.bottom) - top);
    const available = Math.min(spaceWidth / width, spaceHeight / height, 4);
    // 224 and 144 share a factor of 16. Fractional steps of 1/16 keep both
    // edges on whole CSS pixels, avoiding composited blur at rotated boundaries.
    const scale = available >= 2 ? Math.floor(available) : Math.floor(available * 16) / 16;
    const presentation = element("screen-presentation");
    presentation.style.width = `${width * scale}px`;
    presentation.style.height = `${height * scale}px`;
    presentation.style.left = `${left - bounds.left + Math.floor((spaceWidth - width * scale) / 2)}px`;
    presentation.style.top = `${top - bounds.top + Math.floor((spaceHeight - height * scale) / 2)}px`;
    display.resize(scale);
  }
  new ResizeObserver(sizeScreen).observe(screenFrame);
  window.addEventListener("resize", clearKeys);

  window.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && gameplayFullscreen && !help.open && !picker.open && !states.open) {
      exitFullscreen();
      return;
    }
    if (!running || help.open || picker.open || states.open || document.activeElement !== canvas || !(event.code in bindings)
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
      if (!running || help.open || states.open || (event.pointerType === "mouse" && event.button !== 0)) return;
      event.preventDefault();
      canvas.focus({ preventScroll: true });
      button.setPointerCapture(event.pointerId);
      pointers.set(event.pointerId, button);
      updateButtons();
    });
    const release = (event: PointerEvent) => {
      if (pointers.get(event.pointerId) === button) {
        pointers.delete(event.pointerId);
        updateButtons();
      }
    };
    button.addEventListener("pointerup", release);
    button.addEventListener("pointercancel", release);
    button.addEventListener("lostpointercapture", release);
    // Also make the on-screen controls usable with keyboard activation.
    button.addEventListener("keydown", (event) => {
      if (running && !help.open && !states.open && (event.code === "Space" || event.code === "Enter")) {
        event.preventDefault();
        activated.set(event.code, button);
        updateButtons();
      }
    });
    button.addEventListener("keyup", (event) => {
      if (event.code === "Space" || event.code === "Enter") {
        event.preventDefault();
        activated.delete(event.code);
        updateButtons();
      }
    });
    button.addEventListener("blur", clearKeys);
  }
  window.addEventListener("blur", clearKeys);
  draw();
  toggleDebug(false);
  for (const control of [modelSelect, effectSelect, loadButton, settingsButton, fullscreenButton, soundButton, volume, enableAudio]) control.disabled = false;
  refreshSound();
  refresh();
}

function initialize() {
  void main().catch((error: unknown) => {
    romStatus.textContent = "Emulator unavailable";
    romStatus.classList.add("visually-hidden");
    errorStatus.textContent = `Could not initialize the player: ${message(error)} Retry initialization or reload the page.`;
    retryButton.hidden = false;
  });
}
retryButton.addEventListener("click", initialize);
initialize();
