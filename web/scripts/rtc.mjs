// SPDX-License-Identifier: CC0-1.0
// Actual CPU-written results from the original cartridge, production Wasm/player.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

// Independent literals, not imported from the core or generated from assembly.
function expected(model,boot) {
  const mono = model === 'mono';
  const words = [0x600d,boot,mono ? 0 : 2,mono ? 1 : 7];
  for (const mode of (mono ? [0] : [0,0x80,0,0xc0,0,0xe0,0])) {
    words.push(mode);
    for (const [command,count] of [
      [0x10,1],[0x11,1],[0x12,1],[0x13,1],[0x14,7],[0x15,7],
      [0x16,3],[0x17,3],[0x18,2],[0x19,2],[0x1a,2],[0x1b,2],[0x93,1],
    ]) {
      words.push(count,0x80 | (command & 15),command & 15,
        command & 1 ? 255 : 0,command & 0x7f);
    }
  }
  while (words.length < 466) words.push(0);
  return words;
}

export async function rtcChecks(browser, base, errors) {
  const bytes = await readFile(new URL('../../fixtures/generated/rtc.ws',import.meta.url));
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
      await page.locator('#settings').click();
      await page.locator('#rtc-mode').selectOption('absent');
      await page.locator('#help-close').click();
      // Poison retained observations, including an old pass marker. The ROM
      // must clear them and complete a fresh boot to satisfy the assertions.
      await page.evaluate(key => {
        const stale = new Uint8Array(32768).fill(0x6d);
        stale.set([0x0d,0x60,0,0]);
        localStorage.setItem(key,btoa(String.fromCharCode(...stale)));
      },key);
      page.on('request', request => requests.push(request.url()));
      const chooser = page.waitForEvent('filechooser');
      await page.locator('#load-rom').click();
      await (await chooser).setFiles({name:'rtc.ws',mimeType:'application/octet-stream',buffer:bytes});
      for (let boot=1;boot<=2;boot++) {
        if (boot === 2) await page.locator('#reset').click();
        // Fresh retained counter prevents old SRAM/pass pixels satisfying reset.
        await page.waitForFunction(({key,boot,model}) => {
          const saved = localStorage.getItem(key);
          if (!saved || document.querySelector('#rom-status').textContent !== 'Running') return false;
          const bytes = atob(saved);
          const count = bytes.charCodeAt(2) | bytes.charCodeAt(3) << 8;
          const phases = bytes.charCodeAt(6) | bytes.charCodeAt(7) << 8;
          const pixel = document.querySelector('#screen').getContext('2d').getImageData(0,0,1,1).data;
          return count === boot && phases === (model === 'mono' ? 1 : 7) && (model === 'mono' ? [170,170,170] : [0,255,0]).every((v,i) => pixel[i] === v);
        },{key,boot,model});
        assert.equal(await page.locator('#error').textContent(),'');
        await page.locator('#pause').click(); // real player flush, real SRAM
        const saved = await page.evaluate(key => Array.from(atob(localStorage.getItem(key)),c => c.charCodeAt(0)),key);
        assert.equal(saved.length,32768);
        for (const [i,value] of expected(model,boot).entries()) {
          assert.equal(saved[i*2] | saved[i*2+1] << 8,value,`${model} boot ${boot} word ${i}`);
        }
      }
      assert.equal(requests.length,0,'ROM and save data stay browser-local');
    } finally { await context.close(); }
  }
  console.log('RTC CPU probe: all supported command counts, reset, terminal status/consumption, absent FF data, READY masking, runtime graphics modes and fresh restart passed on all three production Wasm models');
}
