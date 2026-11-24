import {
  type CartridgeEntry, isRomIdentity, listCartridges, rememberCartridge, removeCartridge,
} from "./cartridge-library";

const element = <T extends HTMLElement>(id: string) => document.querySelector<T>(`#${id}`)!;
const sameKey = (a: IDBValidKey, b: IDBValidKey) => a === b || indexedDB.cmp(a, b) === 0;

export function cartridgePicker(
  trigger: HTMLButtonElement,
  onOpen: () => void,
  openLocal: () => void,
  select: (entry: CartridgeEntry) => void,
) {
  const dialog = element<HTMLDialogElement>("cartridge-picker");
  const close = element<HTMLButtonElement>("cartridge-close");
  const local = element<HTMLButtonElement>("open-local-rom");
  const list = element<HTMLUListElement>("cartridge-list");
  const section = element("my-cartridges");
  const listStatus = element("library-list-status");
  const operation = element("library-operation");
  const retention = element("library-retention");
  const notice = element("cartridge-notice");
  const alert = element("cartridge-notice-message");
  const dismiss = element("cartridge-notice-dismiss");
  const failures = new Map<string, string>();
  let problem: IDBValidKey | undefined;
  let noticeSource: "operation" | "retention" | null = null;
  let entries: CartridgeEntry[] = [];
  let listRevision = 0;
  let removing: IDBValidKey | undefined;
  let lastRemoval: { button: HTMLButtonElement; bounds: DOMRect; time: number } | undefined;

  function updateNotice() {
    const text = noticeSource === "operation" ? operation.textContent
      : noticeSource === "retention" ? retention.textContent : "";
    notice.hidden = !text;
    alert.textContent = text;
  }

  function reportProblem(key: IDBValidKey, text: string, onScreen = false) {
    problem = key;
    operation.textContent = text;
    if (onScreen) noticeSource = "operation";
    else if (noticeSource === "operation") noticeSource = null;
    updateNotice();
  }

  function clearProblem(key: IDBValidKey) {
    if (problem !== undefined && sameKey(key, problem)) {
      problem = undefined;
      if (noticeSource === "operation") noticeSource = null;
      operation.textContent = "";
      updateNotice();
    }
  }

  function updateRetention() {
    const name = [...failures.values()].at(-1);
    retention.textContent = name
      ? `Could not remember ${name}. Allow or free browser storage, then reopen the original file to retry.`
      : "";
    updateNotice();
  }

  function render(next: CartridgeEntry[]) {
    if (next.length === entries.length && next.every((entry, i) =>
      entry.name === entries[i].name && sameKey(entry.key, entries[i].key))) return;

    const focused = list.contains(document.activeElement);
    const counts = new Map<string, number>();
    for (const { name } of next) counts.set(name, (counts.get(name) ?? 0) + 1);
    const rows = next.map((entry, index) => {
      const { key, name } = entry;
      const row = document.createElement("li");
      const choose = document.createElement("button");
      choose.type = "button";
      choose.className = "cartridge-choice";
      choose.title = name;
      const label = document.createElement("span");
      label.textContent = name;
      choose.append(label);
      choose.addEventListener("click", () => {
        if (removing !== undefined && sameKey(removing, key)) return;
        dialog.close();
        select(entry);
      });
      const remove = document.createElement("button");
      remove.type = "button";
      remove.textContent = "Remove";
      remove.setAttribute("aria-label", `Remove ${name}`);
      if (removing !== undefined && sameKey(removing, key)) remove.setAttribute("aria-disabled", "true");
      remove.addEventListener("click", event => {
        if (event.detail) lastRemoval = { button: remove, bounds: remove.getBoundingClientRect(), time: event.timeStamp };
        void removeEntry(entry, remove);
      });
      if (counts.get(name)! > 1) {
        const suffix = document.createElement("small");
        suffix.id = `cartridge-identity-${index}`;
        suffix.textContent = isRomIdentity(key) ? key.slice(-64, -52) : "Invalid saved entry";
        choose.setAttribute("aria-label", name);
        for (const button of [choose, remove]) button.setAttribute("aria-describedby", suffix.id);
        choose.append(suffix);
      }
      row.append(choose, remove);
      return row;
    });
    entries = next;
    list.replaceChildren(...rows);
    section.hidden = !next.length;
    if (focused) close.focus({ preventScroll: true });
  }

  async function refreshList() {
    const revision = ++listRevision;
    try {
      const next = await listCartridges();
      if (revision !== listRevision || !dialog.open) return;
      next.sort((a, b) => a.name.localeCompare(b.name) || indexedDB.cmp(a.key, b.key));
      render(next);
      listStatus.textContent = "";
    } catch {
      if (revision === listRevision && dialog.open) {
        listStatus.textContent = "Could not read My cartridges. Reopen the picker to retry or open a local ROM.";
      }
    }
  }

  async function removeEntry({ key, name }: CartridgeEntry, button: HTMLButtonElement) {
    if (removing !== undefined) return;
    removing = key;
    button.setAttribute("aria-disabled", "true");
    try {
      await removeCartridge(key);
      listRevision++;
      render(entries.filter(entry => !sameKey(entry.key, key)));
      clearProblem(key);
      if (dialog.open) void refreshList();
    } catch {
      reportProblem(key, `Could not remove ${name}. Choose Remove to retry.`, !dialog.open);
    } finally {
      removing = undefined;
      list.querySelector('[aria-disabled="true"]')?.removeAttribute("aria-disabled");
    }
  }

  // Repeated activation must not act on whatever control moves into a removed row.
  const suppressEnterRepeat = (event: KeyboardEvent) => {
    if (event.key === "Enter" && event.repeat) event.preventDefault();
  };
  trigger.addEventListener("keydown", suppressEnterRepeat);
  dialog.addEventListener("keydown", suppressEnterRepeat);
  document.addEventListener("click", event => {
    if (!lastRemoval || !event.detail) return;
    const { button, bounds, time } = lastRemoval;
    const nearby = event.clientX >= bounds.left - 8 && event.clientX <= bounds.right + 8
      && event.clientY >= bounds.top - 8 && event.clientY <= bounds.bottom + 8;
    if (!button.isConnected && (event.detail > 1 || (nearby && event.timeStamp - time < 500))) {
      event.preventDefault();
      event.stopPropagation();
    } else {
      lastRemoval = undefined;
    }
  }, { capture: true });

  trigger.addEventListener("click", () => {
    onOpen();
    trigger.focus({ preventScroll: true });
    dialog.showModal();
    void refreshList();
  });
  close.addEventListener("click", () => dialog.close());
  let outside = false;
  function isBackdrop(event: MouseEvent) {
    const bounds = dialog.getBoundingClientRect();
    return event.target === dialog && (event.clientX < bounds.left || event.clientX >= bounds.right
      || event.clientY < bounds.top || event.clientY >= bounds.bottom);
  }
  dialog.addEventListener("pointerdown", event => { outside = isBackdrop(event); });
  dialog.addEventListener("click", event => {
    if (outside && isBackdrop(event)) dialog.close();
    outside = false;
  });
  local.addEventListener("click", () => {
    dialog.close();
    openLocal();
  });
  dismiss.addEventListener("click", () => {
    noticeSource = null;
    updateNotice();
    trigger.focus({ preventScroll: true });
  });

  return {
    get open() { return dialog.open; },
    clearProblem,
    readFailed(key: IDBValidKey) {
      reportProblem(key, "Could not read the remembered ROM. Reopen the original file or remove the entry.", true);
    },
    async remember(key: string, name: string, bytes: Uint8Array) {
      try {
        await rememberCartridge(key, name, bytes);
      } catch {
        failures.delete(key);
        failures.set(key, name);
        noticeSource = "retention";
        updateRetention();
        return;
      }
      clearProblem(key);
      if (failures.delete(key)) updateRetention();
      if (dialog.open) void refreshList();
    },
  };
}
