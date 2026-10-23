// SPDX-License-Identifier: CC0-1.0
// Actual CPU-written results from the original cartridge, production Wasm/player.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

// Independent literals, not imported from the core or generated from assembly.
function expected(model,boot) {
  const mono = model === 'mono';
  const words = [0x600d,boot,mono ? 0 : 2,mono ? 1 : 3];
  // Independent destination / DX / full FLAGS / ZF/CF/OF branch observations.
  const results = [6,0x1357,mono ? 0xf002 : 0xf042,mono ? 0 : 1,
    0xfffe,1,mono ? 0xfc03 : 0xfc43,mono ? 6 : 7,
    0xffeb,0x1357,0xf042,1,0x8000,0,0xfc43,7,
    0xffeb,0x1357,0xf042,1,0x8000,0x1357,0xf442,1,
    0,0x1357,0xfc43,7,0x7f80,0x1357,0xf843,7];
  for (let i=0;i<(mono ? 1 : 3);i++) words.push(...results);
  while (words.length < 100) words.push(0);
  return words;
}

export async function multiplicationChecks(browser, base, errors) {
  const bytes = await readFile(new URL('../../fixtures/generated/multiply.ws',import.meta.url));
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
      await (await chooser).setFiles({name:'multiply.ws',mimeType:'application/octet-stream',buffer:bytes});
      for (let boot=1;boot<=2;boot++) {
        if (boot === 2) await page.locator('#reset').click();
        // Fresh retained counter prevents old SRAM/pass pixels satisfying reset.
        await page.waitForFunction(({key,boot,model}) => {
          const saved = localStorage.getItem(key);
          if (!saved || document.querySelector('#rom-status').textContent !== 'Running') return false;
          const bytes = atob(saved);
          const count = bytes.charCodeAt(2) | bytes.charCodeAt(3) << 8;
          const pixel = document.querySelector('#screen').getContext('2d').getImageData(0,0,1,1).data;
          return count === boot && (model === 'mono' ? [170,170,170] : [0,255,0]).every((v,i) => pixel[i] === v);
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
  console.log('Multiplication CPU probe: all six forms, full flags/products, conditional branches, physical models, runtime video transitions and fresh restart passed on all three production Wasm models');
}
