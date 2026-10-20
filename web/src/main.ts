import init, { WebMachine } from "../pkg/ws_web.js";

const controls = document.querySelector<HTMLFieldSetElement>("#controls")!;
const modelSelect = document.querySelector<HTMLSelectElement>("#model")!;
const resetButton = document.querySelector<HTMLButtonElement>("#reset")!;
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
  let loadedFilename: string | null = null;
  let readRevision = 0;

  function refresh() {
    modelSelect.value = machine.model();
    activeModel.textContent = `Active model (Rust core): ${machine.model_name()}`;
    romStatus.textContent = loadedFilename === null
      ? `No ROM loaded. Core stores ${machine.rom_len()} bytes.`
      : `Loaded “${loadedFilename}”: ${machine.rom_len()} bytes accepted by the core. Games cannot run yet.`;
    const pixels = new Uint8ClampedArray(machine.framebuffer());
    context!.putImageData(new ImageData(pixels, canvas.width, canvas.height), 0, 0);
  }

  function reset(model: string) {
    machine.reset(model);
    // Invalidate pending file reads so they cannot reload a reset machine.
    readRevision++;
    loadedFilename = null;
    romInput.value = "";
    errorStatus.textContent = "";
    refresh();
  }

  modelSelect.addEventListener("change", () => reset(modelSelect.value));
  resetButton.addEventListener("click", () => reset(machine.model()));
  romInput.addEventListener("change", async () => {
    const file = romInput.files?.[0];
    if (!file) return;
    const revision = ++readRevision;
    errorStatus.textContent = "";
    try {
      const bytes = new Uint8Array(await file.arrayBuffer());
      if (revision !== readRevision) return;
      machine.load_rom(bytes);
      loadedFilename = file.name;
      refresh();
    } catch (error) {
      if (revision === readRevision) {
        errorStatus.textContent = `Could not load “${file.name}”: ${message(error)}`;
      }
    } finally {
      // Permit choosing the same file again, including after an error.
      if (revision === readRevision) romInput.value = "";
    }
  });

  refresh();
  controls.disabled = false;
}

main().catch((error: unknown) => {
  activeModel.textContent = "Prototype unavailable.";
  errorStatus.textContent = `Could not initialize the prototype: ${message(error)}`;
});
