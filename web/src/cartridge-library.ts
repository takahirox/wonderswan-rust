import { database } from "./database";

const databaseName = "wonderswan-rust:cartridges";
const metadata = "cartridges";
const roms = "roms";
const stores = [metadata, roms];
const unnamedCartridge = "Unnamed cartridge";
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

const transaction = database(databaseName, "Cartridge", db => {
  db.createObjectStore(metadata);
  db.createObjectStore(roms);
});

export async function listCartridges(): Promise<CartridgeEntry[]> {
  const [metadataKeys, records, romKeys] = await transaction(stores, "readonly", tx => {
    const store = tx.objectStore(metadata);
    return [store.getAllKeys(), store.getAll(), tx.objectStore(roms).getAllKeys()] as const;
  });
  // Use the actual database key so even malformed records can be removed.
  const keys = metadataKeys.result;
  const entries = keys.map((key, index) => ({
    key, name: filename(records.result[index]) ?? unnamedCartridge,
  }));
  // Both key lists are in IndexedDB key order. Include bodies missing metadata
  // without reading their bytes, and compare compound keys by value.
  let metadataIndex = 0;
  for (const key of romKeys.result) {
    while (metadataIndex < keys.length && indexedDB.cmp(keys[metadataIndex], key) < 0) metadataIndex++;
    if (metadataIndex === keys.length || indexedDB.cmp(keys[metadataIndex], key) !== 0) {
      entries.push({ key, name: unnamedCartridge });
    }
  }
  return entries;
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
    tx.objectStore(metadata).put({ name: filename({ name }) ?? unnamedCartridge }, key);
    tx.objectStore(roms).put(bytes, key);
  });
}

export async function removeCartridge(key: IDBValidKey) {
  await transaction(stores, "readwrite", tx => {
    tx.objectStore(metadata).delete(key);
    tx.objectStore(roms).delete(key);
  });
}
