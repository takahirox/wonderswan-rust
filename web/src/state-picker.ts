import { deleteState, readState, statePage, writeState, type StateEntry } from "./state-history";

const element = <T extends HTMLElement>(id: string) => document.querySelector<T>(`#${id}`)!;
const formatTime = new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "medium" });
const label = (entry: StateEntry) =>
  [entry.label, formatTime.format(entry.timestamp)].filter(Boolean).join(" · ");

interface StateControls {
  scope: () => string | null;
  canSave: () => boolean;
  onBusy: () => void;
  onOpen: () => void;
  capture: () => Promise<{ bytes: Uint8Array; thumbnail: Blob; timestamp: number }>;
  inspect: (bytes: Uint8Array) => Promise<Blob>;
  restore: (bytes: Uint8Array) => void;
  maxBytes: number;
}

export function statePicker(controls: StateControls) {
  const trigger = element<HTMLButtonElement>("select-state");
  const save = element<HTMLButtonElement>("save-state");
  const load = element<HTMLButtonElement>("load-state");
  const dialog = element<HTMLDialogElement>("state-picker");
  const close = element<HTMLButtonElement>("state-close");
  const list = element<HTMLUListElement>("state-list");
  const fresh = element<HTMLButtonElement>("state-new");
  const older = element<HTMLButtonElement>("state-older");
  const newer = element<HTMLButtonElement>("state-newer");
  const importButton = element<HTMLButtonElement>("state-import");
  const fileInput = element<HTMLInputElement>("state-file");
  const status = element("state-status");
  const dialogStatus = element("state-picker-status");
  const selections = new Map<string, StateEntry>();
  let busy = false;
  let clearStatus: number | undefined;
  let pageKeys: IDBValidKey[] = [];
  let nextPage: IDBValidKey | undefined;
  let deletePrompt: HTMLElement | undefined;
  const thumbnails: string[] = [];

  function refresh() {
    const scope = controls.scope();
    const selected = scope ? selections.get(scope) : undefined;
    const title = selected ? label(selected) : "New state";
    element("state-selection").textContent = title;
    trigger.title = `Save state: ${title}`;
    trigger.disabled = busy || !scope;
    save.disabled = busy || !scope || !controls.canSave();
    load.disabled = busy || !selected;
    fresh.setAttribute("aria-pressed", String(!selected));
    dialog.setAttribute("aria-busy", String(busy));
    for (const button of dialog.querySelectorAll("button")) {
      if (button !== close) button.setAttribute("aria-disabled", String(busy));
    }
  }

  function feedback(text: string, temporary = false) {
    window.clearTimeout(clearStatus);
    status.textContent = dialog.open ? "" : text;
    dialogStatus.textContent = dialog.open ? text : "";
    if (temporary) clearStatus = window.setTimeout(() => {
      status.textContent = "";
      dialogStatus.textContent = "";
    }, 2500);
  }

  async function run(action: (scope: string) => Promise<void>) {
    const scope = controls.scope();
    if (busy || !scope) return;
    const focus = document.activeElement;
    busy = true;
    controls.onBusy();
    feedback("");
    try {
      await action(scope);
    } catch (error) {
      feedback(error instanceof Error ? error.message : String(error));
    } finally {
      busy = false;
      controls.onBusy();
      if (document.activeElement === document.body && !document.hidden) {
        if (dialog.open) close.focus({ preventScroll: true });
        else if (focus instanceof HTMLElement && !dialog.contains(focus)) focus.focus({ preventScroll: true });
        else trigger.focus({ preventScroll: true });
      }
    }
  }

  function choose(entry?: StateEntry) {
    const scope = controls.scope();
    if (busy || !scope) return;
    if (entry) selections.set(scope, entry);
    else selections.delete(scope);
    refresh();
    dialog.close();
  }

  function clearList() {
    list.replaceChildren();
    deletePrompt = undefined;
    for (const url of thumbnails) URL.revokeObjectURL(url);
    thumbnails.length = 0;
  }

  function button(text: string, action: () => void) {
    const result = document.createElement("button");
    result.type = "button";
    result.textContent = text;
    result.addEventListener("click", action);
    return result;
  }

  async function showPage(scope: string, keys: IDBValidKey[]) {
    const page = await statePage(scope, keys.at(-1));
    if (!dialog.open) return;
    if (!page.entries.length && keys.length) {
      await showPage(scope, keys.slice(0, -1));
      return;
    }
    const listHadFocus = list.contains(document.activeElement);
    clearList();
    const selected = selections.get(scope)?.id;
    for (const entry of page.entries) {
      const row = document.createElement("li");
      const name = label(entry);
      const choice = button(name, () => choose(entry));
      choice.className = "state-choice";
      choice.setAttribute("aria-pressed", String(selected === entry.id));
      const image = document.createElement("img");
      const url = URL.createObjectURL(entry.thumbnail);
      thumbnails.push(url);
      image.src = url;
      image.alt = "";
      const title = document.createElement("span");
      title.textContent = name;
      choice.replaceChildren(image, title);
      const download = button("Export", () => void run(async () => {
        const bytes = await readState(scope, entry.id);
        const file = URL.createObjectURL(new Blob([bytes.slice().buffer], { type: "application/octet-stream" }));
        const link = document.createElement("a");
        link.href = file;
        link.download = `wonderswan-${entry.timestamp}-${entry.id.slice(0, 8)}.wsstate`;
        link.click();
        window.setTimeout(() => URL.revokeObjectURL(file), 1000);
      }));
      const remove = button("Delete", () => {
        if (busy) return;
        deletePrompt?.remove();
        const prompt = document.createElement("div");
        prompt.className = "state-delete-confirmation";
        prompt.setAttribute("role", "group");
        prompt.setAttribute("aria-label", `Delete the save state “${name}”?`);
        const cancel = button("Cancel", () => {
          if (busy) return;
          prompt.remove();
          deletePrompt = undefined;
          remove.focus();
        });
        const confirm = button("Confirm delete", () => void run(async () => {
          await deleteState(entry);
          if (selected === entry.id) selections.delete(scope);
          await showPage(scope, keys);
          feedback("Save state deleted.", true);
        }));
        prompt.append(cancel, confirm);
        row.append(prompt);
        deletePrompt = prompt;
        cancel.focus();
      });
      download.setAttribute("aria-label", `Export ${name}`);
      remove.setAttribute("aria-label", `Delete ${name}`);
      row.append(choice, download, remove);
      list.append(row);
    }
    pageKeys = keys;
    nextPage = page.before;
    older.hidden = !nextPage;
    newer.hidden = !pageKeys.length;
    if (listHadFocus || (document.activeElement === older && older.hidden)
      || (document.activeElement === newer && newer.hidden)) fresh.focus();
  }

  trigger.addEventListener("click", () => {
    if (busy || !controls.scope()) return;
    controls.onOpen();
    trigger.focus({ preventScroll: true });
    dialog.showModal();
    void run(scope => showPage(scope, []));
  });
  close.addEventListener("click", () => dialog.close());
  dialog.addEventListener("close", () => {
    if (dialog.open) return;
    clearList();
    older.hidden = true;
    newer.hidden = true;
    if (!busy && !document.hidden) trigger.focus({ preventScroll: true });
  });
  let outside = false;
  function backdrop(event: MouseEvent) {
    const bounds = dialog.getBoundingClientRect();
    return event.target === dialog && (event.clientX < bounds.left || event.clientX >= bounds.right
      || event.clientY < bounds.top || event.clientY >= bounds.bottom);
  }
  dialog.addEventListener("pointerdown", event => { outside = backdrop(event); });
  dialog.addEventListener("click", event => {
    if (outside && backdrop(event)) dialog.close();
    outside = false;
  });
  fresh.addEventListener("click", () => choose());
  older.addEventListener("click", () => {
    if (nextPage) void run(scope => showPage(scope, [...pageKeys, nextPage!]));
  });
  newer.addEventListener("click", () => void run(scope => showPage(scope, pageKeys.slice(0, -1))));
  save.addEventListener("click", () => void run(async scope => {
    const { bytes, timestamp, thumbnail } = await controls.capture();
    const entry = { scope, id: selections.get(scope)?.id ?? crypto.randomUUID(), timestamp, thumbnail };
    await writeState(entry, bytes);
    selections.set(scope, entry);
    feedback("Save state saved.", true);
  }));
  load.addEventListener("click", () => void run(async scope => {
    const entry = selections.get(scope);
    if (!entry) return;
    const bytes = await readState(scope, entry.id);
    controls.restore(bytes);
    feedback("Save state loaded.", true);
  }));
  importButton.addEventListener("click", () => { if (!busy) fileInput.click(); });
  fileInput.addEventListener("change", () => {
    const file = fileInput.files?.[0];
    fileInput.value = "";
    if (!file || !dialog.open) return;
    void run(async scope => {
      if (file.size > controls.maxBytes) throw new Error("Save-state file exceeds the format size limit.");
      const bytes = new Uint8Array(await file.arrayBuffer());
      const thumbnail = await controls.inspect(bytes);
      const entry = { scope, id: crypto.randomUUID(), timestamp: Date.now(), thumbnail, label: "Imported state" };
      await writeState(entry, bytes);
      await showPage(scope, []);
      feedback("Save state imported.", true);
    });
  });

  return {
    get open() { return dialog.open; },
    get busy() { return busy; },
    refresh,
  };
}
