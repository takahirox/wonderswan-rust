// Production WebAssembly replay of original CC0 cartridges.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { inflateSync } from 'node:zlib';

export async function saveStateWasmChecks(browser, base) {
  const probes = await Promise.all([
    ['mono', 'sram32-common'], ['color', 'gdma'], ['crystal', 'sram128-2003'],
    ['crystal', 'sound-hyper'], ['color', 'eeprom2048-packed'],
  ].map(async ([model, name]) => ({
    model, name, bytes: [...await readFile(new URL(`../../fixtures/generated/${name}.ws`, import.meta.url))],
  })));
  const bindings = await readFile(new URL('../pkg/ws_web.js', import.meta.url), 'utf8');
  const page = await browser.newPage();
  try {
    const served = page.waitForResponse(response => response.url().endsWith('.wasm') && response.ok());
    await page.goto(base);
    const wasm = await (await served).body();
    const exports = await page.evaluate(async ({ bindings, wasm, probes }) => {
      const url = URL.createObjectURL(new Blob([bindings], { type: 'text/javascript' }));
      const equal = (a, b, context) => {
        if (a.length !== b.length || a.some((value, i) => value !== b[i])) throw new Error(context);
      };
      try {
        const { default: init, WebMachine, max_snapshot_bytes } = await import(url);
        await init({ module_or_path: new Uint8Array(wasm) });
        return await Promise.all(probes.map(async ({model, name, bytes}, probe) => {
          const m = new WebMachine();
          const other = new WebMachine();
          const inputs = [32, 260, 512, 0];
          const clockStep = 371;
          const initialAnchor = 1000;
          const restoredAnchor = 200000;
          function drive(machine, anchor) {
            const pcm = [];
            for (const [index, mask] of inputs.entries()) {
              machine.sync_rtc(anchor + (index + 1) * clockStep);
              machine.set_buttons(mask);
              machine.run_frame();
              pcm.push(...machine.audio_samples());
            }
            return pcm;
          }
          try {
            for (const machine of [m, other]) {
              machine.reset(model);
              machine.load_rom(Uint8Array.from(bytes));
            }
            m.sync_rtc(initialAnchor);
            const snapshots = [];
            // The reference runs straight through power-on and the active boundary.
            for (const frame of [0, inputs.length]) {
              const state = m.capture_snapshot();
              snapshots.push(state);
              const prepared = other.prepare_snapshot(state);
              const pixels = prepared.framebuffer();
              other.run_frame(); // Leave generated PCM in the restore target.
              other.apply_snapshot(prepared, restoredAnchor);
              if (m.frame_number() !== frame || other.frame_number() !== frame) {
                throw new Error(`${name}: wrong frame boundary`);
              }
              equal(other.audio_samples(), [], 'old generated PCM leaked');
              equal(other.framebuffer(), pixels, 'captured framebuffer');
              equal(other.framebuffer(), m.framebuffer(), 'restored framebuffer');
              equal(other.save_data(), m.save_data(), 'restored SRAM/EEPROM');
              equal(other.console_data(), m.console_data(), 'restored console data');
              const pcm = drive(m, initialAnchor + frame * clockStep);
              if (probe === 3 && !pcm.some(sample => sample !== 0)) throw new Error('no Hyper Voice PCM');
              equal(drive(other, restoredAnchor), pcm, `${name}: resumed PCM`);
              equal(other.capture_snapshot(), m.capture_snapshot(), `${name}: machine replay`);
              equal(other.rtc_data().slice(0, 16), m.rtc_data().slice(0, 16), 'clock anchor rebase');
            }
            const [boot, active] = snapshots;
            const baseline = m.capture_snapshot();
            const version = active.slice();
            version[8] = 2;
            const modelMismatch = active.slice();
            modelMismatch[10] = (active[10] + 1) % 3;
            const romMismatch = active.slice();
            romMismatch[11] ^= 1;
            const corrupt = active.slice();
            corrupt[corrupt.length - 1] ^= 1;
            const rejected = [
              [version, 'version'], [modelMismatch, 'different physical'], [romMismatch, 'different ROM'],
              [corrupt, 'checksum'], [active.slice(0, -1), 'lengths'],
              [new Uint8Array(max_snapshot_bytes() + 1), 'size limit'],
            ];
            if (probe === 0) {
              const invalidState = new Uint8Array(4096);
              const stream = new Blob([invalidState]).stream().pipeThrough(new CompressionStream('deflate'));
              const body = new Uint8Array(await new Response(stream).arrayBuffer());
              const forged = new Uint8Array(83 + body.length);
              forged.set(active.slice(0, 51));
              const sizes = new DataView(forged.buffer);
              sizes.setUint32(43, invalidState.length, true);
              sizes.setUint32(47, body.length, true);
              const checksummed = new Uint8Array(51 + body.length);
              checksummed.set(forged.slice(0, 51));
              checksummed.set(body, 51);
              forged.set(new Uint8Array(await crypto.subtle.digest('SHA-256', checksummed)), 51);
              forged.set(body, 83);
              rejected.push([forged, 'Invalid machine state']);
            }
            for (const [invalid, message] of rejected) {
              let error;
              try { m.prepare_snapshot(invalid).free(); } catch (err) { error = err; }
              if (!error?.message.includes(message)) throw new Error(`expected ${message}: ${error}`);
              equal(m.capture_snapshot(), baseline, 'validation changed the live machine');
            }
            return [...boot];
          } finally {
            m.free();
            other.free();
          }
        }));
      } finally { URL.revokeObjectURL(url); }
    }, { bindings, wasm: [...wasm], probes });
    for (const [index, state] of exports.entries()) {
      const file = Buffer.from(state);
      const rom = Buffer.from(probes[index].bytes);
      assert.deepEqual(file.subarray(11, 43), createHash('sha256').update(rom).digest());
      assert.equal(inflateSync(file.subarray(83)).includes(rom.subarray(-64)), false);
    }
    console.log('Production Wasm save states: three physical models, IPL boundary, DMA, CPU/video/audio/input, native bytes, clock rebase, file rejection and raw ROM identity passed');
  } finally { await page.close(); }
}
