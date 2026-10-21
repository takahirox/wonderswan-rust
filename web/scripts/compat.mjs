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
const observeAudio = () => {
  window.audioProbe = { buffers:0, nonzero:0, active:0, peak:0, contexts:0, resumes:0, suspends:0,
    activations:[], fetchResumes:[], chooserResumes:[], readResumes:[] };
  let clicking = false;
  document.addEventListener('click', event => { clicking = event.isTrusted; }, true);
  window.addEventListener('click', () => { clicking = false; });
  const fetchROM = window.fetch;
  window.fetch = (...args) => {
    if (String(args[0]).endsWith('.ws')) window.audioProbe.fetchResumes.push(window.audioProbe.resumes);
    return fetchROM(...args);
  };
  const choose = HTMLInputElement.prototype.click;
  HTMLInputElement.prototype.click = function() {
    if (this.type === 'file') window.audioProbe.chooserResumes.push(window.audioProbe.resumes);
    return choose.call(this);
  };
  const read = Blob.prototype.arrayBuffer;
  Blob.prototype.arrayBuffer = function() {
    window.audioProbe.readResumes.push(window.audioProbe.resumes);
    return read.call(this);
  };
  const Native = window.AudioContext;
  if (!Native) return;
  window.AudioContext = class extends Native {
    constructor(...args) {
      super(...args); window.audioProbe.contexts++; window.audioContext = this;
      if (window.audioMode) void super.suspend();
    }
    resume() {
      window.audioProbe.resumes++;
      window.audioProbe.activations.push(clicking && navigator.userActivation.isActive);
      if (window.audioMode === 'reject') return Promise.reject(new Error('simulated resume rejection'));
      if (window.audioMode === 'suspended') return Promise.resolve();
      if (window.audioMode === 'pending') return new Promise(resolve => {
        (window.pendingAudio ??= []).push(() => super.resume().then(resolve));
        window.releaseAudio = () => Promise.all(window.pendingAudio.splice(0).map(release => release()));
      });
      return super.resume();
    }
    createGain() {
      const gain = super.createGain();
      window.outputGain = gain;
      const set = gain.gain.setValueAtTime.bind(gain.gain);
      gain.gain.setValueAtTime = (value,time) => {
        window.audioProbe.lastGain = value;
        return set(value,time);
      };
      window.outputAnalyser = this.createAnalyser();
      gain.connect(window.outputAnalyser);
      return gain;
    }
    createBiquadFilter() {
      const filter = super.createBiquadFilter();
      window.inputAnalyser = this.createAnalyser();
      filter.connect(window.inputAnalyser);
      return filter;
    }
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
};
await context.addInitScript(observeAudio);
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
  const chooser = page.waitForEvent('filechooser');
  await page.locator('#load-rom').click();
  await (await chooser).setFiles({ name:filename, mimeType:'application/octet-stream', buffer:probe.bytes });
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
  assert.equal(await page.locator('#sound').textContent(),'Mute');
  assert.equal(await page.locator('#volume').inputValue(),'70');
  assert.match(await page.locator('#sound-status').textContent(),/Start a game/);
  assert.equal(await page.locator('#enable-audio').isVisible(),false,'no extra unlock action before game start');
  assert.equal(await page.evaluate(() => window.audioProbe.contexts),0,'no context on page load');
  assert.equal(await page.getByRole('slider',{name:'Volume',exact:true}).count(),1);
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

  // Default-on playback uses the actual original CC0 square-wave cartridge.
  await page.waitForFunction(() => window.audioProbe.nonzero > 3);
  assert.equal(await page.locator('#sound').textContent(),'Mute');
  assert.match(await page.locator('#sound-status').textContent(),/Audio ready/);
  assert.equal(await page.evaluate(() => window.audioProbe.activations[0]),true,'Load ROM resumes in its trusted click');
  assert.ok(await page.evaluate(() => window.audioProbe.chooserResumes[0] > 0),'resume precedes file chooser');
  assert.ok(await page.evaluate(() => window.audioProbe.readResumes[0] > 0),'resume precedes file read');
  async function level(value) {
    await page.locator('#volume').fill(String(value));
    await page.waitForTimeout(100); // Includes the 10ms ramp and analyser window.
    assert.equal(await page.locator('#volume-value').textContent(),`${value}%`);
    assert.ok(Math.abs(await page.evaluate(() => window.outputGain.gain.value)-value/100)<0.001);
    const ratio = await page.evaluate(() => {
      const rms = analyser => {
        const samples = new Float32Array(analyser.fftSize);
        analyser.getFloatTimeDomainData(samples);
        return Math.sqrt(samples.reduce((sum,v) => sum+v*v,0)/samples.length);
      };
      return { input:rms(window.inputAnalyser), output:rms(window.outputAnalyser) };
    });
    assert.ok(ratio.input > 0.001,'unchanged, non-silent emulated samples before gain');
    assert.ok(Math.abs(ratio.output/ratio.input-value/100)<0.025,'measured output follows gain');
    await display(); // Video and execution continue at every volume.
  }
  for (const value of [0,35,100,70]) await level(value);
  await page.locator('#volume').focus();
  await page.keyboard.press('ArrowLeft');
  assert.equal(await page.locator('#volume').inputValue(),'69','native keyboard slider');
  await level(70);
  await page.locator('#sound').click();
  assert.equal(await page.locator('#sound').textContent(),'Unmute');
  assert.equal(await page.locator('#volume').inputValue(),'70');
  assert.equal(await page.evaluate(() => window.audioProbe.lastGain),0);
  assert.equal(await page.evaluate(() => window.audioProbe.active),0);
  await page.locator('#reset').click(); await display();
  await page.locator('#model').selectOption('color'); await display();
  await pause(); await page.locator('#pause').click(); await display();
  await load(probes[0]);
  assert.equal(await page.locator('#sound').textContent(),'Unmute','lifecycle never overrides mute');
  await page.locator('#volume').fill('42');
  assert.equal(await page.evaluate(() => window.audioProbe.lastGain),0,'slider respects explicit mute');
  await page.reload(); await ready();
  assert.equal(await page.locator('#sound').textContent(),'Unmute');
  assert.equal(await page.locator('#volume').inputValue(),'42');
  assert.equal(await page.evaluate(() => window.audioProbe.contexts),0,'remembered mute creates no context');
  await load(probes[0]);
  assert.equal(await page.evaluate(() => window.audioProbe.contexts),0);
  await page.locator('#sound').click();
  await page.waitForFunction(() => window.audioProbe.nonzero > 3);
  await level(42);
  for (let i=0; i<5; i++) {
    await page.locator('#sound').click();
    assert.equal(await page.evaluate(() => window.audioProbe.active),0);
    assert.doesNotMatch(await page.locator('#sound-status').textContent(),/Tap Enable audio/,'muted guidance uses the visible control');
    await page.locator('#sound').click();
  }
  await pause();
  assert.equal(await page.evaluate(() => window.audioProbe.active),0);
  const buffers = await page.evaluate(() => window.audioProbe.buffers);
  await page.waitForTimeout(100);
  assert.equal(await page.evaluate(() => window.audioProbe.buffers),buffers);
  await page.locator('#pause').click();
  await page.waitForFunction(before => window.audioProbe.buffers > before,buffers);
  for (let i=0; i<3; i++) {await page.locator('#reset').click();await display();}
  const audio = await page.evaluate(() => window.audioProbe);
  assert.equal(audio.contexts,1,'reuse one context across toggles and resets');assert.ok(audio.peak <= 8,'bounded sources');assert.ok(audio.suspends>0 && audio.resumes>0);
  await page.evaluate(() => {Object.defineProperty(document,'hidden',{configurable:true,value:true});document.dispatchEvent(new Event('visibilitychange'));});
  assert.equal(await page.evaluate(() => window.audioProbe.active),0);
  await page.evaluate(() => {delete document.hidden;document.dispatchEvent(new Event('visibilitychange'));});
  assert.equal(await page.locator('#pause').textContent(),'Resume');
  await page.locator('#sound').click();
  await page.locator('#sound').click(); // Unmute while paused.
  await page.waitForFunction(() => window.audioContext.state === 'suspended');
  assert.equal(await page.evaluate(() => window.audioProbe.active),0);
  await page.locator('#pause').click();
  const createBuffer = await page.evaluateHandle(() => AudioContext.prototype.createBuffer);
  await page.evaluate(() => { AudioContext.prototype.createBuffer = () => { throw new Error('test playback unavailable'); }; });
  await page.waitForFunction(() => document.querySelector('#sound-status').textContent.includes('Audio playback unavailable'));
  assert.equal(await page.locator('#sound').textContent(),'Mute','failure preserves preference');
  assert.equal(await page.locator('#rom-status').textContent(),'Running','audio failure must not stop CPU/video');
  await page.evaluate(original => { AudioContext.prototype.createBuffer = original; },createBuffer);
  const beforeRecovery = await page.evaluate(() => window.audioProbe.buffers);
  await page.locator('#enable-audio').click();
  await page.waitForFunction(before => window.audioProbe.buffers > before,beforeRecovery);
  await page.evaluate(() => window.dispatchEvent(new Event('pagehide')));
  assert.equal(await page.evaluate(() => window.audioProbe.active),0);
  assert.equal(await page.locator('#rom-status').textContent(),'Paused');
  console.log('Real default-on Web Audio, measured 0/35/100% gain, keyboard volume, persistent mute/level, lifecycle cleanup and playback-error recovery passed');

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
  await page.locator('#enable-audio').click();
  await page.waitForFunction(() => document.querySelector('#sound-status').textContent.includes('Audio unavailable'));
  assert.equal(await page.locator('#sound').textContent(),'Mute');
  await unavailable.close();page = original;

  // Isolated contexts exercise policies deterministically; native Chromium output
  // above is real, while these restrictions/errors and visibility are injected.
  const originalPage = page;
  for (const mode of ['suspended','reject','pending','create']) {
    const policy = await browser.newContext();
    await policy.addInitScript(observeAudio);
    await policy.addInitScript(mode => {
      window.audioMode = mode;
      if (mode === 'create') {
        const Available = window.AudioContext;
        window.AudioContext = class { constructor() {
          if (window.audioMode === 'create') throw new Error('simulated creation failure');
          return new Available();
        } };
      }
    },mode);
    page = await policy.newPage();page.on('pageerror',e => errors.push(e.message));
    await page.goto(base);await ready();await load(probes[0]);
    assert.equal(await page.locator('#rom-status').textContent(),'Running');
    assert.equal(await page.locator('#error').textContent(),'');
    assert.equal(await page.locator('#enable-audio').isVisible(),true);
    assert.doesNotMatch(await page.locator('#sound-status').textContent(),/Audio ready|Sound playing/);
    if (mode === 'pending') {
      await pause();
      await page.evaluate(async () => { window.audioMode = ''; await window.releaseAudio(); });
      await page.waitForFunction(() => window.audioContext.state === 'suspended');
      assert.equal(await page.evaluate(() => window.audioProbe.active),0,'late resume stays paused');
      await page.locator('#pause').click();
    } else {
      await page.evaluate(() => { window.audioMode = ''; });
      await page.locator('#enable-audio').click();
    }
    await page.waitForFunction(() => window.audioProbe.nonzero > 3);
    assert.match(await page.locator('#sound-status').textContent(),/Audio ready/);
    assert.equal(await page.evaluate(() => window.audioProbe.contexts),1,'retry reuses an existing context');
    await page.evaluate(() => window.audioContext.suspend());
    await page.waitForFunction(() => document.querySelector('#sound-status').textContent.includes('Audio suspended'));
    assert.equal(await page.evaluate(() => window.audioProbe.active),0,'external suspension clears queued buffers');
    const before = await page.evaluate(() => window.audioProbe.buffers);
    await page.locator('#enable-audio').click();
    await page.waitForFunction(before => window.audioProbe.buffers > before,before);
    await policy.close();
  }
  page = originalPage;
  console.log('Simulated suspended/resolved, rejected, pending/late and creation-failed audio; explicit retry and external suspension recovery passed');

  const fresh = await browser.newContext();
  await fresh.addInitScript(observeAudio);
  page = await fresh.newPage();page.on('pageerror',e => errors.push(e.message));
  await page.goto(base);await ready();
  let releaseDemo;
  const gate = new Promise(resolve => { releaseDemo = resolve; });
  await page.route('**/demo/beacon-trail.ws',async route => { await gate; await route.continue(); });
  await page.locator('#demo').click();
  assert.ok(await page.evaluate(() => window.audioProbe.resumes > 0),'resume attempted while fetch is gated');
  assert.equal(await page.evaluate(() => window.audioProbe.activations[0]),true,'Play demo resumes from trusted click');
  assert.ok(await page.evaluate(() => window.audioProbe.fetchResumes[0] > 0),'resume precedes demo fetch');
  releaseDemo();
  await page.waitForFunction(() => document.querySelector('#rom-status').textContent === 'Running');
  await page.waitForFunction(() => window.audioProbe.buffers > 3);
  assert.equal(await page.evaluate(() => window.audioProbe.nonzero),0,'shipped original demo is silent');
  await page.locator('#volume').fill('31');
  await page.reload();await ready();
  assert.equal(await page.locator('#volume').inputValue(),'31');
  assert.equal(await page.locator('#sound').textContent(),'Mute','on preference survives reload');
  assert.equal(await page.evaluate(() => window.audioProbe.contexts),0);
  // Mute before any context exists, including starting/restarting a cartridge.
  await page.locator('#sound').click();
  await page.locator('#demo').click();
  await page.waitForFunction(() => document.querySelector('#rom-status').textContent === 'Running');
  assert.equal(await page.locator('#sound').textContent(),'Unmute','Play demo preserves mute');
  await load(probes[0]); await page.locator('#reset').click(); await display();
  assert.equal(await page.evaluate(() => window.audioProbe.contexts),0);
  assert.equal(await page.locator('#sound').textContent(),'Unmute');
  await page.close();page = await fresh.newPage();page.on('pageerror',e => errors.push(e.message));
  await page.goto(base);await ready();
  assert.equal(await page.locator('#sound').textContent(),'Unmute','mute survives reopen');
  assert.equal(await page.locator('#volume').inputValue(),'31');
  await page.locator('#sound').click(); // Unmute before any game is loaded.
  await page.waitForFunction(() => window.audioContext?.state === 'suspended');
  assert.equal(await page.evaluate(() => window.audioProbe.active),0);
  await load(probes[0]);
  await page.waitForFunction(() => window.audioProbe.nonzero > 3);
  await fresh.close();page = originalPage;
  console.log('Gesture-before-fetch/chooser/read, intentionally silent demo, persisted on/mute/volume and pre-unlock mute/unmute passed');

  for (const preference of ['bad JSON','null','{"muted":"yes","volume":"20"}',
    '{"muted":null,"volume":-1}','{"volume":101}','{"volume":1e999}','denied']) {
    const storage = await browser.newContext();
    await storage.addInitScript(observeAudio);
    await storage.addInitScript(preference => {
      if (preference === 'denied') {
        for (const method of ['getItem','setItem']) Storage.prototype[method] = () => { throw new DOMException('simulated denial','SecurityError'); };
      } else localStorage.setItem('wonderswan:audio:v1',preference);
    },preference);
    page = await storage.newPage();page.on('pageerror',e => errors.push(e.message));
    await page.goto(base);await ready();
    assert.equal(await page.locator('#sound').textContent(),'Mute');
    assert.equal(await page.locator('#volume').inputValue(),'70');
    await load(probes[0]);
    await page.waitForFunction(() => window.audioProbe.nonzero > 3);
    await page.locator('#sound').click();
    await page.locator('#volume').fill('56');
    assert.equal(await page.locator('#sound').textContent(),'Unmute');
    assert.equal(await page.locator('#rom-status').textContent(),'Running');
    await storage.close();
  }
  page = originalPage;
  console.log('Malformed/out-of-range/nonfinite preferences and denied storage reads/writes fall back safely without blocking gameplay');

  // The CC0 cartridge checks byte OUT/IN, forward/reverse copies and readbacks
  // itself. Green means success; red means its CPU assertions failed.
  const gdma = await readFile(new URL('../../fixtures/generated/gdma.ws', import.meta.url));
  for (const model of ['color','crystal']) {
    await page.reload();await ready();
    await page.locator('#model').selectOption(model);
    await page.locator('#rom').setInputFiles({name:'gdma.ws',mimeType:'application/octet-stream',buffer:gdma});
    await page.waitForFunction(() => {
      const c = document.querySelector('#screen').getContext('2d');
      return document.querySelector('#rom-status').textContent === 'Running'
        && [[0,0],[9,9]].every(([x,y]) => {
          const p = c.getImageData(x,y,1,1).data;
          return p[0] === 0 && p[1] === 255 && p[2] === 0;
        });
    });
    assert.equal(await page.locator('#error').textContent(),'');
    await pause();
  }
  console.log('Original GDMA CPU byte-I/O fixture: forward/reverse transfers and register assertions passed on Color/Crystal');
  assert.deepEqual(errors,[]);
  console.log('Unreadable saves preserved; storage/audio unavailable degrade safely without browser exceptions');
} finally { await browser.close(); }
