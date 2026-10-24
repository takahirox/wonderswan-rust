// SPDX-License-Identifier: CC0-1.0
// Independent absolute budgets: docs/multiplication-timing.md.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

export async function multiplyTimingChecks(browser, base, errors) {
  const bytes = await readFile(new URL('../../fixtures/generated/multiply-timing.ws',import.meta.url));
  const key = `wonderswan:save:v1:${createHash('sha256').update(bytes).digest('hex')}`;
  const failures = [];
  for (const model of ['mono','color','crystal']) {
    const context = await browser.newContext();
    await context.addInitScript(() => {
      // Control host RAF delivery to inspect startup invalidation through the
      // real player's pause/save path. Core execution stays unchanged.
      let next = 0;
      window.multiplyRAF = new Map();
      window.requestAnimationFrame = callback => { window.multiplyRAF.set(++next, callback); return next; };
      window.cancelAnimationFrame = id => window.multiplyRAF.delete(id);
      localStorage.setItem('wonderswan:audio:v1','{"muted":true,"volume":70}');
    });
    const page = await context.newPage();
    page.on('pageerror', error => errors.push(error.message));
    const requests = [];
    const click = selector => page.locator(selector).evaluate(button => button.click());
    const step = time => page.evaluate(time => {
      const callbacks = [...window.multiplyRAF.values()];
      window.multiplyRAF.clear();
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
      await page.locator('#rom').setInputFiles({name:'multiply-timing.ws',mimeType:'application/octet-stream',buffer:bytes});
      await page.waitForFunction(() => document.querySelector('#rom-status').textContent==='Running',null,{polling:25});
      for (let boot=1;boot<=2;boot++) {
        if (boot===2) {
          assert.equal(requests.length,0,'first boot ROM/save handling stays local');
          // Restore poisoned retained observations through the real local save path,
          // then restart before any RAF can execute the newly loaded cartridge.
          await page.reload();
          await page.waitForFunction(() => !document.querySelector('#model').disabled,null,{polling:25});
          await page.locator('#model').selectOption(model);
          await page.evaluate(key => {
            const stale = new Uint8Array(32768).fill(0x6d);
            stale.set([0x0d,0x60,1,0]);
            localStorage.setItem(key,btoa(String.fromCharCode(...stale)));
          },key);
          requests.length=0; // Page assets are outside local ROM/save handling.
          await page.locator('#rom').setInputFiles({name:'multiply-timing.ws',mimeType:'application/octet-stream',buffer:bytes});
          await page.waitForFunction(() => document.querySelector('#rom-status').textContent==='Running',null,{polling:25});
          await click('#reset');
        }
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
          const unsigned = model==='mono' ? 0xf002 : 0xf042;
          const records = [
            [6,0x5aa5,unsigned,152],[6,0x5aa5,unsigned,156],[6,0x5aa5,unsigned,156],
            [6,0,unsigned,152],[6,0,unsigned,156],[6,0,unsigned,160],
            [0xffeb,0x5aa5,0xf042,152],[0xffeb,0x5aa5,0xf042,156],[0xffeb,0x5aa5,0xf042,156],
            [0xffeb,0xffff,0xf042,152],[0xffeb,0xffff,0xf042,156],[0xffeb,0xffff,0xf042,160],
            [0x0303,0x5aa5,0xf042,156],[0x0303,0x5aa5,0xf042,160],[0x0303,0x5aa5,0xf042,164],
            [0x0180,0x5aa5,0xf042,156],[0x0180,0x5aa5,0xf042,160],[0x0180,0x5aa5,0xf042,164],
          ];
          for (const [sample,[result,highProduct,flags,low]] of records.entries()) {
            const profile = sample%3;
            const high = low+1;
            const source = profile===0 ? [2,2,7,7,0xfffd,0xfffd][Math.floor(sample/3)] : 0x3200+Number(profile===2);
            const i = 4+pass*144+sample*8;
            observations.push(word(i));
            if (word(i)<low || word(i)>high) failures.push(`${model} boot ${boot} pass ${pass} sample ${sample}: ${word(i)} vs ${low}..${high}`);
            assert.deepEqual([word(i+1),word(i+2),word(i+3),word(i+4),word(i+5),word(i+6),word(i+7)],
              [result,highProduct,flags,1024,0,source,0x51a7],`sample ${sample}: CPU product, full FLAGS, execution/remaining/source and stack continuation`);
          }
        }
        assert.ok(saved.slice(8+passes*288,7168).every(value => value===0),'unused observations cleared');
        console.log(`${model} boot ${boot}: HBlank samples ${observations}; execution count 1024, remaining=0 and continuation checked`);
      }
      assert.equal(requests.length,0,'ROM/save handling stays local');
    } finally { await context.close(); }
  }
  assert.deepEqual(failures,[],`absolute multiply budgets failed: ${failures.join('; ')}`);
  console.log('Absolute multiply budgets: all six forms, registers and even/odd RAM, products/full FLAGS/counts/stack continuation, all nine configurations, live modes and fresh restart passed');
}
