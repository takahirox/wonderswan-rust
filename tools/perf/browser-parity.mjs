// Run the project and external Beetle Wasm in one Chromium host. The same
// frontend drives both cores, so this isolates the core/adapter contribution.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { parseArgs } from 'node:util';
import { chromium } from '../../web/node_modules/playwright/index.mjs';

const { values } = parseArgs({ options: {
  browser: { type: 'string' }, beetle: { type: 'string' }, frontend: { type: 'string' },
  pkg: { type: 'string', default: 'web/pkg' }, output: { type: 'string' },
  frames: { type: 'string', default: '6000' }, seconds: { type: 'string', default: '12' }, check: { type: 'boolean' },
} });
if (!values.beetle || !values.frontend || !values.output) {
  throw new Error('usage: browser-parity.mjs --beetle MODULE_MJS --frontend COMPILED_DIR --output JSON [--browser EXECUTABLE] [--pkg PKG]');
}
const frames = Number(values.frames), seconds = Number(values.seconds);
assert.ok(Number.isInteger(frames) && frames > 0 && Number.isFinite(seconds) && seconds >= 3);
const assets = new Map();
for (const [name, path] of Object.entries({
  '/project.js': resolve(values.pkg, 'ws_web.js'),
  '/ws_web_bg.wasm': resolve(values.pkg, 'ws_web_bg.wasm'),
  '/beetle.mjs': resolve(values.beetle),
  '/beetle.wasm': resolve(values.beetle.replace(/\.mjs$/, '.wasm')),
  '/audio.js': resolve(values.frontend, 'audio.js'),
  '/display.js': resolve(values.frontend, 'display.js'),
  '/audio.ws': 'fixtures/generated/sram128-2003.ws',
  '/keypad.ws': 'fixtures/generated/keypad-horizontal.ws',
})) assets.set(name, await readFile(path));
const identity = Object.fromEntries([...assets].map(([name, bytes]) => [name, createHash('sha256').update(bytes).digest('hex')]));
const browser = await chromium.launch({ headless: true, executablePath: values.browser });
const origin = 'http://127.0.0.1:4180';
const rows = [];
const profiles = [
  { name: 'clean', effect: 'clean', cpu: 1 },
  { name: 'clean-10hz', effect: 'clean', cpu: 8, hz: 10 },
  { name: 'crt-stalls', effect: 'retro-crt', cpu: 8, hz: 60, block: 80 },
  { name: 'crt-long-stalls', effect: 'retro-crt', cpu: 8, hz: 60, block: 120 },
];
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.route(`${origin}/**`, route => {
    const path = new URL(route.request().url()).pathname;
    if (path === '/') return route.fulfill({ contentType: 'text/html', body: '<button>Start audio</button><canvas width="224" height="144"></canvas><canvas hidden></canvas>' });
    const type = path.endsWith('.wasm') ? 'application/wasm' : path.endsWith('.ws') ? 'application/octet-stream' : 'text/javascript';
    return assets.has(path) ? route.fulfill({ contentType: type, body: assets.get(path) }) : route.abort();
  });
  await page.goto(origin);
  await page.evaluate(async () => {
    const { default: init, WebMachine } = await import('/project.js');
    const { default: initBeetle } = await import('/beetle.mjs');
    await init();
    const inputs = Object.fromEntries(await Promise.all(['audio', 'keypad'].map(async name => [name, new Uint8Array(await (await fetch(`/${name}.ws`)).arrayBuffer())])));
    window.newCore = async (core, workload) => {
      const bytes = inputs[workload];
      if (core === 'project') {
        const machine = new WebMachine();
        machine.load_rom(bytes);
        return {
          fps: machine.frame_rate(), rate: machine.sample_rate(),
          step: () => machine.run_frame(), pcm: () => machine.audio_samples(),
          pixels: () => {
            const rgba = machine.framebuffer();
            return new Uint8ClampedArray(rgba.buffer, rgba.byteOffset, rgba.byteLength);
          },
          marker: () => {
            const save = machine.save_data();
            return workload === 'audio' ? save[0] : save[0] | save[1] << 8;
          },
          close: () => machine.free(),
        };
      }
      const module = await initBeetle({ locateFile: name => `/${name}` });
      const pointer = module._malloc(bytes.length);
      module.HEAPU8.set(bytes, pointer);
      if (!module._bench_open(pointer, bytes.length)) throw new Error('Beetle ROM load failed');
      return {
        fps: module._bench_frame_rate(), rate: module._bench_sample_rate(),
        step: () => module._bench_step(0), pcm: () => {
          const start = module._bench_pcm() / 4;
          return module.HEAPF32.slice(start, start + module._bench_pcm_values());
        },
        pixels: () => {
          const start = module._bench_rgba();
          return new Uint8ClampedArray(module.HEAPU8.slice(start, start + 224 * 144 * 4).buffer);
        },
        marker: () => module._bench_marker(0x400),
        close: () => { module._bench_close(); module._free(pointer); },
      };
    };
  });
  const throughput = await page.evaluate(async count => {
    const samples = [];
    for (const workload of ['audio', 'keypad']) {
      for (let trial = 0; trial < 5; trial++) {
        const cores = trial % 2 ? ['beetle', 'project'] : ['project', 'beetle'];
        for (const core of cores) {
          const machine = await window.newCore(core, workload);
          try {
            for (let i = 0; i < 120; i++) { machine.step(); machine.pcm(); machine.pixels(); }
            const start = performance.now();
            let sampleValues = 0;
            for (let i = 0; i < count; i++) {
              machine.step();
              sampleValues += machine.pcm().length;
              machine.pixels();
            }
            const us = (performance.now() - start) * 1000 / count;
            const pixels = machine.pixels();
            const marker = machine.marker();
            const expected = workload === 'audio' ? 1 : 0x1000;
            if (marker !== expected || sampleValues !== count * 636) throw new Error(`${core}/${workload}: marker ${marker}, PCM ${sampleValues}`);
            samples.push({ core, workload, trial, us, sample_values: sampleValues, marker,
              first_pixel: [...pixels.slice(0, 4)] });
          } finally { machine.close(); }
        }
      }
    }
    return samples;
  }, frames);
  const cdp = await page.context().newCDPSession(page);
  for (const [index, profile] of profiles.entries()) {
    const cores = index % 2 ? ['beetle', 'project'] : ['project', 'beetle'];
    for (const core of cores) {
      await cdp.send('Emulation.setCPUThrottlingRate', { rate: profile.cpu });
      await page.evaluate(async ({ core, profile, seconds }) => {
        const { Sound } = await import('/audio.js');
        const { Display } = await import('/display.js');
        const machine = await window.newCore(core, 'audio');
        let notice = '';
        for (const old of document.querySelectorAll('canvas')) old.remove();
        const clean = document.createElement('canvas'), output = document.createElement('canvas');
        clean.width = 224;
        clean.height = 144;
        document.body.append(clean, output);
        const display = new Display(clean, output, message => { notice = message; });
        display.select(profile.effect);
        display.resize(4);
        const sound = new Sound(() => {});
        const Native = window.AudioContext;
        let context, frames = 0, scheduled = 0, nonzero = 0, generated = 0;
        let gaps = 0, gapMs = 0, tail = null, lastSubmit = null, maxSubmissionMs = 0, minLeadMs = Infinity;
        let clock, blockTimer, raf, timer, rendered = 0, lastRender = -Infinity, pending = false;
        window.AudioContext = class extends Native {
          constructor() { super(); context = this; }
          createBufferSource() {
            const node = super.createBufferSource(), start = node.start.bind(node);
            node.start = when => {
              const now = this.currentTime;
              if (tail !== null && when > tail + 0.00001) { gaps++; gapMs += (when - tail) * 1000; }
              if (lastSubmit !== null) maxSubmissionMs = Math.max(maxSubmissionMs, (now - lastSubmit) * 1000);
              lastSubmit = now;
              minLeadMs = Math.min(minLeadMs, (when - now) * 1000);
              tail = when + node.buffer.duration / node.playbackRate.value;
              scheduled += node.buffer.length;
              if (node.buffer.getChannelData(0).some(value => value !== 0)) nonzero++;
              return start(when);
            };
            return node;
          }
        };
        const frameMs = 1000 / machine.fps;
        let elapsed = 0, lastTime;
        const tick = () => {
          const start = performance.now();
          elapsed = Math.min(sound.catchUpMs ?? 100, elapsed + start - lastTime);
          lastTime = start;
          for (let count = 0; count < 8 && elapsed >= frameMs && performance.now() - start < 8; count++) {
            machine.step();
            const pcm = machine.pcm();
            generated += pcm.length / 2;
            sound.push(pcm, machine.rate);
            frames++;
            pending = true;
            elapsed -= frameMs;
          }
          timer = setTimeout(tick, Math.max(0, Math.min(8, frameMs - elapsed - (performance.now() - start))));
        };
        const present = now => {
          if (pending && (!profile.hz || now - lastRender >= 1000 / profile.hz - 0.5)) {
            display.advance(machine.pixels(), 'crystal');
            display.present();
            lastRender = now;
            pending = false;
            rendered++;
          }
          raf = requestAnimationFrame(present);
        };
        window.measuredPlayback = new Promise((resolve, reject) => {
          document.querySelector('button').onclick = () => {
            sound.resume();
            sound.unlock();
            void context.resume().then(() => {
              clock = lastTime = performance.now();
              tick();
              raf = requestAnimationFrame(present);
              if (profile.block) blockTimer = setInterval(() => {
                const until = performance.now() + profile.block;
                while (performance.now() < until) {}
              }, 503);
              setTimeout(() => {
                clearInterval(blockTimer);
                clearTimeout(timer);
                cancelAnimationFrame(raf);
                const duration = (performance.now() - clock) / 1000;
                const timing = sound.timing;
                const result = { core, profile: profile.name, seconds: duration, frames, rendered, emulated_fps: frames / duration,
                  sample_rate: machine.rate, marker: machine.marker(), display: clean.dataset.effect, notice, generated, scheduled,
                  nonzero, gaps, gap_ms: gapMs, max_submission_ms: maxSubmissionMs, min_lead_ms: minLeadMs, ...timing };
                sound.pause();
                void context.close();
                machine.close();
                window.AudioContext = Native;
                resolve(result);
              }, seconds * 1000);
            }).catch(reject);
          };
        });
      }, { core, profile, seconds });
      await page.getByRole('button', { name: 'Start audio' }).click();
      const row = await page.evaluate(() => window.measuredPlayback);
      assert.equal(row.marker, 1);
      assert.equal(row.sample_rate, 24000);
      assert.equal(row.display, profile.effect);
      assert.equal(row.notice, '');
      assert.ok(row.nonzero > 0 && row.scheduled > 0 && row.rendered > 0);
      if (values.check) {
        const longStall = profile.block === 120;
        assert.ok(row.gaps <= Number(longStall) && row.underruns <= Number(longStall), `${core}/${profile.name}: playback gaps`);
        assert.equal(row.overruns, 0);
        assert.equal(row.generated, row.scheduled);
        assert.ok(Math.abs(row.seconds - row.generated / 24000) < (longStall ? 0.17 : 0.12), `${core}/${profile.name}: PCM production`);
      }
      rows.push(row);
      console.log(`${core} ${profile.name}: ${row.emulated_fps.toFixed(2)} fps, ${row.gaps} gaps, ${row.generated - row.scheduled} unscheduled samples/channel`);
    }
  }
  assert.deepEqual(errors, []);
  const summary = ['audio', 'keypad'].flatMap(workload => ['project', 'beetle'].map(core => ({
    workload, core, median_us: throughput.filter(row => row.core === core && row.workload === workload).map(row => row.us).sort((a, b) => a - b)[2],
  })));
  await writeFile(values.output, JSON.stringify({ browser: await browser.version(), frames, seconds, assets_sha256: identity, summary, throughput, playback: rows }, null, 2) + '\n');
  console.table(summary);
  console.table(rows.map(({ core, profile, emulated_fps, gaps, gap_ms, generated, scheduled, min_lead_ms }) => ({
    core, profile, fps: emulated_fps.toFixed(2), gaps, gap_ms: gap_ms.toFixed(1), unscheduled: generated - scheduled, min_lead_ms: min_lead_ms.toFixed(1),
  })));
} finally { await browser.close(); }
