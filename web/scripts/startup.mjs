// SPDX-License-Identifier: CC0-1.0
// Original incoming-register probe through the production player and Wasm CPU.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

// Literal expectations independent of the Rust handoff implementation.
const models = [
  {name:'mono',ds:0xff00,a0:0x8d},
  {name:'color',ds:0xfe00,a0:0x8f},
  {name:'crystal',ds:0xfe00,a0:0x8f},
];
function markers(bytes, model, count) {
  const word = offset => bytes[offset] | bytes[offset+1] << 8;
  assert.equal(word(0),count,'retained cartridge boot count');
  assert.equal(word(2),0x2000,'CPU-captured incoming SP');
  assert.equal(word(4),model.ds,'CPU-captured incoming DS by physical console');
  assert.equal(word(6),0x600d,'CPU literal self-check passes');
  assert.equal(word(8),model.a0,'header-seeded A0 remains intact');
  assert.equal(word(10),0xf000,'normal footer jump reaches final mapped bank');
}

export async function startupChecks(browser, base, errors) {
  const probes = await Promise.all(['mono-common','mono-2003','color-common','color-2003'].map(async name => {
    const bytes = await readFile(new URL(`../../fixtures/generated/startup-${name}.ws`,import.meta.url));
    return {name,bytes,colorOnly:name.startsWith('color'),
      key:`wonderswan:save:v1:${createHash('sha256').update(bytes).digest('hex')}`};
  }));
  const bindings = await readFile(new URL('../pkg/ws_web.js',import.meta.url),'utf8');
  const wasm = await readFile(new URL('../pkg/ws_web_bg.wasm',import.meta.url));
  const context = await browser.newContext();
  const page = await context.newPage();
  page.on('pageerror', error => errors.push(error.message));
  const requests = [];
  try {
    await page.goto(base);
    await page.waitForFunction(() => !document.querySelector('#model').disabled);
    await page.locator('#model').selectOption('crystal');
    page.on('request', request => requests.push(request.url()));
    async function completed(probe, model, count) {
      // Production periodic persistence exposes actual CPU-written SRAM; wait
      // for a fresh boot counter so an old paused save cannot satisfy the check.
      await page.waitForFunction(({key,count}) => {
        const data = localStorage.getItem(key);
        if (!data) return false;
        const bytes = atob(data);
        return (bytes.charCodeAt(0) | bytes.charCodeAt(1) << 8) === count;
      },{key:probe.key,count});
      assert.equal(await page.locator('#error').textContent(),'');
      assert.equal(await page.locator('#rom-status').textContent(),'Running');
      await page.locator('#pause').click();
      const saved = await page.evaluate(key => Array.from(atob(localStorage.getItem(key)),c=>c.charCodeAt(0)),probe.key);
      assert.equal(saved.length,32768);
      markers(saved,model,count);
    }
    const counts = new Map();
    // Two passes replace every running cartridge and verify retained save identity.
    for (let pass=0;pass<2;pass++) {
      for (const probe of probes) {
        const supported = models.filter(model => !probe.colorOnly || model.name !== 'mono');
        // Each preceding iteration ends on Crystal, compatible with both headers.
        const chooser = page.waitForEvent('filechooser');
        await page.locator('#load-rom').click();
        await (await chooser).setFiles({name:`startup-${probe.name}.ws`,mimeType:'application/octet-stream',buffer:probe.bytes});
        let count = (counts.get(probe.name) ?? 0)+1;
        await completed(probe,models[2],count);
        await page.locator('#reset').click(); // player's same-model restart path
        await completed(probe,models[2],++count);
        for (const model of supported) {
          await page.locator('#model').selectOption(model.name);
          await completed(probe,model,++count);
          await page.locator('#reset').click();
          await completed(probe,model,++count);
        }
        counts.set(probe.name,count);
      }
    }
    assert.equal(requests.length,0,'local cartridges and saves make no requests');

    // Exercise the actual compiled production adapter's restart API and every
    // directed compatible model transition, separately from the player's API.
    const results = await page.evaluate(async ({bindings,wasm,probes}) => {
      const url = URL.createObjectURL(new Blob([bindings],{type:'text/javascript'}));
      try {
        const adapter = await import(url);
        await adapter.default({module_or_path:new Uint8Array(wasm)});
        const results = [];
        for (const probe of probes) {
          const supported = probe.colorOnly ? ['color','crystal'] : ['mono','color','crystal'];
          for (const model of supported) {
            const m = new adapter.WebMachine();
            try {
              let count = 0;
              function run(model) {
                m.run_frame();
                results.push({model,count:++count,saved:Array.from(m.save_data().slice(0,12))});
              }
              m.reset(model);m.load_rom(new Uint8Array(probe.bytes));run(model);
              m.restart();run(model);
              m.change_model(model);run(model);
              for (const replacement of probes) {
                if (replacement.colorOnly && m.model() === 'mono') continue;
                m.load_rom(new Uint8Array(replacement.bytes));
                count = 0;run(m.model());
              }
              m.load_rom(new Uint8Array(probe.bytes));count = 0;run(model);
              for (const next of supported) {
                m.change_model(model);run(model);
                m.change_model(next);run(next);
              }
            } finally { m.free(); }
          }
        }
        return results;
      } finally { URL.revokeObjectURL(url); }
    },{bindings,wasm:Array.from(wasm),probes:probes.map(p => ({colorOnly:p.colorOnly,bytes:Array.from(p.bytes)}))});
    for (const result of results) markers(result.saved,models.find(m => m.name === result.model),result.count);
    console.log('Incoming SP/DS CPU probe: both headers/mappers, physical models, load, restart, replacement, same-model and all compatible directed Wasm transitions passed');
  } finally { await context.close(); }
}
