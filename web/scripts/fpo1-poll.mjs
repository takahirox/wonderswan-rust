// SPDX-License-Identifier: CC0-1.0
// Actual CPU-written results from the original cartridge, production Wasm/player.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

// Independent literals, not imported from the core or generated from assembly.
const expected = [0x600d,0x1235,0xf003,0x2201,0xf003,0x2202,0xf003,
  0x2203,0xf003,0x2204,0xf003,0x2205,0xf003,0x2206,0xf003,
  0x2207,0xf003,0x2208,0xf003,0xbeef,0xf000];

export async function fpo1PollChecks(browser, base, errors) {
  const bytes = await readFile(new URL('../../fixtures/generated/fpo1-poll.ws',import.meta.url));
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
      await (await chooser).setFiles({name:'fpo1-poll.ws',mimeType:'application/octet-stream',buffer:bytes});
      for (let boot=1;boot<=2;boot++) {
        if (boot === 2) await page.locator('#reset').click();
        // Fresh retained counter prevents old SRAM/pass pixels satisfying reset.
        await page.waitForFunction(({key,boot,model}) => {
          const saved = localStorage.getItem(key);
          if (!saved || document.querySelector('#rom-status').textContent !== 'Running') return false;
          const bytes = atob(saved);
          const count = bytes.charCodeAt(42) | bytes.charCodeAt(43) << 8;
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
  console.log('FPO1/POLL CPU probe: all eight opcodes, memory/prefix-looking bytes, flags/carry, alignment, seven prefixes and fresh restart passed on all three production Wasm models');
}
