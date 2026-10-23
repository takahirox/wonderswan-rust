// SPDX-License-Identifier: CC0-1.0
// CPU observations: 4096 extra bus clocks / 256 clocks per HBlank = 16 ticks.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

export async function wordTimingChecks(browser, base, errors) {
  const bytes = await readFile(new URL('../../fixtures/generated/word-timing.ws',import.meta.url));
  const key = `wonderswan:save:v1:${createHash('sha256').update(bytes).digest('hex')}`;
  for (const model of ['mono','color','crystal']) {
    const context = await browser.newContext();
    await context.addInitScript(() => localStorage.setItem('wonderswan:audio:v1','{"muted":true,"volume":70}'));
    const page = await context.newPage();
    page.on('pageerror', error => errors.push(error.message));
    try {
      await page.goto(base);
      await page.waitForFunction(() => !document.querySelector('#model').disabled);
      await page.locator('#model').selectOption(model);
      const requests = [];
      page.on('request', request => requests.push(request.url()));
      await page.locator('#rom').setInputFiles({name:'word-timing.ws',mimeType:'application/octet-stream',buffer:bytes});
      for (let boot=1;boot<=2;boot++) {
        if (boot === 2) await page.locator('#reset').click();
        await page.waitForFunction(({key,boot}) => {
          const saved = localStorage.getItem(key);
          if (!saved || document.querySelector('#rom-status').textContent !== 'Running') return false;
          const data = atob(saved);
          const word = i => data.charCodeAt(i) | data.charCodeAt(i+1) << 8;
          return word(0) === boot && word(2) !== 0;
        },{key,boot});
        assert.equal(await page.locator('#error').textContent(),'');
        await page.locator('#pause').click();
        const data = await page.evaluate(key => Array.from(atob(localStorage.getItem(key)),c => c.charCodeAt(0)),key);
        const word = i => data[i] | data[i+1] << 8;
        assert.equal(data.length,32768);
        assert.equal(word(0),boot,`${model}: fresh CPU boot`);
        assert.equal(word(2),0x600d,`${model}: cartridge timing self-check`);
        for (const group of [4,10,16,22]) {
          const byte = word(group), even = word(group+2), odd = word(group+4);
          assert.ok(even > 0,'loop actually ran');
          assert.ok(Math.abs(byte-even) <= 2,`${model} group ${group}: byte ${byte}, even ${even}`);
          assert.ok(odd-even >= 14 && odd-even <= 18,`${model} group ${group}: even ${even}, odd ${odd}`);
        }
      }
      assert.equal(requests.length,0,'ROM/save handling stays local');
    } finally { await context.close(); }
  }
  console.log('Word timing probe: byte/even/odd RAM reads/writes and valid internal IN/OUT, relative 16 +/-2 HBlank ticks, fresh CPU boot/results and restart passed on all three production Wasm models');
}
