import type { WebMachine } from "../pkg/ws_web.js";
import { stateThumbnail } from "./display";
import { applyNativeWrites, consoleKey, encodeRtc, encodeSave, rtcKey, type RtcRecords, type Saves } from "./saves";
import { writeStates, type StateRecord } from "./state-history";

interface NativeRecords {
  saves: Saves;
  clocks: RtcRecords;
  consoles: Saves;
  heldWrites: Set<string>;
}

function sameBytes(a: Uint8Array, b: Uint8Array, clock: boolean) {
  // The first 16 bytes hold guest clock data. The rest is the host anchor.
  return a.length === b.length && (clock ? a.subarray(0, 16) : a).every((byte, index) => byte === b[index]);
}

export async function restoreState(
  machine: WebMachine, bytes: Uint8Array, scope: string, romKey: string, vertical: boolean,
  { saves, clocks, consoles, heldWrites }: NativeRecords,
): Promise<boolean> {
  const candidate = machine.prepare_snapshot(bytes);
  let consumed = false;
  const consoleSlot = consoleKey(machine.model());
  const clockSlot = rtcKey(romKey);
  const timestamp = Date.now();
  // A paused or hidden player may not have sampled the outgoing clock recently.
  machine.sync_rtc(timestamp);
  const live = [
    { type: "save" as const, key: romKey, bytes: machine.save_data() },
    { type: "rtc" as const, key: clockSlot, bytes: machine.rtc_data() },
    { type: "console" as const, key: consoleSlot, bytes: machine.console_data() },
  ].filter(record => record.bytes.length);

  try {
    const targets = {
      save: candidate.save_data(), rtc: candidate.rtc_data(0), console: candidate.console_data(),
    };
    const records = live.map(record => {
      const { type, key, bytes } = record;
      const clock = type === "rtc";
      const store = type === "save" ? saves : consoles;
      const stored = clock
        ? clocks.inspect(key, data => machine.validate_rtc_data(data))
        : store.inspect(key, bytes.length);
      const target = targets[type];
      const liveChanges = !sameBytes(bytes, target, clock);
      const storedChanges = !!stored && !sameBytes(stored.bytes, target, clock);
      const needsSeparateBackup = storedChanges && !sameBytes(stored.bytes, bytes, clock);
      return { ...record, stored, target, liveChanges, storedChanges, needsSeparateBackup };
    });
    const recovery: StateRecord[] = [];
    if (records.some(record => record.liveChanges || record.storedChanges)) {
      const outgoing = machine.capture_snapshot();
      const thumbnail = await stateThumbnail(machine.framebuffer(), vertical);
      const backup = (bytes: Uint8Array, label: string) => ({
        entry: { scope, id: crypto.randomUUID(), timestamp, thumbnail, label }, bytes,
      });
      recovery.push(backup(outgoing, "Recovery · live progress"));
      if (records.some(record => record.needsSeparateBackup)) {
        const persisted = machine.prepare_snapshot(outgoing);
        const distinct = (key: string) => records.find(record => record.key === key && record.needsSeparateBackup)
          ?.stored?.bytes;
        try {
          persisted.replace_native_records(distinct(romKey), distinct(clockSlot), distinct(consoleSlot));
          recovery.push(backup(persisted.encode(), "Recovery · stored progress"));
        } finally {
          persisted.free();
        }
      }
      try {
        await writeStates(...recovery);
      } catch (error) {
        const detail = error instanceof Error ? error.message : String(error);
        throw new Error(`Could not save recovery progress. ${detail}`);
      }
    }

    const now = Date.now();
    const writes = records.filter(record => record.liveChanges || record.storedChanges || record.type === "rtc")
      .map(({ type, key, stored, target }) => ({
        key,
        before: stored?.text ?? null,
        after: type === "rtc" ? encodeRtc(candidate.rtc_data(now)) : encodeSave(target),
      }));
    applyNativeWrites(writes, () => {
      consumed = true;
      machine.apply_snapshot(candidate, now);
    });
    for (const { key, after } of writes) {
      if (key === romKey) saves.accept(key, after);
      if (key === consoleSlot) consoles.accept(key, after);
    }
    for (const { key } of live) heldWrites.delete(key);
    return recovery.length > 0;
  } catch (error) {
    // A failed restore must not let routine autosave replace the preserved
    // records with distinct in-memory data. A later successful load can retry.
    for (const { key } of live) heldWrites.add(key);
    const detail = error instanceof Error ? error.message : String(error);
    throw new Error(`${detail} Automatic saving for this progress is on hold until Load succeeds or the page is reloaded.`);
  } finally {
    if (!consumed) candidate.free();
  }
}
