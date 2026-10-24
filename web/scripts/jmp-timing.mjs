// SPDX-License-Identifier: CC0-1.0
// Independent absolute budgets: 90120 / 94216 / 106504 / 110600 clocks, quantized at 256.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

export async function jmpTimingChecks(browser, base, errors) {
  const bytes = await readFile(new URL('../../fixtures/generated/jmp-timing.ws',import.meta.url));
  const key = `wonderswan:save:v1:${createHash('sha256').update(bytes).digest('hex')}`;
  for (const model of ['mono','color','crystal']) {
    const context = await browser.newContext();
    await context.addInitScript(() => {
      // Control host RAF delivery to inspect startup invalidation through the
      // real player's pause/save path. Core execution stays unchanged.
      let next = 0;
      window.jmpRAF = new Map();
      window.requestAnimationFrame = callback => { window.jmpRAF.set(++next, callback); return next; };
      window.cancelAnimationFrame = id => window.jmpRAF.delete(id);
      localStorage.setItem('wonderswan:audio:v1','{"muted":true,"volume":70}');
    });
    const page = await context.newPage();
    page.on('pageerror', error => errors.push(error.message));
    const requests = [];
    const click = selector => page.locator(selector).evaluate(button => button.click());
    const step = time => page.evaluate(time => {
      const callbacks = [...window.jmpRAF.values()];
      window.jmpRAF.clear();
      callbacks.forEach(callback => callback(time));
    },time);
    try {
      await page.goto(base);
      await page.waitForFunction(() => !document.querySelector('#model').disabled,null,{polling:25});
      await page.locator('#model').selectOption(model);
      await page.evaluate(key => {
        const stale = new Uint8Array(32768).fill(0x6d);
        stale.set([0x0d,0x60,0,0]);
        localStorage.setItem(key,btoa(String.fromCharCode(...stale)));
      },key);
      page.on('request', request => requests.push(request.url()));
      await page.locator('#rom').setInputFiles({name:'jmp-timing.ws',mimeType:'application/octet-stream',buffer:bytes});
      await page.waitForFunction(() => document.querySelector('#rom-status').textContent==='Running',null,{polling:25});
      for (let boot=1;boot<=2;boot++) {
        if (boot===2) await click('#reset');
        await step(0);
        await step(14); // one emulated frame, before the probe completes
        await click('#pause');
        assert.equal(await page.evaluate(key => {
          const data = atob(localStorage.getItem(key));
          return data.charCodeAt(0) | data.charCodeAt(1)<<8;
        },key),0,'stale completion invalidated before sampling finishes');
        await click('#pause');
        await step(0);
        for (let i=1;i<=40;i++) await step(i*100); // bounded 160 emulated frames
        await click('#pause');
        assert.equal(await page.locator('#error').textContent(),'');
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
          for (const [sample,low,high] of [[0,352,353],[1,368,369],[2,352,353],[3,368,369],[4,416,417],[5,432,433]]) {
            const i = 4+pass*30+sample*5;
            observations.push(word(i));
            assert.ok(word(i)>=low && word(i)<=high,`${model} boot ${boot} pass ${pass} sample ${sample}: ${word(i)} ticks`);
            assert.deepEqual([word(i+1),word(i+2),word(i+3),word(i+4)],[4096,4096,0,0x51a7],'CPU execution and destination counts, final CX and continuation');
          }
        }
        assert.ok(saved.slice(8+passes*60,512).every(value => value===0),'unused observations cleared');
        console.log(`${model} boot ${boot}: HBlank samples ${observations}; execution/destination counts 4096/4096, CX=0 and continuation checked`);
      }
      assert.equal(requests.length,0,'ROM/save handling stays local');
    } finally { await context.close(); }
  }
  console.log('Absolute JMP budgets: all three forms and even/odd destinations, counts/CX/continuation, all nine configurations, live modes and fresh restart passed');
}
