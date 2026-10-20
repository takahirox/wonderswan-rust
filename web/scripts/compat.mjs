// SPDX-License-Identifier: CC0-1.0
// Production-preview checks using only this repository's original CC0 probes.
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
const base = process.env.WS_PREVIEW_URL ?? 'http://127.0.0.1:4173/wonderswan-rust/';
const probes = await Promise.all(['sram32-common','sram128-2003','eeprom128','eeprom1024','eeprom2048-packed'].map(async name => {
  const bytes = await readFile(new URL(`../../fixtures/generated/${name}.ws`, import.meta.url));
  return { name, bytes, key:`wonderswan:save:v1:${createHash('sha256').update(bytes).digest('hex')}` };
}));
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext();
let page = await context.newPage();
const errors = [];
const requests = [];
page.on('pageerror', e => errors.push(e.message));
page.on('request', r => requests.push(r.url()));
// Observe real Web Audio nodes/buffers while letting them play normally.
await context.addInitScript(() => {
  window.audioProbe = { buffers:0, nonzero:0, active:0, peak:0, contexts:0, resumes:0, suspends:0 };
  const Native = window.AudioContext;
  if (!Native) return;
  window.AudioContext = class extends Native {
    constructor(...args) { super(...args); window.audioProbe.contexts++; }
    resume() { window.audioProbe.resumes++; return super.resume(); }
    suspend() { window.audioProbe.suspends++; return super.suspend(); }
    createBufferSource() {
      const source = super.createBufferSource();
      const start = source.start.bind(source), stop = source.stop.bind(source);
      let active = false;
      const end = () => { if (active) { active = false; window.audioProbe.active--; } };
      source.addEventListener('ended', end);
      source.start = (...args) => {
        window.audioProbe.buffers++;
        const buffer = source.buffer;
        if (buffer && buffer.getChannelData(0).some(v => v !== 0)) window.audioProbe.nonzero++;
        active = true;
        window.audioProbe.active++;
        window.audioProbe.peak = Math.max(window.audioProbe.peak, window.audioProbe.active);
        return start(...args);
      };
      source.stop = (...args) => { end(); return stop(...args); };
      return source;
    }
  };
});
async function ready() {
  await page.waitForFunction(() => !document.querySelector('#model').disabled);
}
async function pixel(x,y) {
  return page.locator('#screen').evaluate((canvas,{x,y}) => Array.from(canvas.getContext('2d').getImageData(x,y,1,1).data),{x,y});
}
async function display() {
  await page.waitForFunction(() => {
    if (document.querySelector('#rom-status').textContent !== 'Running') return false;
    const context = document.querySelector('#screen').getContext('2d');
    return [[0,0,0,255,0],[9,9,255,0,0],[16,16,0,0,255]].every(([x,y,r,g,b]) => {
      const data = context.getImageData(x,y,1,1).data;
      return data[0] === r && data[1] === g && data[2] === b;
    });
  });
  assert.deepEqual(await pixel(0,0),[0,255,0,255]);
  assert.deepEqual(await pixel(9,9),[255,0,0,255]);
  assert.equal(await page.locator('#error').textContent(),'');
}
async function load(probe, filename='same-name.ws') {
  await page.locator('#rom').setInputFiles({ name:filename, mimeType:'application/octet-stream', buffer:probe.bytes });
  await display();
}
async function saved(probe, expected, bankOffset=0) {
  await page.waitForFunction(({key,expected,bankOffset}) => {
    const value = localStorage.getItem(key);
    return value !== null && atob(value).charCodeAt(bankOffset) === expected;
  }, {key:probe.key,expected,bankOffset});
}
async function pause() {
  if (await page.locator('#pause').textContent() === 'Pause') await page.locator('#pause').click();
}
try {
  await page.goto(base); await ready();
  const count = requests.length;
  for (const probe of probes) {
    await load(probe);
    await saved(probe,1);
    if (probe.name === 'sram128-2003') await saved(probe,65,65536);
    await page.locator('#reset').click(); await display(); await saved(probe,2);
    await page.locator('#model').selectOption('color'); await display(); await saved(probe,3);
    await page.locator('#model').selectOption('crystal'); await display(); await saved(probe,4);
    await pause(); await saved(probe,4);
    if (probe.name === 'sram128-2003') await saved(probe,68,65536);
  }
  assert.equal(requests.length,count,'ROM selection and save persistence make no requests');
  assert.equal(await page.evaluate(() => Object.keys(localStorage).filter(k => k.startsWith('wonderswan:save:')).length),5);
  // A new page in the same context models closing/reopening this origin.
  await page.close();page = await context.newPage();page.on('pageerror',e => errors.push(e.message));
  await page.goto(base);await ready();await load(probes[0],'renamed.ws');await saved(probes[0],5);
  await page.reload();await ready();await load(probes[0]);await saved(probes[0],6);
  await load(probes[2]);await saved(probes[2],5); // Isolated EEPROM identity, same filename.
  await load(probes[0]);await saved(probes[0],7);
  console.log('SHA-256 save isolation, renamed/same-name ROMs, reopen/reload, all SRAM/EEPROM probes, pause/reset/model preservation passed');

  await page.locator('#sound').click();
  await page.waitForFunction(() => window.audioProbe.nonzero > 3);
  assert.equal(await page.locator('#sound').textContent(),'Mute sound');
  await pause();
  assert.equal(await page.evaluate(() => window.audioProbe.active),0);
  const buffers = await page.evaluate(() => window.audioProbe.buffers);
  await page.waitForTimeout(100);
  assert.equal(await page.evaluate(() => window.audioProbe.buffers),buffers);
  await page.locator('#pause').click();
  await page.waitForFunction(before => window.audioProbe.buffers > before,buffers);
  for (let i=0; i<3; i++) {await page.locator('#reset').click();await display();}
  const audio = await page.evaluate(() => window.audioProbe);
  assert.equal(audio.contexts,1,'reuse one context across resets');assert.ok(audio.peak <= 8,'bounded sources');assert.ok(audio.suspends>0 && audio.resumes>0);
  await page.evaluate(() => {Object.defineProperty(document,'hidden',{configurable:true,value:true});document.dispatchEvent(new Event('visibilitychange'));});
  assert.equal(await page.evaluate(() => window.audioProbe.active),0);
  await page.evaluate(() => {delete document.hidden;document.dispatchEvent(new Event('visibilitychange'));});
  assert.equal(await page.locator('#pause').textContent(),'Resume');
  await page.locator('#sound').click();assert.equal(await page.locator('#sound').textContent(),'Enable sound');
  await page.locator('#sound').click(); // Enable while paused.
  await page.evaluate(() => { AudioContext.prototype.createBuffer = () => { throw new Error('test playback unavailable'); }; });
  await page.locator('#pause').click();
  await page.waitForFunction(() => document.querySelector('#notice').textContent.includes('Sound playback became unavailable'));
  assert.equal(await page.locator('#sound').textContent(),'Enable sound');
  assert.equal(await page.locator('#rom-status').textContent(),'Running','audio failure must not stop CPU/video');
  await pause();
  console.log('Actual Web Audio non-silent buffers, bounded sources, single context, pause/resume/reset/mute/visibility passed');

  // Existing invalid data must be preserved, not overwritten with an erased save.
  await page.reload();await ready();await page.evaluate(key => localStorage.setItem(key,'invalid!'),probes[0].key);
  await load(probes[0]);await pause();
  assert.match(await page.locator('#notice').textContent(),/could not be read/);
  assert.equal(await page.evaluate(key => localStorage.getItem(key),probes[0].key),'invalid!');
  // Storage-denied and missing-audio paths use real probes and stay playable.
  const unavailable = await browser.newContext();
  await unavailable.addInitScript(() => {
    Object.defineProperty(window,'AudioContext',{configurable:true,value:undefined});
    Storage.prototype.setItem = () => { throw new DOMException('test storage unavailable','QuotaExceededError'); };
  });
  const original = page;page = await unavailable.newPage();page.on('pageerror',e => errors.push(e.message));
  await page.goto(base);await ready();await load(probes[0]);await pause();
  assert.match(await page.locator('#notice').textContent(),/storage is unavailable/);
  await page.locator('#sound').click();
  await page.waitForFunction(() => document.querySelector('#notice').textContent.includes('Sound is unavailable'));
  assert.equal(await page.locator('#sound').textContent(),'Enable sound');
  await unavailable.close();page = original;
  assert.deepEqual(errors,[]);
  console.log('Unreadable saves preserved; storage/audio unavailable degrade safely without browser exceptions');
} finally { await browser.close(); }
