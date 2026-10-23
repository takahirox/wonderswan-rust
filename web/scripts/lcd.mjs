// SPDX-License-Identifier: CC0-1.0
// Controlled RAF timestamps, real production Wasm and original CC0 CPU fixture.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

export async function lcdChecks(browser, base, errors) {
  const bytes = await readFile(new URL('../../fixtures/generated/lcd.ws', import.meta.url));
  const bindings = await readFile(new URL('../pkg/ws_web.js', import.meta.url), 'utf8');
  const wasm = await readFile(new URL('../pkg/ws_web_bg.wasm', import.meta.url));
  const key = `wonderswan:save:v1:${createHash('sha256').update(bytes).digest('hex')}`;
  const context = await browser.newContext();
  await context.addInitScript(() => {
    // Only time delivery is controlled. The player, cartridge CPU and all Wasm
    // execution are unchanged. Suppress periodic timers; exercise their callbacks
    // explicitly so diagnostics read actual core counters at each assertion.
    let id = 0;
    window.lcdRAF = new Map();
    window.requestAnimationFrame = callback => { window.lcdRAF.set(++id, callback); return id; };
    window.cancelAnimationFrame = id => window.lcdRAF.delete(id);
    window.lcdTimers = new Map();
    const interval = window.setInterval.bind(window);
    window.setInterval = (callback, delay, ...args) => {
      if (delay === 500 || delay === 1000) { window.lcdTimers.set(delay, callback); return ++id; }
      return interval(callback, delay, ...args);
    };
    localStorage.setItem('wonderswan:audio:v1', JSON.stringify({muted:true,volume:70}));
  });
  const page = await context.newPage();
  page.on('pageerror', error => errors.push(error.message));
  try {
    await page.goto(base);
    await page.waitForFunction(() => !document.querySelector('#model').disabled, null, {polling:25});
    // A second instance of the exact production adapter supplies independently
    // executed frame durations. It cannot inherit the player's cached duration.
    await page.evaluate(async ({bindings, wasm, rom}) => {
      const url = URL.createObjectURL(new Blob([bindings], {type:'text/javascript'}));
      try {
        const adapter = await import(url);
        await adapter.default({module_or_path:new Uint8Array(wasm)});
        const machine = new adapter.WebMachine();
        window.lcdReference = {machine,rom:new Uint8Array(rom),time:null,credit:0};
      } finally { URL.revokeObjectURL(url); }
    }, {bindings,wasm:Array.from(wasm),rom:Array.from(bytes)});
    const click = selector => page.locator(selector).evaluate(button => button.click());
    async function referenceReset(model, restart=false) {
      await page.evaluate(({model,restart}) => {
        const r = window.lcdReference;
        if (restart) r.machine.restart();
        else { r.machine.reset(model);r.machine.load_rom(r.rom); }
        r.time = null;r.credit = 0;
        document.querySelector('#debug-toggle').checked = true;
        document.querySelector('#debug-toggle').dispatchEvent(new Event('change'));
        document.querySelector('#debug-panel details').open = true;
      }, {model,restart});
    }
    let expectedError = '';
    async function step(time) {
      const result = await page.evaluate(time => {
        const r = window.lcdReference;
        const before = r.machine.frame_number();
        if (r.time !== null) r.credit += Math.min(100,time-r.time);
        r.time = time;
        let count = 0;
        while (count < 4) {
          const cost = 1000/r.machine.frame_rate();
          if (r.credit < cost) break;
          r.credit -= cost;
          r.machine.run_frame();r.machine.audio_samples();count++;
        }
        r.credit = Math.min(r.credit,1000/r.machine.frame_rate());
        const callbacks = [...window.lcdRAF.values()];window.lcdRAF.clear();
        callbacks.forEach(callback => callback(time));
        window.lcdTimers.get(500)();
        return {actual:Number(document.querySelector('#debug-frames').textContent),
          expected:r.machine.frame_number(),before,rate:r.machine.frame_rate(),
          pixel:Array.from(document.querySelector('#screen').getContext('2d').getImageData(0,0,1,1).data),
          error:document.querySelector('#error').textContent,queued:window.lcdRAF.size};
      }, time);
      assert.equal(result.error,expectedError);
      assert.equal(result.actual,result.expected,`controlled timestamp ${time}: current cadence must drive the player`);
      assert.ok(result.actual-result.before <= 4,'bounded catch-up');
      assert.equal(result.queued,1,'one pending animation callback');
      return result;
    }
    async function saved(model, total, switches) {
      const result = await page.evaluate(key => Array.from(atob(localStorage.getItem(key)), c => c.charCodeAt(0)),key);
      assert.deepEqual(result.slice(0,8),[0x0d,0x60,157,0,181,0,total,0],`${model}: CPU execution/raster markers`);
      assert.deepEqual(result.slice(8,11),[switches,0,0xf0]);
    }
    for (const model of ['mono','color','crystal']) {
      expectedError = '';
      await page.locator('#model').selectOption(model);
      await page.locator('#rom').setInputFiles({name:'lcd.ws',mimeType:'application/octet-stream',buffer:bytes});
      await page.waitForFunction(() => document.querySelector('#rom-status').textContent === 'Running',null,{polling:25});
      await referenceReset(model);
      let time = 0;
      await step(time);
      let result;
      // Includes initialization's shorter total and its runtime switch to 180.
      for (let i=0;i<30;i++) result = await step(time += 14);
      assert.equal(result.rate,3_072_000/46336);
      assert.deepEqual(result.pixel,model === 'mono' ? [170,170,170,255] : [0,255,0,255]);
      // Steady-state threshold and suspension vectors, delivered without sleeps.
      for (const delta of [0.001,15.082,0.001,29,1000,0,1,13,15,16]) await step(time += delta);
      await click('#pause');await saved(model,180,0);
      const paused = await page.locator('#debug-frames').textContent();
      await page.evaluate(time => {
        const callbacks=[...window.lcdRAF.values()];window.lcdRAF.clear();callbacks.forEach(cb=>cb(time));
        window.lcdTimers.get(500)();
      },time+5000);
      assert.equal(await page.locator('#debug-frames').textContent(),paused);
      await click('#pause');
      await page.evaluate(() => {window.lcdReference.time=null;window.lcdReference.credit=0;});
      await step(time += 5000); // resume cannot catch up paused time
      // Real input asks the cartridge to change timing during execution.
      await page.locator('#screen').evaluate(canvas => canvas.focus());
      await page.keyboard.down('ArrowRight');
      await page.evaluate(() => window.lcdReference.machine.set_buttons(32));
      for (let i=0;i<20;i++) result = await step(time += 14);
      assert.equal(result.rate,3_072_000/40192);
      await page.keyboard.up('ArrowRight');
      await page.evaluate(() => window.lcdReference.machine.set_buttons(0));
      for (let i=0;i<20;i++) result = await step(time += 14);
      assert.equal(result.rate,3_072_000/46336);
      await click('#pause');await saved(model,180,2);
      // Restart resets total/counter, then repeats the cartridge's own checks.
      await click('#reset');await referenceReset(model,true);
      await step(time += 5000);
      for (let i=0;i<25;i++) await step(time += 14);
      await click('#pause');await saved(model,180,0);
      // Invalid load preserves the running cartridge and resets pacing history.
      await click('#pause');
      await page.locator('#rom').setInputFiles({name:'bad.ws',mimeType:'application/octet-stream',buffer:Buffer.from([1,2,3])});
      await page.waitForFunction(() => document.querySelector('#error').textContent.includes('Could not load'),null,{polling:25});
      expectedError = await page.locator('#error').textContent();
      await page.evaluate(() => {window.lcdReference.time=null;window.lcdReference.credit=0;});
      await step(time += 5000);
      for (let i=0;i<5;i++) await step(time += 14);
      // Visibility cancels scheduling, requires explicit resume, and drops debt.
      await page.evaluate(() => {Object.defineProperty(document,'hidden',{configurable:true,value:true});document.dispatchEvent(new Event('visibilitychange'));});
      assert.equal(await page.evaluate(() => window.lcdRAF.size),0);
      await page.evaluate(() => {delete document.hidden;document.dispatchEvent(new Event('visibilitychange'));});
      assert.equal(await page.locator('#pause').textContent(),'Resume');
      await click('#pause');
      await page.evaluate(() => {window.lcdReference.time=null;window.lcdReference.credit=0;});
      await step(time += 5000);await step(time += 16);
      await click('#pause');
    }
    await page.evaluate(() => window.lcdReference.machine.free());
    console.log('LCD original CPU markers/pixels on all models, byte/word I/O, restart, live Wasm cadence, controlled RAF thresholds/catch-up, input timing changes, pause/visibility and failed-load recovery passed');
  } finally { await context.close(); }
}
