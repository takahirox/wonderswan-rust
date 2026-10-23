// SPDX-License-Identifier: CC0-1.0
// Original CPU OUT/IN probe through the production player and compiled adapter.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

function markers(bytes) {
  const word = offset => bytes[offset] | bytes[offset+1] << 8;
  assert.equal(word(0),0x600d,'CPU self-check passes');
  assert.equal(word(2),1,'clearing OUT/IN completed');
  assert.ok(word(4) >= 32 && word(4) <= 1536,'CPU observes fast frequency progression');
  assert.equal(word(6),0x100,'clearing restores ordinary short-interval stability');
  assert.equal(word(8),0x101,'ordinary clock still produces frequency updates');
  assert.equal(word(10),0,'initial/restarted control reads zero');
  assert.equal(word(12),2,'CPU leaves FAST set to exercise next restart');
}
function pcm(left,right) {
  assert.equal(left.length,right.length);
  assert.ok(left.includes(0) && left.includes(0.2197265625));
  assert.ok(left.every((v,i) => [0,0.2197265625].includes(v) && v === right[i]));
  const edges = left.slice(1).filter((v,i) => v !== left[i]).length;
  assert.ok(edges >= 38 && edges <= 40,`1500 Hz square in 318 samples: ${edges} edges`);
}

export async function sweepChecks(browser, base, errors, observeAudio) {
  const bytes = await readFile(new URL('../../fixtures/generated/sweep.ws',import.meta.url));
  const bindings = await readFile(new URL('../pkg/ws_web.js',import.meta.url),'utf8');
  const wasm = await readFile(new URL('../pkg/ws_web_bg.wasm',import.meta.url));
  const key = `wonderswan:save:v1:${createHash('sha256').update(bytes).digest('hex')}`;
  const context = await browser.newContext();
  await context.addInitScript(observeAudio);
  const page = await context.newPage();
  page.on('pageerror', error => errors.push(error.message));
  const requests = [];
  page.on('request', request => requests.push(request.method()));
  try {
    await page.goto(base);
    await page.waitForFunction(() => !document.querySelector('#model').disabled);
    for (const model of ['mono','color','crystal']) {
      await page.locator('#model').selectOption(model);
      const chooser = page.waitForEvent('filechooser');
      await page.locator('#load-rom').click();
      await (await chooser).setFiles({name:'sweep.ws',mimeType:'application/octet-stream',buffer:bytes});
      for (let iteration=0;iteration<2;iteration++) {
        const before = await page.evaluate(() => window.audioProbe.buffers);
        if (iteration) {
          await page.locator('#reset').click();
        }
        await page.waitForFunction(({model,before}) => {
          const probe = window.audioProbe, b = probe.lastBuffer;
          const pixel = document.querySelector('#screen').getContext('2d').getImageData(0,0,1,1).data;
          const expected = model === 'mono' ? [170,170,170] : [0,255,0];
          return document.querySelector('#rom-status').textContent === 'Running'
            && expected.every((v,i) => pixel[i] === v)
            && probe.buffers > before+3 && b && b.left.includes(0.2197265625);
        },{model,before});
        assert.equal(await page.locator('#error').textContent(),'');
        const buffer = await page.evaluate(() => window.audioProbe.lastBuffer);
        assert.equal(buffer.rate,24000);
        assert.equal(buffer.length,318);
        pcm(buffer.left,buffer.right);
        await page.locator('#pause').click(); // flush actual cartridge SRAM
        await page.waitForFunction(key => localStorage.getItem(key) !== null,key);
        const saved = await page.evaluate(key => Array.from(atob(localStorage.getItem(key)),c=>c.charCodeAt(0)),key);
        assert.equal(saved.length,32768);
        markers(saved);
        assert.equal(await page.evaluate(() => window.audioProbe.active),0);
        if (iteration === 0) await page.locator('#pause').click();
      }
    }
    // The same production adapter also exposes the drain/reset contract. This
    // second real machine runs the cartridge CPU; no register/PCM replacement.
    const results = await page.evaluate(async ({bindings,wasm,rom}) => {
      const url = URL.createObjectURL(new Blob([bindings],{type:'text/javascript'}));
      try {
        const adapter = await import(url);
        await adapter.default({module_or_path:new Uint8Array(wasm)});
        const m = new adapter.WebMachine();
        const results = [];
        try {
          for (const model of ['mono','color','crystal']) {
            m.reset(model);m.load_rom(new Uint8Array(rom));
            const runs = [];
            for (let i=0;i<2;i++) {
              const emptyStart = m.audio_samples().length;
              m.run_frame();
              const first = Array.from(m.audio_samples());
              const emptyDrain = m.audio_samples().length;
              m.run_frame();
              const steady = Array.from(m.audio_samples());
              const saved = Array.from(m.save_data().slice(0,14));
              m.restart();
              runs.push({first,steady,saved,emptyStart,emptyDrain,emptyRestart:m.audio_samples().length});
            }
            results.push({model,runs});
          }
        } finally { m.free(); }
        return results;
      } finally { URL.revokeObjectURL(url); }
    },{bindings,wasm:Array.from(wasm),rom:Array.from(bytes)});
    for (const {runs} of results) {
      assert.deepEqual(runs[0].first,runs[1].first,'restart reproduces first-frame PCM');
      for (const run of runs) {
        markers(run.saved);
        assert.equal(run.emptyStart,0);assert.equal(run.emptyDrain,0);assert.equal(run.emptyRestart,0);
        assert.equal(run.steady.length,636);
        pcm(run.steady.filter((_,i)=>i%2===0),run.steady.filter((_,i)=>i%2===1));
      }
    }
    assert.ok(requests.every(method => method === 'GET'),'ROM/save bytes remain browser-local');
    console.log('Original port-95 CPU OUT/IN probe: all models, clearing, fast/ordinary progression, SRAM markers, real PCM, restart/control reset and Wasm empty-after-drain passed');
  } finally { await context.close(); }
}
