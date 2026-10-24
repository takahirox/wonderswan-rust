// SPDX-License-Identifier: CC0-1.0
// Original CPU timer observations through the production player and Wasm.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

export async function ordinaryIoChecks(browser, base, errors) {
  const bytes = await readFile(new URL('../../fixtures/generated/ordinary-io.ws', import.meta.url));
  const key = `wonderswan:save:v1:${createHash('sha256').update(bytes).digest('hex')}`;
  for (const model of ['mono', 'color', 'crystal']) {
    const context = await browser.newContext();
    await context.addInitScript(() => {
      // Control only host time delivery to inspect the real player's first
      // frame and save flush. Core execution and result bytes are unchanged.
      let next = 0;
      window.ioRAF = new Map();
      window.requestAnimationFrame = callback => { window.ioRAF.set(++next, callback); return next; };
      window.cancelAnimationFrame = id => window.ioRAF.delete(id);
      localStorage.setItem('wonderswan:audio:v1', '{"muted":true,"volume":70}');
    });
    const page = await context.newPage();
    page.on('pageerror', error => errors.push(error.message));
    const requests = [];
    const click = selector => page.locator(selector).evaluate(button => button.click());
    const step = time => page.evaluate(time => {
      const callbacks = [...window.ioRAF.values()];
      window.ioRAF.clear();
      callbacks.forEach(callback => callback(time));
    }, time);
    const saved = () => page.evaluate(key => Array.from(atob(localStorage.getItem(key)), c => c.charCodeAt(0)), key);
    try {
      await page.goto(base);
      await page.waitForFunction(() => !document.querySelector('#model').disabled, null, {polling:25});
      await page.locator('#model').selectOption(model);
      await page.evaluate(key => {
        const stale = new Uint8Array(32768).fill(0x6d);
        stale.set([0, 0, 0x0d, 0x60]);
        localStorage.setItem(key, btoa(String.fromCharCode(...stale)));
      }, key);
      page.on('request', request => requests.push(request.url()));
      await page.locator('#rom').setInputFiles({name:'ordinary-io.ws', mimeType:'application/octet-stream', buffer:bytes});
      await page.waitForFunction(() => document.querySelector('#rom-status').textContent === 'Running', null, {polling:25});
      for (let boot = 1; boot <= 2; boot++) {
        if (boot === 2) await click('#reset');
        await step(0);
        await step(14); // exactly one default frame, before samples finish
        await click('#pause'); // real player persists current SRAM
        const pending = await saved();
        assert.equal(pending[0] | pending[1] << 8, boot, `${model}: fresh boot began`);
        assert.equal(pending[2] | pending[3] << 8, 0, `${model}: retained success invalidated before completion`);
        await click('#pause'); // resume
        await step(0);
        for (let i = 1; i <= 30; i++) await step(i * 100); // bounded 120 frames
        await click('#pause');
        assert.equal(await page.locator('#error').textContent(), '');
        const data = await saved();
        const word = offset => data[offset] | data[offset + 1] << 8;
        assert.equal(data.length, 32768);
        assert.equal(word(0), boot);
        const modes = model === 'mono' ? [0] : [0, 0x80, 0, 0xc0, 0, 0xe0, 0];
        assert.equal(word(4), modes.length);
        for (const [phase, mode] of modes.entries()) {
          assert.equal(word(6 + phase * 26), mode);
          // Independent bounds: 11 + 2048*8/9/10 clocks, quantized by 256.
          // Initial IN + MOV are included; terminal IN samples before charging.
          for (const [index, low] of [64,64,72,64,64,72,72,72,80,72,72,80].entries()) {
            const ticks = word(8 + phase * 26 + index * 2);
            assert.ok(ticks >= low && ticks <= low + 1,
              `${model} boot ${boot} phase ${phase} sample ${index}: ${ticks}, expected ${low}..${low + 1}`);
          }
        }
        assert.equal(word(2), 0x600d, `${model}: CPU self-check`);
        assert.ok(data.slice(6 + modes.length * 26, 256).every(v => v === 0), 'volatile result tail cleared on fresh boot');
      }
      assert.equal(requests.length, 0, 'ROM/save bytes stay browser-local');
    } finally { await context.close(); }
  }
  console.log('Ordinary IN/OUT: absolute CPU-written HBlank observations for all eight encodings, byte/even/odd bursts, 64/65, 72/73 and 80/81 independent tick bounds, all models/runtime round trips, invalidated retained success and fresh restart passed');
}
