const databaseName = "wonderswan-rust:cartridges";
const metadata = "cartridges";
const roms = "roms";
const stores = [metadata, roms];
const openTimeoutMs = 2000;
export const maxRomBytes = 64 * 1024 * 1024;

export interface CartridgeEntry {
  key: IDBValidKey;
  name: string;
}

export function isRomIdentity(key: IDBValidKey): key is string {
  return typeof key === "string" && /^wonderswan:save:v1:[0-9a-f]{64}$/.test(key);
}

function filename(record: unknown): string | undefined {
  if (typeof record !== "object" || record === null || !("name" in record)) return;
  const { name } = record;
  // Reject empty and control-only names and keep corrupt metadata bounded.
  if (typeof name === "string" && name.length <= 1024 && /[^\s\p{C}]/u.test(name)) return name;
}

function openLibrary(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(databaseName, 1);
    let expired = false;
    const fail = (error: unknown) => {
      expired = true;
      window.clearTimeout(timeout);
      reject(error);
    };
    // An open queued behind a blocked deletion may never receive a blocked event.
    const timeout = window.setTimeout(() => fail(new Error("Cartridge storage timed out.")), openTimeoutMs);
    request.onblocked = () => fail(new Error("Cartridge storage is busy."));
    request.onupgradeneeded = () => {
      if (expired) {
        request.transaction!.abort();
        return;
      }
      try {
        request.result.createObjectStore(metadata);
        request.result.createObjectStore(roms);
      } catch (error) {
        request.transaction!.abort();
        fail(error);
      }
    };
    request.onerror = () => fail(request.error);
    request.onsuccess = () => {
      window.clearTimeout(timeout);
      const db = request.result;
      if (expired) db.close();
      else {
        db.onversionchange = () => db.close();
        resolve(db);
      }
    };
  });
}

async function transaction<T>(
  storeNames: string | string[], mode: IDBTransactionMode, run: (tx: IDBTransaction) => T,
): Promise<T> {
  const db = await openLibrary();
  try {
    return await new Promise<T>((resolve, reject) => {
      const tx = db.transaction(storeNames, mode);
      tx.onabort = () => reject(tx.error ?? new Error("Cartridge storage transaction aborted."));
      try {
        const requests = run(tx);
        tx.oncomplete = () => resolve(requests);
      } catch (error) {
        tx.abort();
        reject(error);
      }
    });
  } finally {
    db.close();
  }
}

export async function listCartridges(): Promise<CartridgeEntry[]> {
  const [keys, records] = await transaction(metadata, "readonly", tx => {
    const store = tx.objectStore(metadata);
    return [store.getAllKeys(), store.getAll()] as const;
  });
  // Use the actual database key so even malformed records can be removed.
  return keys.result.map((key, index) => ({
    key, name: filename(records.result[index]) ?? "Unnamed cartridge",
  }));
}

export async function readCartridge(key: IDBValidKey): Promise<Uint8Array> {
  if (!isRomIdentity(key)) throw new Error("Invalid saved ROM identity.");
  const [record, body] = await transaction(stores, "readonly", tx => [
    tx.objectStore(metadata).get(key), tx.objectStore(roms).get(key),
  ]);
  const bytes = body.result;
  if (!filename(record.result) || !(bytes instanceof Uint8Array)
    || bytes.byteLength === 0 || bytes.byteLength > maxRomBytes) {
    throw new Error("Saved ROM data is missing or invalid.");
  }
  return bytes;
}

export async function rememberCartridge(key: string, name: string, bytes: Uint8Array) {
  await transaction(stores, "readwrite", tx => {
    // The latest committed import supplies the name and repairs both records.
    tx.objectStore(metadata).put({ name: filename({ name }) ?? "Unnamed cartridge" }, key);
    tx.objectStore(roms).put(bytes, key);
  });
}

export async function removeCartridge(key: IDBValidKey) {
  await transaction(stores, "readwrite", tx => {
    tx.objectStore(metadata).delete(key);
    tx.objectStore(roms).delete(key);
  });
}
