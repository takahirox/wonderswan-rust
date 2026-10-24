// SPDX-License-Identifier: CC0-1.0
// Independent frame literals, checked against CPU-written SRAM in the real player.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

function expected(model, boot) {
  const mono = model === 'mono';
  const words = [0x600d,boot,mono ? 0 : 2,mono ? 1 : 7];
  // IP, CS, saved FLAGS, live handler FLAGS, entry SP, CX, SI, DI, BX.
  const cases = [
    [[0x1005,0,0xf102,0xf002,0x2ff4,2,0x1800,0x1900,0]],
    [[0x1005,0,0xf302,0xf002,0x2ff4,2,0x1800,0x1900,0]],
    [[0x1004,0,0xf102,0xf002,0x2ff4,1,0x1801,0x1901,0],
     [0x1008,0,0xf102,0xf002,0x2ff4,0,0x1802,0x1902,0]],
    [[0x1008,0,0xf102,0xf002,0x2ff4,0,0x1800,0x1900,0]],
    [[0x1005,0,0xf102,0xf002,0x2ff4,2,0x1800,0x1900,0]],
    [[0x1007,0,0xf102,0xf002,0x2ff4,2,0x1800,0x1900,0]],
    [[0x1005,0,0xf102,0xf002,0x2ff4,2,0x1800,0x1900,0],
     [0x1006,0,0xf102,0xf002,0x2ff4,2,0x1800,0x1900,0]],
    [[0x1009,0,0xf102,0xf002,0x2ff4,2,0x1800,0x1900,0]],
    [[0x1008,0,0xf102,0xf002,0x2ff4,2,0x1800,0x1900,0]],
    [[0x1006,0,0xf102,0xf002,0x2ff4,2,0x1800,0x1900,0]],
  ];
  for (let pass=0;pass<(mono ? 1 : 7);pass++) {
    for (const frames of cases) {
      words.push(frames.length,0x1234);
      for (const frame of frames) words.push(...frame);
    }
  }
  while (words.length<900) words.push(0);
  return words;
}

export async function trapChecks(browser, base, errors) {
  const bytes = await readFile(new URL('../../fixtures/generated/trap.ws',import.meta.url));
  const key = `wonderswan:save:v1:${createHash('sha256').update(bytes).digest('hex')}`;
  for (const model of ['mono','color','crystal']) {
    const context = await browser.newContext();
    const page = await context.newPage();
    page.on('pageerror', error => errors.push(error.message));
    const requests = [];
    try {
      await page.goto(base);
      await page.waitForFunction(() => !document.querySelector('#model').disabled);
      await page.locator('#model').selectOption(model);
      await page.evaluate(key => {
        const stale = new Uint8Array(32768).fill(0x6d);
        stale.set([0x0d,0x60,0,0]);
        localStorage.setItem(key,btoa(String.fromCharCode(...stale)));
      },key);
      page.on('request', request => requests.push(request.url()));
      const chooser = page.waitForEvent('filechooser');
      await page.locator('#load-rom').click();
      await (await chooser).setFiles({name:'trap.ws',mimeType:'application/octet-stream',buffer:bytes});
      for (let boot=1;boot<=2;boot++) {
        if (boot===2) await page.locator('#reset').click();
        await page.waitForFunction(({key,boot,model}) => {
          const saved = localStorage.getItem(key);
          if (!saved || document.querySelector('#rom-status').textContent!=='Running') return false;
          const bytes = atob(saved);
          const count = bytes.charCodeAt(2) | bytes.charCodeAt(3)<<8;
          const pixel = document.querySelector('#screen').getContext('2d').getImageData(0,0,1,1).data;
          return count===boot && (model==='mono' ? [170,170,170] : [0,255,0]).every((v,i) => pixel[i]===v);
        },{key,boot,model});
        assert.equal(await page.locator('#error').textContent(),'');
        await page.locator('#pause').click();
        const saved = await page.evaluate(key => Array.from(atob(localStorage.getItem(key)),c => c.charCodeAt(0)),key);
        assert.equal(saved.length,32768);
        for (const [i,value] of expected(model,boot).entries()) {
          assert.equal(saved[2*i] | saved[2*i+1]<<8,value,`${model} boot ${boot} trap word ${i}`);
        }
      }
      assert.equal(requests.length,0,'ROM and save data stay browser-local');
    } finally { await context.close(); }
  }
  console.log('Single-step CPU probe: saved IP/CS/FLAGS, live flags/SP, handler counts, REP progress, SS suppression, HALT, IRET continuation, modes and fresh restart passed on all three production Wasm models');
}
