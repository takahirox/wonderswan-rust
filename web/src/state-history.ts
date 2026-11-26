import { database } from "./database";

const metadata = "metadata";
const snapshots = "snapshots";
const newest = "scope-and-time";
const pageSize = 20;
const transaction = database("wonderswan-rust:save-states", "Save state", db => {
  db.createObjectStore(metadata, { keyPath: ["scope", "id"] })
    .createIndex(newest, ["scope", "timestamp", "id"]);
  db.createObjectStore(snapshots);
});

export interface StateEntry {
  scope: string;
  id: string;
  timestamp: number;
  thumbnail: Blob;
  label?: string;
}

export async function statePage(scope: string, before?: IDBValidKey) {
  return transaction(metadata, "readonly", tx => {
    const page = { entries: [] as StateEntry[], before: undefined as IDBValidKey | undefined };
    const request = tx.objectStore(metadata).index(newest)
      .openCursor(IDBKeyRange.bound([scope], before ?? [scope, []], true, true), "prev");
    let last: IDBValidKey;
    request.onsuccess = () => {
      const cursor = request.result;
      if (!cursor) return;
      if (page.entries.length === pageSize) {
        page.before = last;
        return;
      }
      page.entries.push(cursor.value);
      last = cursor.key;
      cursor.continue();
    };
    return page;
  });
}

export async function readState(scope: string, id: string): Promise<Uint8Array> {
  const request = await transaction(snapshots, "readonly", tx => tx.objectStore(snapshots).get([scope, id]));
  if (!(request.result instanceof Uint8Array)) throw new Error("Save-state data is missing or unreadable.");
  return request.result;
}

export async function writeState(entry: StateEntry, bytes: Uint8Array) {
  await transaction([metadata, snapshots], "readwrite", tx => {
    tx.objectStore(metadata).put(entry);
    tx.objectStore(snapshots).put(bytes, [entry.scope, entry.id]);
  });
}

export async function deleteState({ scope, id }: StateEntry) {
  await transaction([metadata, snapshots], "readwrite", tx => {
    tx.objectStore(metadata).delete([scope, id]);
    tx.objectStore(snapshots).delete([scope, id]);
  });
}
