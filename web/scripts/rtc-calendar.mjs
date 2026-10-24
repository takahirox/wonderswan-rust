// SPDX-License-Identifier: CC0-1.0
// Original CPU-written calendar vectors through the real production player.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

export async function rtcCalendarChecks(browser, base, errors) {
  const bytes = await readFile(new URL('../../fixtures/generated/rtc-calendar.ws', import.meta.url));
  const hash = createHash('sha256').update(bytes).digest('hex');
  const key = `wonderswan:save:v1:${hash}`;
  const clockKey = `wonderswan:rtc:v1:${hash}`;
  const click = (page, selector) => page.locator(selector).evaluate(button => button.click());
  const closeSettings = page => page.evaluate(() => new Promise(resolve => {
    // Native close is asynchronous. Wait for the player's close listener before
    // selecting another ROM, otherwise it can pause that newly loaded cartridge.
    document.querySelector('#help').addEventListener('close',resolve,{once:true});
    document.querySelector('#help-close').click();
  }));
  const step = (page, time) => page.evaluate(time => {
    const callbacks = [...window.rtcRAF.values()]; window.rtcRAF.clear();
    callbacks.forEach(callback => callback(time));
  }, time);
  const open = async (context, now = 1000) => {
    const page = await context.newPage();
    page.on('pageerror', error => errors.push(error.message));
    await page.addInitScript(now => {
      window.rtcNow = now;
      Date.now = () => window.rtcNow;
      let next = 0; window.rtcRAF = new Map();
      window.requestAnimationFrame = callback => { window.rtcRAF.set(++next,callback); return next; };
      window.cancelAnimationFrame = id => window.rtcRAF.delete(id);
      localStorage.setItem('wonderswan:audio:v1', '{"muted":true,"volume":70}');
    },now);
    await page.goto(base);
    await page.waitForFunction(() => !document.querySelector('#model').disabled,null,{polling:25});
    return page;
  };
  const load = async (page, rom = bytes) => {
    await page.locator('#rom').setInputFiles({name:'rtc-calendar.ws',mimeType:'application/octet-stream',buffer:rom});
    try {
      await page.waitForFunction(() => document.querySelector('#rom-status').textContent === 'Running',null,{polling:25});
    } catch (error) {
      const state = await page.evaluate(() => ({status:document.querySelector('#rom-status').textContent,error:document.querySelector('#error').textContent,hidden:document.hidden,dialog:document.querySelector('#help').open}));
      throw new Error(`RTC fixture load failed: ${JSON.stringify(state)}`,{cause:error});
    }
  };
  const execute = async page => {
    await step(page,0); await step(page,100); // four genuine emulator frames
    await click(page,'#pause');
    assert.equal(await page.locator('#error').textContent(),'');
    return page.evaluate(key => Array.from(atob(localStorage.getItem(key)), c=>c.charCodeAt(0)),key);
  };
  const check = (data, model, initial, boot = 1) => {
    assert.equal(data[0x100] | data[0x101] << 8,boot,"fresh CPU boot counter");
    assert.deepEqual(data.slice(0,2),[0x0d,0x63],'fresh CPU completed');
    assert.deepEqual(data.slice(2,10),initial,'CPU read retained state before writes');
    for (const [phase,mode] of (model === 'mono' ? [0] : [0,0x80,0,0xc0,0,0xe0,0]).entries()) {
      assert.deepEqual(data.slice(10+phase*12,22+phase*12),
        [mode,0x6a,0x24,2,0x28,3,0xa3,0x59,0x58,0x92,0x34,0x56],`${model} phase ${phase}`);
    }
  };
  let validClockRecord;
  const cold = [0x82,0,1,1,0,0,0,0];
  const retained = [0x6a,0x24,2,0x28,3,0x92,0x35,0x57];
  for (const model of ['mono','color','crystal']) {
    const context = await browser.newContext();
    let page = await open(context);
    try {
      await page.locator('#model').selectOption(model);
      // Stale success in SRAM cannot satisfy CPU readback assertions.
      await page.evaluate(key => {
        const stale = new Uint8Array(32768).fill(0x6d); stale.set([0x0d,0x63]); stale[0x100]=0; stale[0x101]=0;
        localStorage.setItem(key,btoa(String.fromCharCode(...stale)));
      },key);
      await load(page);
      check(await execute(page),model,cold);
      await page.evaluate(() => {window.rtcNow = 62000;});
      await click(page,'#reset');
      check(await execute(page),model,retained,2);
      // A different physical model shares this cartridge clock record.
      const target = model === 'mono' ? 'color' : 'mono';
      await page.evaluate(() => {window.rtcNow = 123000;});
      await page.locator('#model').selectOption(target);
      check(await execute(page),target,retained,3);
      await page.close();
      page = await open(context,184000);
      await page.locator('#model').selectOption(model);
      await load(page);
      check(await execute(page),model,retained,4);
      const stored = await page.evaluate(key=>localStorage.getItem(key),clockKey);
      assert.ok(stored);
      validClockRecord = stored;
      // Explicit absent load never replaces the equipped clock record.
      await click(page,'#settings');
      await page.locator('#rtc-mode').selectOption('absent');
      await closeSettings(page);
      await load(page); await execute(page);
      assert.equal(await page.evaluate(key=>localStorage.getItem(key),clockKey),stored);
      // Cartridge identity is raw SHA-256, never a filename or console model.
      const other = Buffer.from(bytes); other[123] ^= 1;
      let sum=0;for (const b of other.subarray(0,-2)) sum=(sum+b)&65535;
      other.writeUInt16LE(sum,other.length-2);
      await click(page,'#settings');await page.locator('#rtc-mode').selectOption('auto');await closeSettings(page);
      await load(page,other);
      await step(page,0);await step(page,100);await click(page,'#pause');
      const otherKey=`wonderswan:save:v1:${createHash('sha256').update(other).digest('hex')}`;
      const isolated=await page.evaluate(key=>Array.from(atob(localStorage.getItem(key)),c=>c.charCodeAt(0)),otherKey);
      check(isolated,model,cold);
      // A failed target load cannot replace either clock slot with cold data.
      const before=await page.evaluate(()=>Object.fromEntries(Object.entries(localStorage).filter(([k])=>k.startsWith('wonderswan:rtc:'))));
      await page.locator('#rom').setInputFiles({name:'bad.ws',mimeType:'application/octet-stream',buffer:Buffer.from([1,2,3])});
      await page.waitForFunction(()=>document.querySelector('#error').textContent.includes('Could not load'),null,{polling:25});
      assert.deepEqual(await page.evaluate(()=>Object.fromEntries(Object.entries(localStorage).filter(([k])=>k.startsWith('wonderswan:rtc:')))),before);
    } finally {await context.close();}
  }
  // Malformed envelopes, unsupported versions and validated-core import errors
  // must preserve the original record even after the fixture writes a good clock.
  for (const record of ['broken',JSON.stringify({version:2,data:'AA=='}),JSON.stringify({version:1,data:btoa('bad')})]) {
    const context=await browser.newContext(); const page=await open(context);
    try {
      await page.evaluate(({clockKey,record})=>localStorage.setItem(clockKey,record),{clockKey,record});
      await load(page);check(await execute(page),'crystal',cold);
      assert.equal(await page.evaluate(k=>localStorage.getItem(k),clockKey),record);
      assert.match(await page.locator('#rtc-notice').textContent(),/could not be restored/);
    } finally {await context.close();}
  }
  const denied=await browser.newContext();const page=await open(denied);
  try {
    await page.evaluate(()=>{
      const get=Storage.prototype.getItem,set=Storage.prototype.setItem;
      Storage.prototype.getItem=function(k){if(k.startsWith('wonderswan:rtc:'))throw new Error('denied');return get.call(this,k);};
      Storage.prototype.setItem=function(k,v){if(k.startsWith('wonderswan:rtc:'))throw new Error('denied');return set.call(this,k,v);};
    });
    await load(page);check(await execute(page),'crystal',cold);
    assert.match(await page.locator('#rtc-notice').textContent(),/could not be restored/);
  } finally {await denied.close();}

  const full=await browser.newContext();const fullPage=await open(full);
  try {
    await fullPage.evaluate(({clockKey,record})=>{
      localStorage.setItem(clockKey,record);
      const set=Storage.prototype.setItem;
      Storage.prototype.setItem=function(k,v){if(k.startsWith('wonderswan:rtc:'))throw new Error('full');return set.call(this,k,v);};
    },{clockKey,record:validClockRecord});
    await load(fullPage);await execute(fullPage);
    assert.equal(await fullPage.evaluate(k=>localStorage.getItem(k),clockKey),validClockRecord);
    assert.match(await fullPage.locator('#rtc-notice').textContent(),/unavailable or full/);
  } finally {await full.close();}

  // Validate the thin API using the exact release Wasm and bindings.
  const context=await browser.newContext();const adapterPage=await open(context);
  try {
    const bindings=await readFile(new URL('../pkg/ws_web.js',import.meta.url),'utf8');
    const wasm=await readFile(new URL('../pkg/ws_web_bg.wasm',import.meta.url));
    await adapterPage.evaluate(async ({bindings,wasm,rom})=>{
      const url=URL.createObjectURL(new Blob([bindings],{type:'text/javascript'}));
      try {
        const adapter=await import(url);await adapter.default({module_or_path:new Uint8Array(wasm)});
        const m=new adapter.WebMachine();m.load_rom(new Uint8Array(rom));m.sync_rtc(1000);m.run_frame();
        const data=m.rtc_data();const same=()=>m.rtc_data().every((v,i)=>v===data[i]);
        const check=(v,why)=>{if(!v)throw new Error(why);};
        for(const now of [-1,NaN,Infinity,0.1,Number.MAX_SAFE_INTEGER+1]) {
          let failed=false;try{m.sync_rtc(now);}catch{failed=true;}check(failed&&same(),'invalid clock preserves RTC');
        }
        for(const corrupt of [data.slice(1),new Uint8Array(data.length),Uint8Array.from(data,(v,i)=>i===5?13:v)]) {
          let failed=false;try{m.load_rtc_data(corrupt);}catch{failed=true;}check(failed&&same(),'failed import preserves RTC');
        }
        const frame=m.frame_number();m.sync_rtc(1999);m.sync_rtc(500);m.sync_rtc(1999);
        check(m.frame_number()===frame,'host time does not execute frames');
        const partial=m.rtc_data();m.load_rtc_data(partial);m.restart();check(m.rtc_data().every((v,i)=>v===partial[i]),'import/restart retains fraction');
        m.configure_rtc('absent');check(m.rtc_data().length===0,'Wasm absent override');
        let failed=false;try{m.load_rtc_data(data);}catch{failed=true;}check(failed,'absent import rejected');
        m.free();
      } finally {URL.revokeObjectURL(url);}
    },{bindings,wasm:Array.from(wasm),rom:Array.from(bytes)});
  } finally {await context.close();}
  console.log('RTC calendar: CPU-written status/date/time on all models/modes, restart/model/reopen/offline retention, raw-ROM isolation, absent non-destruction, malformed/unsupported/failed imports, denied storage, failed loads and validated Wasm APIs passed');
}
