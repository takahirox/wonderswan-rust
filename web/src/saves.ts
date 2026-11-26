// Browser-only storage. A ROM's content, never its name, selects its save slot.
export async function romIdentity(bytes: Uint8Array): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new Uint8Array(bytes));
  return `wonderswan:save:v1:${Array.from(new Uint8Array(digest), b => b.toString(16).padStart(2, "0")).join("")}`;
}

function encodeSave(data: Uint8Array): string {
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
  restore(key: string, expected: number): Uint8Array | null {
    if (!expected) return null;
    try {
      const text = localStorage.getItem(key);
      if (text === null) return null;
      const bytes = decodeSave(text);
      if (bytes.length !== expected) throw new Error("Unexpected local save length.");
      this.previous.set(key, text);
      return bytes;
    } catch {
      this.blocked.add(key); // Keep an unreadable existing save intact.
      this.report(this.console
        ? "This console model’s local data could not be read. Its stored record will be preserved; console changes this session cannot be saved."
        : "This cartridge's local save could not be read. Its stored data will be preserved; changes this session cannot be saved.");
      return null;
    }
  }
  store(key: string, data: Uint8Array) {
    if (!data.length || this.blocked.has(key)) return;
    try {
      const encoded = encodeSave(data);
      if (this.previous.get(key) === encoded) return;
      localStorage.setItem(key, encoded);
      this.previous.set(key, encoded);
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
const encodeRtc = (bytes: Uint8Array) => JSON.stringify({ version: 1, data: encodeSave(bytes) });

export class RtcRecords {
  private blocked = new Set<string>();
  constructor(private report: (message: string) => void) {}
  restore(key: string, load: (bytes: Uint8Array) => void) {
    try {
      const text = localStorage.getItem(key);
      if (text === null) return;
      const record = JSON.parse(text);
      if (record.version !== 1 || typeof record.data !== "string") throw new Error("Unsupported RTC record");
      load(decodeSave(record.data));
    } catch {
      this.blocked.add(key);
      this.report("This cartridge’s clock record could not be restored. Its stored record will be preserved.");
    }
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
