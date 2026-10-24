// SPDX-License-Identifier: CC0-1.0
// Independent absolute budgets: docs/return-timing.md.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

export async function returnTimingChecks(browser, base, errors) {
  const bytes = await readFile(new URL('../../fixtures/generated/return-timing.ws',import.meta.url));
  const key = `wonderswan:save:v1:${createHash('sha256').update(bytes).digest('hex')}`;
  const failures = [];
  for (const model of ['mono','color','crystal']) {
    const context = await browser.newContext();
    await context.addInitScript(() => {
      // Control host RAF delivery to inspect startup invalidation through the
      // real player's pause/save path. Core execution stays unchanged.
      let next = 0;
      window.returnRAF = new Map();
      window.requestAnimationFrame = callback => { window.returnRAF.set(++next, callback); return next; };
      window.cancelAnimationFrame = id => window.returnRAF.delete(id);
      localStorage.setItem('wonderswan:audio:v1','{"muted":true,"volume":70}');
    });
    const page = await context.newPage();
    page.on('pageerror', error => errors.push(error.message));
    const requests = [];
    const click = selector => page.locator(selector).evaluate(button => button.click());
    const step = time => page.evaluate(time => {
      const callbacks = [...window.returnRAF.values()];
      window.returnRAF.clear();
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
      await page.locator('#rom').setInputFiles({name:'return-timing.ws',mimeType:'application/octet-stream',buffer:bytes});
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
        for (let i=1;i<=100;i++) await step(i*100); // bounded 400 emulated frames
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
          for (const [sample,low,high] of [[0,400,401],[1,416,417],[2,480,481],[3,496,497],
            [4,480,481],[5,496,497],[6,592,593],[7,608,609],[8,576,577],[9,592,593]]) {
            const i = 4+pass*60+sample*6;
            observations.push(word(i));
            if (word(i)<low || word(i)>high) failures.push(`${model} boot ${boot} pass ${pass} sample ${sample}: ${word(i)} vs ${low}..${high}`);
            assert.deepEqual([word(i+1),word(i+2),word(i+3),word(i+4),word(i+5)],[4096,4096,0,0xf046,0x51a7],'CPU execution and destination counts, final CX, full FLAGS and frame/stack continuation');
          }
        }
        assert.ok(saved.slice(8+passes*120,1024).every(value => value===0),'unused observations cleared');
        console.log(`${model} boot ${boot}: HBlank samples ${observations}; execution/destination counts 4096/4096, CX=0 and continuation checked`);
      }
      assert.equal(requests.length,0,'ROM/save handling stays local');
    } finally { await context.close(); }
  }
  assert.deepEqual(failures,[],`absolute return budgets failed: ${failures.join('; ')}`);
  console.log('Absolute return budgets: all five forms and even/odd destinations, counts/CX/stack integrity/continuation, all nine configurations, live modes and fresh restart passed');
}
