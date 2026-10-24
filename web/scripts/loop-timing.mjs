// SPDX-License-Identifier: CC0-1.0
// Independent absolute budgets: 45064 / 49161 / 49160 / 53257 / 94216 / 98312 clocks, quantized at 256.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

export async function loopTimingChecks(browser, base, errors) {
  const bytes = await readFile(new URL('../../fixtures/generated/loop-timing.ws',import.meta.url));
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
      await page.locator('#rom').setInputFiles({name:'loop-timing.ws',mimeType:'application/octet-stream',buffer:bytes});
      for (let boot=1;boot<=2;boot++) {
        if (boot===2) await page.locator('#reset').click();
        await page.waitForFunction(({key,boot}) => {
          const saved = localStorage.getItem(key);
          if (!saved || document.querySelector('#rom-status').textContent!=='Running') return false;
          const bytes = atob(saved);
          return (bytes.charCodeAt(2) | bytes.charCodeAt(3)<<8)===boot
            && (bytes.charCodeAt(0) | bytes.charCodeAt(1)<<8)===0x600d;
        },{key,boot});
        assert.equal(await page.locator('#error').textContent(),'');
        await page.locator('#pause').click();
        const saved = await page.evaluate(key => Array.from(atob(localStorage.getItem(key)),c => c.charCodeAt(0)),key);
        const word = i => saved[2*i] | saved[2*i+1]<<8;
        assert.equal(saved.length,32768);
        assert.equal(word(0),0x600d);
        assert.equal(word(1),boot);
        assert.equal(word(2),model==='mono' ? 0 : 2);
        const passes = model==='mono' ? 1 : 7;
        assert.equal(word(3),passes);
        const observations = [];
        for (let pass=0;pass<passes;pass++) {
          for (const [sample,low,high,cx] of [[0,176,177,0],[1,192,193,0],[2,192,193,0],[3,208,209,0],[4,192,193,0],[5,208,209,0],[6,368,369,0],[7,384,385,1],[8,384,385,1]]) {
            const i = 4+pass*36+sample*4;
            observations.push(word(i));
            assert.ok(word(i)>=low && word(i)<=high,`${model} boot ${boot} pass ${pass} sample ${sample}: ${word(i)} ticks`);
            assert.deepEqual([word(i+1),word(i+2),word(i+3)],[4096,cx,0x51a7],'CPU iteration count, final CX and destination/continuation');
          }
        }
        console.log(`${model} boot ${boot}: HBlank samples ${observations}; count 4096, CX and continuation checked`);
      }
      assert.equal(requests.length,0,'ROM/save handling stays local');
    } finally { await context.close(); }
  }
  console.log('Absolute LOOP budgets: all three opcodes, even/odd taken and fallthrough, counts/CX/continuation, all nine configurations, live modes and fresh restart passed');
}
