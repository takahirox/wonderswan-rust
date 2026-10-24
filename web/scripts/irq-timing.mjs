// SPDX-License-Identifier: CC0-1.0
// Independent absolute budgets: 62482 / 68626 clocks, quantized at 256.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

export async function irqTimingChecks(browser, base, errors) {
  const bytes = await readFile(new URL('../../fixtures/generated/irq-timing.ws',import.meta.url));
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
      await page.locator('#rom').setInputFiles({name:'irq-timing.ws',mimeType:'application/octet-stream',buffer:bytes});
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
          for (const [sample,low,high] of [[0,244,245],[1,268,269]]) {
            const i = 4+pass*8+sample*4;
            observations.push(word(i));
            assert.ok(word(i)>=low && word(i)<=high,`${model} boot ${boot} pass ${pass} sample ${sample}: ${word(i)} ticks`);
            assert.deepEqual([word(i+1),word(i+2),word(i+3)],[1024,0,0],'CPU handler count and application/handler continuation');
          }
        }
        console.log(`${model} boot ${boot}: HBlank samples ${observations}; counts/continuation 1024/0/0`);
      }
      assert.equal(requests.length,0,'ROM/save handling stays local');
    } finally { await context.close(); }
  }
  console.log('Absolute IRQ budget: 1024 deliveries, 244/268 HBlank ticks (+1 quantization), counts/continuation, all nine configurations, live modes and fresh restart passed');
}
