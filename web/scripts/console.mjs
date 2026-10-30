// SPDX-License-Identifier: CC0-1.0
// Real CPU/Wasm console persistence checks, invoked by the compatibility suite.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

export async function consoleChecks(browser, base, errors) {
  const probes = await Promise.all(['internal-eeprom','internal-eeprom-protected'].map(async name => {
    const bytes = await readFile(new URL(`../../fixtures/generated/${name}.ws`, import.meta.url));
    return { bytes, name, key:`wonderswan:save:v1:${createHash('sha256').update(bytes).digest('hex')}` };
  }));
  const rejected = await Promise.all(Array.from({length:8},async (_,i) => Array.from(
    await readFile(new URL(`../../fixtures/generated/internal-eeprom-rejected-${i+1}.ws`,import.meta.url)))));
  const key = model => `wonderswan:console:v1:${model}`;
  const context = await browser.newContext();
  await context.addInitScript(() => {
    // Suppress periodic persistence to prove the lifecycle flush paths themselves.
    const interval = window.setInterval.bind(window);
    window.setInterval = (callback, delay, ...args) => delay === 1000 ? 0 : interval(callback, delay, ...args);
    for (const [i,model] of ['mono','color','crystal'].entries()) {
      const key = `wonderswan:console:v1:${model}`;
      if (localStorage.getItem(key) !== null) continue;
      const bytes = new Uint8Array(model === 'mono' ? 128 : 2048).fill(255);
      bytes.fill(0,0x60);bytes[0]=0x78;bytes[1]=0x56;bytes[2]=(i+1)*10;bytes[3]=0;
      bytes[0x60]=0x34;bytes[0x61]=0x12;
      if (model !== 'mono') {bytes[2046]=0xcd;bytes[2047]=0xab;}
      localStorage.setItem(key,btoa(String.fromCharCode(...bytes)));
    }
  });
  let page;
  async function open(ctx = context) {
    page = await ctx.newPage();page.on('pageerror', error => errors.push(error.message));
    await page.goto(base);await ready();
  }
  async function ready() { await page.waitForFunction(() => !document.querySelector('#model').disabled); }
  async function success(model) {
    await page.waitForFunction(model => {
      const p = document.querySelector('#screen').getContext('2d').getImageData(0,0,1,1).data;
      const expected = model === 'mono' ? [170,170,170] : [0,255,0];
      return document.querySelector('#rom-status').textContent === 'Running' && expected.every((v,i) => p[i] === v);
    },model);
    assert.equal(await page.locator('#error').textContent(),'');
  }
  async function load(probe, model) {
    // Clear the old screen via the player's genuine load/restart path.
    await page.locator('#rom').setInputFiles({name:`${probe.name}.ws`,mimeType:'application/octet-stream',buffer:probe.bytes});
    await success(model);
  }
  async function pause() {
    if (await page.locator('#pause').textContent() === 'Pause') await page.locator('#pause').click();
  }
  async function saved(model, count) {
    const bytes = await page.evaluate(key => Array.from(atob(localStorage.getItem(key)), c => c.charCodeAt(0)),key(model));
    assert.equal(bytes.length,model === 'mono' ? 128 : 2048);
    assert.deepEqual(bytes.slice(0,4),[255,255,count,0]);
    assert.deepEqual(bytes.slice(0x5e,0x62),[0xaa,0x55,0x34,0x12]);
    if (model !== 'mono') assert.deepEqual(bytes.slice(2046),[0xcd,0xab]);
  }
  try {
    await open();
    // Exercise the length-checked thin API directly in Chromium using the exact
    // optimized Wasm and generated bindings consumed by the production build.
    const bindings = await readFile(new URL('../pkg/ws_web.js',import.meta.url),'utf8');
    const wasm = await readFile(new URL('../pkg/ws_web_bg.wasm',import.meta.url));
    await page.evaluate(async ({bindings,wasm,rom,rejected}) => {
      const url = URL.createObjectURL(new Blob([bindings],{type:'text/javascript'}));
      try {
        const adapter = await import(url);
        await adapter.default({module_or_path:new Uint8Array(wasm)});
        const machine = new adapter.WebMachine();
        const check = (value,why) => {if (!value) throw new Error(why);};
        for (const [i,model] of ['mono','color','crystal'].entries()) {
          machine.reset(model);
          const data = machine.console_data();
          check(data.length === (model === 'mono' ? 128 : 2048),'physical Wasm capacity');
          data[0]=0x78;data[1]=0x56;data[2]=i+1;data[3]=0;
          machine.load_console_data(data);
          machine.load_rom(new Uint8Array(rom));
          const save = new Uint8Array(32768).fill(0x77);
          machine.load_save(save);
          let failed = false;
          try {machine.load_console_data(new Uint8Array(data.length-1));} catch {failed=true;}
          check(failed,'Wasm rejects mismatched console import');
          check(machine.console_data().every((b,i) => b === data[i]),'failed import preserves console bytes');
          check(machine.save_data().every(b => b === 0x77),'console import preserves cartridge save');
          machine.run_frame();machine.run_frame();
          const completed = machine.console_data();
          check(completed[0] === 255 && completed[1] === 255,'Wasm CPU direct ERASE bytes');
          check(completed[2] === i+2 && completed[3] === 0,'Wasm CPU shared counter');
          check(completed.slice(0x60).every((b,i) => b === data[i+0x60]),'Wasm erase preserves settings');
          check(machine.save_data().every(b => b === 0x77),'Wasm erase preserves cartridge save');
          machine.restart();
          check(machine.console_data().every((b,i) => b === completed[i]),'Wasm restart retains completed erase');
        }
        for (const [i,model] of ['mono','color','crystal'].entries()) {
          machine.reset(model);
          const bytes = machine.console_data();
          check(bytes[0] === 255 && bytes[1] === 255 && bytes[2] === i+2,'Wasm unload/model isolation');
        }
        const messages = ['ERASE','ERASE','ERASE','WRAL','ERAL','command','format','busy'];
        for (const model of ['mono','color','crystal']) {
          for (const [i,rom] of rejected.entries()) {
            machine.reset(model);
            const expected = new Uint8Array(model === 'mono' ? 128 : 2048).fill(0x5a);
            machine.load_console_data(expected);machine.load_rom(new Uint8Array(rom));
            machine.load_save(new Uint8Array(32768).fill(0x77));
            let error = '';
            try {machine.run_frame();} catch (e) {error=String(e);}
            check(error.includes(messages[i]),`Wasm CPU diagnostic ${i+1}: ${error}`);
            expected[0]=0x34;expected[1]=0x12;
            check(machine.console_data().every((b,i) => b === expected[i]),'rejected command preserves all completed bytes');
            check(machine.save_data().every(b => b === 0x77),'rejected command preserves cartridge save');
            machine.restart();
            check(machine.console_data().every((b,i) => b === expected[i]),'restart cancels busy erase');
          }
        }
        machine.free();
      } finally {URL.revokeObjectURL(url);}
    },{bindings,wasm:Array.from(wasm),rom:Array.from(probes[0].bytes),rejected});
    const requests = [];
    page.on('request', request => requests.push(request.url()));
    await load(probes[0],'crystal');
    await load(probes[1],'crystal'); // cross-cartridge shared console data
    await page.locator('#reset').click();await success('crystal');
    // Switching while running must flush the outgoing console before replacement.
    await page.locator('#model').selectOption('color');await success('color');
    await saved('crystal',33);
    await load(probes[0],'color');
    await page.locator('#model').selectOption('mono');await success('mono');
    await saved('color',22);
    await load(probes[1],'mono');
    await page.locator('#model').selectOption('crystal');await success('crystal');
    await saved('mono',12);
    await pause();await saved('crystal',34);
    assert.deepEqual(requests,[],'console data and selected cartridges stay local');
    await page.reload();await ready();await saved('crystal',34);
    await load(probes[0],'crystal');await pause();await saved('crystal',35);
    // pagehide must flush without relying on the periodic timer or pause button.
    await page.locator('#pause').click();await success('crystal');
    await page.locator('#reset').click();await success('crystal');
    await page.evaluate(() => window.dispatchEvent(new Event('pagehide')));
    await saved('crystal',36);
    await page.close();await open();await saved('crystal',36);
    await load(probes[1],'crystal');await pause();await saved('crystal',37);
    await page.locator('#model').selectOption('color');await success('color');await pause();await saved('color',23);
    await page.locator('#model').selectOption('mono');await success('mono');await pause();await saved('mono',13);
    assert.equal(await page.evaluate(() => Object.keys(localStorage).filter(key => key.startsWith('wonderswan:console:')).length),3);
  } finally { await context.close(); }

  // Malformed records are preserved independently from valid cartridge saves.
  for (const value of ['invalid!',btoa('wrong length')]) {
    const bad = await browser.newContext();
    await bad.addInitScript(({value,key}) => localStorage.setItem(key,value),{value,key:key('crystal')});
    try {
      await open(bad);await load(probes[0],'crystal');await pause();
      assert.match(await page.locator('#console-notice').textContent(),/could not be read/);
      assert.equal(await page.evaluate(key => localStorage.getItem(key),key('crystal')),value);
      assert.equal(await page.evaluate(key => atob(localStorage.getItem(key)).length,probes[0].key),32768);
      assert.equal(await page.locator('#notice').textContent(),'');
    } finally { await bad.close(); }
  }
  // Denied console reads and quota failures leave cartridge persistence usable.
  for (const mode of ['denied','full']) {
    const failed = await browser.newContext();
    await failed.addInitScript(mode => {
      const get = Storage.prototype.getItem, set = Storage.prototype.setItem;
      Storage.prototype.getItem = function(key) {
        if (mode === 'denied' && key.startsWith('wonderswan:console:')) throw new DOMException('simulated console denial','SecurityError');
        return get.call(this,key);
      };
      Storage.prototype.setItem = function(key,value) {
        if (key.startsWith('wonderswan:console:')) throw new DOMException('simulated console quota','QuotaExceededError');
        return set.call(this,key,value);
      };
    },mode);
    try {
      await open(failed);await load(probes[0],'crystal');await pause();
      assert.match(await page.locator('#console-notice').textContent(),mode === 'denied' ? /could not be read/ : /unavailable or full/);
      assert.equal(await page.evaluate(key => atob(localStorage.getItem(key)).length,probes[0].key),32768);
      await page.locator('#reset').click();await success('crystal');
      assert.equal(await page.locator('#notice').textContent(),'');
    } finally { await failed.close(); }
  }
  console.log('Internal EEPROM CPU/Wasm probes: word-zero ERASE, nonzero/bulk/malformed/busy rejection, all three models, settings/status/locking/protection, cross-cartridge data, model isolation, lifecycle flush, erased bytes through reload/reopen, corrupt/denied/full storage and independent cartridge saves passed');
}
