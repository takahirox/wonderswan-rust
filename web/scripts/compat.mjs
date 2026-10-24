// SPDX-License-Identifier: CC0-1.0
// Production-preview checks using only this repository's original CC0 probes.
import assert from 'node:assert/strict';
import { lcdChecks } from './lcd.mjs';
import { lcdOutputChecks } from './lcd-output.mjs';
import { ordinaryIoChecks } from './ordinary-io.mjs';
import { multiplyTimingChecks } from './multiply-timing.mjs';
import { shiftTimingChecks } from './shift-timing.mjs';
import { returnTimingChecks } from './return-timing.mjs';
import { indirectTimingChecks } from './indirect-timing.mjs';
import { callTimingChecks } from './call-timing.mjs';
import { jccTimingChecks } from './jcc-timing.mjs';
import { jmpTimingChecks } from './jmp-timing.mjs';
import { loopTimingChecks } from './loop-timing.mjs';
import { irqTimingChecks } from './irq-timing.mjs';
import { wordTimingChecks } from './word-timing.mjs';
import { keypadChecks } from './keypad.mjs';
import { sweepChecks } from './sweep.mjs';
import { startupChecks } from './startup.mjs';
import { shiftRotateChecks } from './shift-rotate.mjs';
import { divisionChecks } from './divide.mjs';
import { trapChecks } from './trap.mjs';
import { rtcCalendarChecks } from './rtc-calendar.mjs';
import { rtcChecks } from './rtc.mjs';
import { pushSpChecks } from './push-sp.mjs';
import { conversionChecks } from './conversion.mjs';
import { multiplicationChecks } from './multiply.mjs';
import { fpo1PollChecks } from './fpo1-poll.mjs';
import { buslockChecks } from './buslock.mjs';
import { consoleChecks } from './console.mjs';
import { rawRomChecks } from './raw-rom.mjs';
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
    activations:[], fetchResumes:[], chooserResumes:[], readResumes:[], lastBuffer:null };
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
      // AnalyserNode downmixes stereo to mono, cancelling the direct Hyper
      // Voice probe's opposite channels. Measure the same isolated left channel
      // before/after gain; the real stereo playback graph remains untouched.
      const splitter = this.createChannelSplitter(2);
      gain.connect(splitter);
      splitter.connect(window.outputAnalyser,0);
      return gain;
    }
    createBiquadFilter() {
      const filter = super.createBiquadFilter();
      window.inputAnalyser = this.createAnalyser();
      const splitter = this.createChannelSplitter(2);
      filter.connect(splitter);
      splitter.connect(window.inputAnalyser,0);
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
        if (buffer) {
          if (buffer.getChannelData(0).some(v => v !== 0)) window.audioProbe.nonzero++;
          // Keep only the latest real buffer, bounded to 1024 stereo frames.
          window.audioProbe.lastBuffer = { rate:buffer.sampleRate, length:buffer.length,
            left:Array.from(buffer.getChannelData(0).slice(0,1024)),
            right:Array.from(buffer.getChannelData(1).slice(0,1024)) };
        }
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
  // The original A0 CPU probe runs width edits from RAM, checks readbacks,
  // matching-width ROM data and Color/Crystal GDMA + both SDMA destinations.
  const a0 = await readFile(new URL('../../fixtures/generated/a0.ws', import.meta.url));
  for (const model of ['mono','color','crystal']) {
    await page.reload();await ready();
    await page.locator('#model').selectOption(model);
    await page.locator('#rom').setInputFiles({name:'a0.ws',mimeType:'application/octet-stream',buffer:a0});
    const success = async () => {
      await page.waitForFunction(model => {
        const p = document.querySelector('#screen').getContext('2d').getImageData(0,0,1,1).data;
        const expected = model === 'mono' ? [170,170,170] : [0,255,0];
        return document.querySelector('#rom-status').textContent === 'Running'
          && expected.every((value,i) => p[i] === value);
      },model);
      assert.equal(await page.locator('#error').textContent(),'');
    };
    await success();
    await page.locator('#reset').click();await success();
    await pause();
  }
  console.log('Original A0 CPU OUT/IN fixture: all models, matching ROM, live wait SDMA, GDMA acceptance/rejection and restart passed');
  // Actual compiled core PCM passes through the same adapter and Web Audio
  // graph as cartridge gameplay. Inspect pre-filter buffers, not port acceptance.
  async function greenSound() {
    await page.waitForFunction(() => {
      const p = document.querySelector('#screen').getContext('2d').getImageData(0,0,1,1).data;
      return document.querySelector('#rom-status').textContent === 'Running'
        && p[0] === 0 && p[1] === 255 && p[2] === 0;
    });
    assert.equal(await page.locator('#error').textContent(),'');
  }
  async function soundBuffer(path, before) {
    await page.waitForFunction(({path,before}) => {
      const p = window.audioProbe, b = p.lastBuffer;
      const expected = path === 0 ? [0.03125,0.21875] : path === 1 ? [-0.125,0.125] : [-0.25,0.25,-0.125,0.125];
      return p.buffers > before+3 && b && expected.every(v => b.left.includes(v));
    },{path,before});
    const buffer = await page.evaluate(() => window.audioProbe.lastBuffer);
    assert.equal(buffer.rate,24000);
    assert.ok(buffer.length >= 318 && buffer.length <= 640,'bounded frame-sized Wasm PCM');
    const allowed = path === 0 ? [0.03125,0.21875] : path === 1 ? [-0.125,0.125] : [-0.25,0.25,-0.125,0.125];
    assert.ok([...buffer.left,...buffer.right].every(v => Number.isFinite(v) && allowed.includes(v)),'exact independently calculated PCM values');
    if (path < 2) assert.deepEqual(buffer.left,buffer.right);
    else assert.ok(buffer.left.filter((v,i) => v === -buffer.right[i]).length > buffer.left.length*0.85);
  }
  for (const [path,name] of ['sound-channel2','sound-hyper','sound-direct'].entries()) {
    const bytes = await readFile(new URL(`../../fixtures/generated/${name}.ws`,import.meta.url));
    for (const model of ['color','crystal']) {
      await page.evaluate(() => localStorage.setItem('wonderswan:audio:v1','{"muted":false,"volume":70}'));
      await page.reload();await ready();await page.locator('#model').selectOption(model);
      const chooser = page.waitForEvent('filechooser');
      await page.locator('#load-rom').click();
      await (await chooser).setFiles({name:`${name}.ws`,mimeType:'application/octet-stream',buffer:bytes});
      await greenSound();await soundBuffer(path,0);
      assert.equal(await page.evaluate(() => window.audioProbe.activations[0]),true);
      await page.locator('#volume').fill('35');await page.waitForTimeout(150);
      assert.ok(Math.abs(await page.evaluate(() => window.outputGain.gain.value)-0.35)<0.001);
      const ratio = await page.evaluate(() => {
        const rms = a => {const data=new Float32Array(a.fftSize);a.getFloatTimeDomainData(data);return Math.sqrt(data.reduce((s,v)=>s+v*v,0)/data.length);};
        return {input:rms(window.inputAnalyser),output:rms(window.outputAnalyser)};
      });
      assert.ok(ratio.input > 0.001,`${name}/${model}: live analyser input ${ratio.input}, output ${ratio.output}`);
      assert.ok(Math.abs(ratio.output/ratio.input-0.35)<0.025,'actual sound probe output follows gain');
      await page.locator('#sound').click();
      assert.equal(await page.evaluate(() => window.audioProbe.active),0);
      assert.equal(await page.evaluate(() => window.audioProbe.lastGain),0,'mute schedules zero gain and stops every source');
      const muted = await page.evaluate(() => window.audioProbe.buffers);
      await page.waitForTimeout(100);assert.equal(await page.evaluate(() => window.audioProbe.buffers),muted);
      await page.locator('#sound').click();await soundBuffer(path,muted);
      await pause();assert.equal(await page.evaluate(() => window.audioProbe.active),0);
      const paused = await page.evaluate(() => window.audioProbe.buffers);
      await page.waitForTimeout(100);assert.equal(await page.evaluate(() => window.audioProbe.buffers),paused);
      await page.locator('#pause').click();await soundBuffer(path,paused);
      const restarted = await page.evaluate(() => window.audioProbe.buffers);
      await page.locator('#reset').click();await greenSound();await soundBuffer(path,restarted);
      await page.evaluate(() => {Object.defineProperty(document,'hidden',{configurable:true,value:true});document.dispatchEvent(new Event('visibilitychange'));});
      assert.equal(await page.evaluate(() => window.audioProbe.active),0);
      await page.evaluate(() => {delete document.hidden;document.dispatchEvent(new Event('visibilitychange'));});
      assert.equal(await page.locator('#pause').textContent(),'Resume');
      const hidden = await page.evaluate(() => window.audioProbe.buffers);
      await page.locator('#pause').click();await soundBuffer(path,hidden);
      const state = await page.evaluate(() => window.audioProbe);
      assert.equal(state.contexts,1);assert.ok(state.peak<=8);
      await pause();
    }
    // Each cartridge explicitly detects mono and halts without color-only I/O.
    await page.reload();await ready();await page.locator('#model').selectOption('mono');
    const chooser = page.waitForEvent('filechooser');await page.locator('#load-rom').click();
    await (await chooser).setFiles({name:`${name}.ws`,mimeType:'application/octet-stream',buffer:bytes});
    await page.waitForFunction(() => window.audioProbe.buffers>3);
    assert.equal(await page.evaluate(() => window.audioProbe.nonzero),0);
    assert.equal(await page.locator('#error').textContent(),'');await pause();
  }
  console.log('Original SDMA Channel 2, SDMA Hyper Voice and direct CPU Hyper Voice: exact real Wasm PCM on Color/Crystal, mono silence, measured gain, mute/pause/restart/visibility and bounded single-context playback passed');
  await sweepChecks(browser,base,errors,observeAudio);
  await startupChecks(browser,base,errors);
  await buslockChecks(browser,base,errors);
  await pushSpChecks(browser,base,errors);
  await rtcChecks(browser,base,errors);
  await rtcCalendarChecks(browser,base,errors);
  await fpo1PollChecks(browser,base,errors);
  await conversionChecks(browser,base,errors);
  await multiplicationChecks(browser,base,errors);
  await divisionChecks(browser,base,errors);
  await trapChecks(browser,base,errors);
  await shiftRotateChecks(browser,base,errors);
  await lcdChecks(browser,base,errors);
  await lcdOutputChecks(browser,base,errors);
  await wordTimingChecks(browser,base,errors);
  await callTimingChecks(browser,base,errors);
  await multiplyTimingChecks(browser,base,errors);
  await shiftTimingChecks(browser,base,errors);
  await returnTimingChecks(browser,base,errors);
  await indirectTimingChecks(browser,base,errors);
  await jccTimingChecks(browser,base,errors);
  await jmpTimingChecks(browser,base,errors);
  await loopTimingChecks(browser,base,errors);
  await irqTimingChecks(browser,base,errors);
  await ordinaryIoChecks(browser,base,errors);
  await keypadChecks(browser,base,errors);
  await rawRomChecks(browser,base,errors);
  await consoleChecks(browser,base,errors);
  assert.deepEqual(errors,[]);
  console.log('Unreadable saves preserved; storage/audio unavailable degrade safely without browser exceptions');
} finally { await browser.close(); }
