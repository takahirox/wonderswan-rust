// Browser-only storage. A ROM's content, never its name, selects its save slot.
export async function romIdentity(bytes: Uint8Array): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new Uint8Array(bytes));
  return `wonderswan:save:v1:${Array.from(new Uint8Array(digest), b => b.toString(16).padStart(2, "0")).join("")}`;
}
export class Saves {
  private blocked = new Set<string>();
  private previous = new Map<string, string>();
  constructor(private report: (message: string) => void) {}
  restore(key: string, expected: number): Uint8Array | null {
    if (!expected) return null;
    try {
      const value = localStorage.getItem(key);
      if (value === null) return null;
      const decoded = Uint8Array.from(atob(value), c => c.charCodeAt(0));
      if (decoded.length !== expected) throw new Error(`Expected ${expected} save bytes, found ${decoded.length}`);
      this.previous.set(key, value);
      return decoded;
    } catch {
      this.blocked.add(key); // Keep an unreadable existing save intact.
      this.report("This cartridge's local save could not be read. Its stored data will be preserved; changes this session cannot be saved.");
      return null;
    }
  }
  store(key: string, data: Uint8Array) {
    if (!data.length || this.blocked.has(key)) return;
    try {
      let text = "";
      // Avoid overflowing the argument stack for 512 KiB SRAM.
      for (let i = 0; i < data.length; i += 8192) text += String.fromCharCode(...data.subarray(i, i + 8192));
      const encoded = btoa(text);
      if (this.previous.get(key) === encoded) return;
      localStorage.setItem(key, encoded);
      this.previous.set(key, encoded);
    } catch {
      this.report("Local save storage is unavailable or full. Keep this page open to retain the current save.");
    }
  }
}
