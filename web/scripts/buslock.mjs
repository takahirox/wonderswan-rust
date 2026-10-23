// SPDX-License-Identifier: CC0-1.0
// Actual CPU-written results from the original cartridge, production Wasm/player.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

// Independent literals, not imported from the core or generated from assembly.
const expected = [0x600d,0xabce,0x005a,0xf046,0x1234,0x3210,
  0x7654,0xbeef,0x1234,0x5678,0x2468,0xf000];

export async function buslockChecks(browser, base, errors) {
  const bytes = await readFile(new URL('../../fixtures/generated/buslock.ws',import.meta.url));
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
      page.on('request', request => requests.push(request.url()));
      const chooser = page.waitForEvent('filechooser');
      await page.locator('#load-rom').click();
      await (await chooser).setFiles({name:'buslock.ws',mimeType:'application/octet-stream',buffer:bytes});
      for (let boot=1;boot<=2;boot++) {
        if (boot === 2) await page.locator('#reset').click();
        // Fresh retained counter prevents old SRAM/pass pixels satisfying reset.
        await page.waitForFunction(({key,boot,model}) => {
          const saved = localStorage.getItem(key);
          if (!saved || document.querySelector('#rom-status').textContent !== 'Running') return false;
          const bytes = atob(saved);
          const count = bytes.charCodeAt(24) | bytes.charCodeAt(25) << 8;
          const pixel = document.querySelector('#screen').getContext('2d').getImageData(0,0,1,1).data;
          return count === boot && (model === 'mono' ? [170,170,170] : [0,255,0]).every((v,i) => pixel[i] === v);
        },{key,boot,model});
        assert.equal(await page.locator('#error').textContent(),'');
        await page.locator('#pause').click(); // real player flush, real SRAM
        const saved = await page.evaluate(key => Array.from(atob(localStorage.getItem(key)),c => c.charCodeAt(0)),key);
        assert.equal(saved.length,32768);
        for (const [i,value] of [...expected,boot].entries()) {
          assert.equal(saved[i*2] | saved[i*2+1] << 8,value,`${model} boot ${boot} word ${i}`);
        }
      }
      assert.equal(requests.length,0,'ROM and save data stay browser-local');
    } finally { await context.close(); }
  }
  console.log('BUSLOCK CPU probe: memory, comparison flags, stack, immediate/DX byte/word I/O, seven mixed/redundant prefixes, REP, NOP/register effects and restart passed on all three production Wasm models');
}
