// SPDX-License-Identifier: CC0-1.0
// Independent absolute budgets: docs/shift-rotate-timing.md.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

export async function shiftTimingChecks(browser, base, errors) {
  const bytes = await readFile(new URL('../../fixtures/generated/shift-timing.ws',import.meta.url));
  const key = `wonderswan:save:v1:${createHash('sha256').update(bytes).digest('hex')}`;
  const failures = [];
  for (const model of ['mono','color','crystal']) {
    const context = await browser.newContext();
    await context.addInitScript(() => {
      // Control host RAF delivery to inspect startup invalidation through the
      // real player's pause/save path. Core execution stays unchanged.
      let next = 0;
      window.shiftRAF = new Map();
      window.requestAnimationFrame = callback => { window.shiftRAF.set(++next, callback); return next; };
      window.cancelAnimationFrame = id => window.shiftRAF.delete(id);
      localStorage.setItem('wonderswan:audio:v1','{"muted":true,"volume":70}');
    });
    const page = await context.newPage();
    page.on('pageerror', error => errors.push(error.message));
    const requests = [];
    const click = selector => page.locator(selector).evaluate(button => button.click());
    const step = time => page.evaluate(time => {
      const callbacks = [...window.shiftRAF.values()];
      window.shiftRAF.clear();
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
      await page.locator('#rom').setInputFiles({name:'shift-timing.ws',mimeType:'application/octet-stream',buffer:bytes});
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
          await page.locator('#rom').setInputFiles({name:'shift-timing.ws',mimeType:'application/octet-stream',buffer:bytes});
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
        for (let i=1;i<=350;i++) await step(i*100); // bounded 1400 emulated frames
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
          const records = [
            [0xa503,0xf803],[0xa540,0xf002],[0xa5c0,0xf802],[0xa540,0xf002],[0xa5ff,0xf003],
            [0x0003,0xf803],[0x4000,0xf002],[0xc000,0xf802],[0x4000,0xf002],[0xffff,0xf003],
            [0xa5c0,0xf003],[0xa540,0xf802],[0xa503,0xf002],[0xa540,0xf802],[0xa5ff,0xf003],
            [0xc000,0xf003],[0x4000,0xf802],[0x0003,0xf002],[0x4000,0xf802],[0xffff,0xf003],
            [0xa502,0xf803],[0xa540,0xf002],[0xa514,0xf002],[0xa540,0xf002],[0xa5f7,0xf003],
            [0x0002,0xf803],[0x4000,0xf002],[0x5000,0xf002],[0x4000,0xf002],[0xdfff,0xf003],
            [0xa540,0xf803],[0xa540,0xf802],[0xa528,0xf002],[0xa540,0xf802],[0xa5ef,0xf003],
            [0x4000,0xf803],[0x4000,0xf802],[0x000a,0xf002],[0x4000,0xf802],[0xfffb,0xf003],
            [0xa502,0xf803],[0xa540,0xf002],[0xa500,0xf046],[0xa540,0xf002],[0xa500,0xf046],
            [0x0002,0xf803],[0x4000,0xf006],[0x0000,0xf046],[0x4000,0xf006],[0x0000,0xf046],
            [0xa540,0xf803],[0xa540,0xf802],[0xa500,0xf046],[0xa540,0xf802],[0xa500,0xf046],
            [0x4000,0xf807],[0x4000,0xf806],[0x0000,0xf046],[0x4000,0xf806],[0x0000,0xf046],
            [0xa5c0,0xf087],[0xa540,0xf802],[0xa5ff,0xf087],[0xa540,0xf802],[0xa5ff,0xf087],
            [0xc000,0xf087],[0x4000,0xf806],[0xffff,0xf087],[0x4000,0xf806],[0xffff,0xf087],
          ];
          for (const [sample,[result,flags]] of records.entries()) {
            const profile = sample%5;
            const low = [128,136,136,140,140][profile], high = low+1;
            const i = 4+pass*490+sample*7;
            observations.push(word(i));
            if (word(i)<low || word(i)>high) failures.push(`${model} boot ${boot} pass ${pass} sample ${sample}: ${word(i)} vs ${low}..${high}`);
            assert.deepEqual([word(i+1),word(i+2),word(i+3),word(i+4),word(i+5),word(i+6)],
              [result,flags,1024,0,[1,0,31,32,255][profile],0x51a7],`sample ${sample}: CPU result, full FLAGS, execution/remaining/count and stack continuation`);
          }
        }
        assert.ok(saved.slice(8+passes*980,7168).every(value => value===0),'unused observations cleared');
        console.log(`${model} boot ${boot}: HBlank samples ${observations}; execution count 1024, remaining=0 and continuation checked`);
      }
      assert.equal(requests.length,0,'ROM/save handling stays local');
    } finally { await context.close(); }
  }
  assert.deepEqual(failures,[],`absolute shift budgets failed: ${failures.join('; ')}`);
  console.log('Absolute shift budgets: all seven operations, both widths, all count sources, masked-zero and large counts, results/full FLAGS/counts/stack continuation, all nine configurations, live modes and fresh restart passed');
}
