// Browser-only storage. A ROM's content, never its name, selects its save slot.
export async function romIdentity(bytes: Uint8Array): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new Uint8Array(bytes));
  return `wonderswan:save:v1:${Array.from(new Uint8Array(digest), b => b.toString(16).padStart(2, "0")).join("")}`;
}

export interface StoredRecord {
  text: string;
  bytes: Uint8Array;
}

export function encodeSave(data: Uint8Array): string {
  let text = "";
  // Keep large SRAM records below the argument stack limit.
  for (let i = 0; i < data.length; i += 8192) text += String.fromCharCode(...data.subarray(i, i + 8192));
  return btoa(text);
}

function decodeSave(text: string): Uint8Array {
  return Uint8Array.from(atob(text), c => c.charCodeAt(0));
}

export class Saves {
  private blocked = new Set<string>();
  private previous = new Map<string, string>();
  constructor(private report: (message: string) => void, private console = false) {}
  inspect(key: string, expected: number): StoredRecord | null {
    if (this.blocked.has(key)) throw new Error("An unreadable local save is protected.");
    return this.read(key, expected);
  }
  private read(key: string, expected: number): StoredRecord | null {
    try {
      const text = localStorage.getItem(key);
      if (text === null) return null;
      const bytes = decodeSave(text);
      if (bytes.length !== expected) throw new Error("Unexpected local save length.");
      return { text, bytes };
    } catch {
      this.blocked.add(key); // Keep an unreadable existing save intact.
      this.report(this.console
        ? "This console model’s local data could not be read. Its stored record will be preserved; console changes this session cannot be saved."
        : "This cartridge's local save could not be read. Its stored data will be preserved; changes this session cannot be saved.");
      throw new Error(this.console ? "The local console data is unreadable." : "The local cartridge save is unreadable.");
    }
  }
  restore(key: string, expected: number): Uint8Array | null {
    if (!expected) return null;
    try {
      const record = this.read(key, expected);
      if (record) this.accept(key, record.text);
      return record?.bytes ?? null;
    } catch {
      return null;
    }
  }
  accept(key: string, text: string) { this.previous.set(key, text); }

  store(key: string, data: Uint8Array) {
    if (!data.length || this.blocked.has(key)) return;
    try {
      const encoded = encodeSave(data);
      if (this.previous.get(key) === encoded) return;
      localStorage.setItem(key, encoded);
      this.accept(key, encoded);
    } catch {
      this.report(this.console
        ? "Local console storage is unavailable or full. Keep this page open to retain console data."
        : "Local save storage is unavailable or full. Keep this page open to retain the current save.");
    }
  }
}

// Version and physical model select the console slot, never a ROM identity.
export const consoleKey = (model: string) => `wonderswan:console:v1:${model}`;

export const orientationKey = (saveKey: string) => saveKey.replace(":save:v1:", ":orientation:v1:");

// Independent envelope and key namespace. Failed decoding/import blocks writes
// for that slot for this session, preserving evidence rather than replacing it.
export const rtcKey = (saveKey: string) => saveKey.replace(":save:v1:", ":rtc:v1:");
export const encodeRtc = (bytes: Uint8Array) => JSON.stringify({ version: 1, data: encodeSave(bytes) });

export class RtcRecords {
  private blocked = new Set<string>();
  constructor(private report: (message: string) => void) {}
  inspect(key: string, validate: (bytes: Uint8Array) => void): StoredRecord | null {
    if (this.blocked.has(key)) throw new Error("An unreadable local clock record is protected.");
    return this.read(key, validate);
  }
  private read(key: string, validate: (bytes: Uint8Array) => void): StoredRecord | null {
    try {
      const text = localStorage.getItem(key);
      if (text === null) return null;
      const record = JSON.parse(text);
      if (record.version !== 1 || typeof record.data !== "string") throw new Error("Unsupported RTC record");
      const bytes = decodeSave(record.data);
      validate(bytes);
      return { text, bytes };
    } catch {
      this.blocked.add(key);
      this.report("This cartridge’s clock record could not be restored. Its stored record will be preserved.");
      throw new Error("The local clock record is unreadable.");
    }
  }
  restore(key: string, load: (bytes: Uint8Array) => void) {
    try { this.read(key, load); } catch { /* Keep the protected record. */ }
  }
  store(key: string, bytes: Uint8Array) {
    if (!bytes.length || this.blocked.has(key)) return;
    try {
      localStorage.setItem(key, encodeRtc(bytes));
    } catch {
      this.report("Local clock storage is unavailable or full. Keep this page open to retain the clock.");
    }
  }
}

export interface NativeWrite {
  key: string;
  before: string | null;
  after: string;
}

// Run synchronously after recovery entries commit in IndexedDB. localStorage
// writes are independent, so roll back earlier keys if any later step fails.
export function applyNativeWrites(writes: NativeWrite[], apply: () => void) {
  const written: NativeWrite[] = [];
  try {
    for (const write of writes) {
      if (write.before === write.after) continue;
      localStorage.setItem(write.key, write.after);
      written.push(write);
    }
    apply();
  } catch (error) {
    let rollbackFailed = false;
    for (const { key, before } of written.reverse()) {
      try {
        if (before === null) localStorage.removeItem(key);
        else localStorage.setItem(key, before);
      } catch {
        rollbackFailed = true;
      }
    }
    if (rollbackFailed) throw new Error("Could not roll back local saves. Select a Recovery entry to recover progress.");
    throw error;
  }
}
