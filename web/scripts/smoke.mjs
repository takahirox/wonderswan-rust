// Production-preview browser acceptance. Run after building and starting Vite preview.
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { fileURLToPath } from 'node:url';
import { readFile, mkdir } from 'node:fs/promises';
import { join } from 'node:path';
import { createHash } from 'node:crypto';

const base = process.env.WS_PREVIEW_URL ?? 'http://127.0.0.1:4173/wonderswan-rust/';
const rom = fileURLToPath(new URL('../../fixtures/generated/keypad-horizontal.ws', import.meta.url));
const emptyRom = {name:'empty.ws',mimeType:'application/octet-stream',buffer:Buffer.alloc(0)};
const browser = await chromium.launch({ headless: true });
let page = await browser.newPage({ viewport: { width:1280, height:800 } }); // fresh, unauthenticated context
console.log(`Chromium ${browser.version()} (${process.platform} ${process.arch})`);
// Observe genuine emulation progress: presentation no longer drives the CPU.
function observeFrameProgress() {
  window.executedFrames=0;
  const wrap=result=>{
    const exports={...result.instance.exports},run=exports.webmachine_run_frame;
    exports.webmachine_run_frame=(...args)=>{
      const ret=run(...args);window.executedFrames++;return ret;
    };
    return {...result,instance:{exports}};
  };
  const streaming=WebAssembly.instantiateStreaming.bind(WebAssembly);
  WebAssembly.instantiateStreaming=async(...args)=>wrap(await streaming(...args));
  const instantiate=WebAssembly.instantiate.bind(WebAssembly);
  WebAssembly.instantiate=async(...args)=>{
    const result=await instantiate(...args);return result.instance?wrap(result):result;
  };
}
await page.addInitScript(observeFrameProgress);
const artifacts = process.env.WS_SMOKE_ARTIFACTS;
if (artifacts) await mkdir(artifacts, { recursive:true });
const errors = [];
const requests = [];
page.on('pageerror', error => errors.push(error.message));
page.on('request', request => requests.push({ url: request.url(), method: request.method() }));
page.on('response', response => { if (response.status() >= 400) errors.push(`${response.status()} ${response.url()}`); });

// Independent CC0 keypad fixture renders CPU IN B5 results as bit tiles.
async function expectInput(y=0, x=0, a=0) {
  const expected = [0,0x10|y,0x20|x,0x30|y|x,0x40|a,0x50|y|a,0x60|x|a,0x70|y|x|a];
  await page.waitForFunction(expected => {
    const context = document.querySelector('#screen').getContext('2d');
    return expected.every((value,row) => {
      let actual = 0;
      for (let bit=0;bit<8;bit++) {
        if (context.getImageData(bit*8+3,row*8+19,1,1).data[0] === 0) actual |= 1<<bit;
      }
      return actual === value;
    });
  }, expected);
}
async function initial() {
  await page.waitForFunction(() => document.querySelector('#rom-status').textContent === 'Running');
  await expectInput();
}
async function load(file=rom) {
  const chooser = page.waitForEvent('filechooser');
  await page.locator('#load-rom').click();
  await (await chooser).setFiles(file);
  await initial();
}
async function cancelPicker() {
  const chooser = page.waitForEvent('filechooser');
  await page.locator('#load-rom').click();
  await (await chooser).setFiles([]);
  assert.equal(await page.evaluate(() => document.activeElement.id), 'load-rom');
}
async function rejectEmptyRom() {
  const chooser = page.waitForEvent('filechooser');
  await page.locator('#load-rom').click();
  await (await chooser).setFiles(emptyRom);
  await page.waitForFunction(() => document.querySelector('#error').textContent.includes('Expected a 128 KiB'));
}
async function checkInViewport(selector) {
  const box = await page.locator(selector).boundingBox();
  const {width, height} = page.viewportSize();
  assert.ok(box && box.width > 0 && box.height > 0 && box.x >= 0 && box.y >= 0
    && box.x + box.width <= width && box.y + box.height <= height,
  `${selector} (${JSON.stringify(box)}) fits the ${width}×${height} viewport without scrolling`);
}
// Hold/release through two genuine emulator frames, then present. Paused input
// still exercises the handlers without expecting the CPU to advance.
async function inputFrames() {
  await page.evaluate(() => new Promise((resolve,reject) => {
    const before=window.executedFrames,deadline=performance.now()+3000;
    const poll=()=>{
      if(document.querySelector('#rom-status').textContent!=='Running'||window.executedFrames>=before+2){
        requestAnimationFrame(resolve);return;
      }
      if(performance.now()>deadline){reject(new Error('No emulator progress with input held/released'));return;}
      setTimeout(poll,4);
    };
    poll();
  }));
}
async function tap(key) {
  await page.keyboard.down(key);
  await inputFrames();
  await page.keyboard.up(key);
  await inputFrames();
}
async function frame() {
  const pixels = await page.locator('#screen').evaluate(c => c.toDataURL());
  return createHash('sha256').update(pixels).digest('hex');
}
async function screenshot(name, fullPage = true) {
  if (artifacts) await page.screenshot({ path:join(artifacts, `${name}.png`), fullPage });
}
async function checkLayout({ orientation = 'horizontal', kind = 'desktop' } = {}) {
  // Let ResizeObserver snap the canvas edges after the grid reflows.
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  const layout = await page.locator('#screen').evaluate(canvas => {
    const rect = selector => document.querySelector(selector).getBoundingClientRect().toJSON();
    return {
      image:rect('#screen'), frame:rect('#display'), effect:rect('#screen-effect'),
      toolbar:rect('#controls'), cartridge:rect('.cartridge'), touch:rect('#touch-controls'),
      xPad:rect('.dpad:first-child'), yPad:rect('.dpad:nth-child(2)'), actions:rect('.action-row'),
      overflow:document.documentElement.scrollWidth > innerWidth,
      backing:[canvas.width,canvas.height], pixels:getComputedStyle(canvas).imageRendering,
    };
  });
  const { image, frame, effect, toolbar, cartridge, touch, xPad, yPad, actions } = layout;
  assert.equal(layout.overflow, false, 'no horizontal overflow');
  assert.deepEqual(layout.backing, [224,144]);
  const ratio = orientation === 'vertical' ? 144/224 : 224/144;
  assert.ok(Math.abs(image.width / image.height - ratio) < 0.01, 'screen aspect ratio');
  assert.equal(layout.pixels, 'pixelated');
  assert.ok(image.left >= frame.left && image.top >= frame.top && image.right <= frame.right && image.bottom <= frame.bottom);
  assert.ok(Math.abs(image.left + image.right - frame.left - frame.right) <= 3, 'picture is horizontally centered');
  assert.ok(Math.abs(image.top + image.bottom - frame.top - frame.bottom) <= 3, 'picture is vertically centered');
  assert.ok(toolbar.top >= frame.bottom, 'toolbar follows the screen');
  if (effect.width) assert.deepEqual(effect, image, 'shader covers the same image bounds');
  if (kind === 'desktop') {
    assert.ok(frame.right < cartridge.left && cartridge.top === frame.top, 'slot sits beside the screen');
    assert.ok(frame.width > cartridge.width * 1.8, 'screen dominates the desktop layout');
  } else {
    assert.ok(cartridge.top >= toolbar.bottom, 'slot follows the toolbar');
    if (kind === 'portrait') assert.ok(touch.top >= cartridge.bottom, 'controller follows the slot');
    else {
      assert.ok(xPad.right < frame.left && frame.right < yPad.left && frame.right < actions.left);
      assert.ok(touch.top >= frame.top && touch.bottom <= toolbar.top, 'both keypads and actions fit beside the image');
    }
  }
  for (const selector of ['#model','#pause','#reset','#load-rom','#fullscreen','#settings','#sound','#volume','#display-effect']) {
    const box = await page.locator(selector).boundingBox();
    const viewport = page.viewportSize();
    assert.ok(box && box.x >= 0 && box.x + box.width <= viewport.width, `${selector} fits viewport`);
    assert.ok(box.height >= 44, `${selector} has practical touch target`);
    if (kind === 'landscape') {
      assert.ok(box.y >= 0 && box.y + box.height <= viewport.height, `${selector} fits the first landscape viewport`);
    }
  }
  if (kind !== 'desktop') {
    for (const button of await page.locator('#touch-controls button').all()) {
      const box = await button.boundingBox();
      assert.ok(box && box.width >= 44 && box.height >= 44, `${await button.textContent()} has a practical touch target`);
    }
  }
  return frame;
}
async function visibility(hidden) {
  await page.evaluate(hidden => {
    if (hidden) Object.defineProperty(document, 'hidden', { configurable:true, value:true });
    else delete document.hidden;
    document.dispatchEvent(new Event('visibilitychange'));
  }, hidden);
}
async function openHelp() {
  await page.locator('#settings').click();
  await page.waitForFunction(() => document.querySelector('#help').open);
  assert.equal(await page.locator('#pause').textContent(),
    await page.locator('#empty-state').isVisible() ? 'Play' : 'Resume');
  assert.equal(await page.evaluate(() => document.activeElement.id), 'help-close');
}
async function closeHelp(keyboard = false) {
  if (keyboard) await page.keyboard.press('Escape');
  else await page.locator('#help-close').click();
  await page.waitForFunction(() => !document.querySelector('#help').open);
  // Wait for the native dialog's queued close event and playback restoration.
  await page.waitForTimeout(50);
}
function checksum(bytes) {
  const sum = bytes.subarray(0,-2).reduce((sum, byte) => (sum + byte) & 0xffff, 0);
  bytes.writeUInt16LE(sum, bytes.length-2);
  return bytes;
}
try {
  await page.goto(base);
  await page.waitForFunction(() => !document.querySelector('#model').disabled);
  assert.equal(await page.getByRole('heading', { name:'wonderswan-rust',exact:true }).count(), 1);
  assert.equal(await page.getByRole('link', { name:'GitHub' }).getAttribute('href'), 'https://github.com/takahirox/wonderswan-rust');
  assert.equal(await page.getByRole('button', { name:'Help',exact:true }).isVisible(), true);
  assert.equal(await page.getByRole('combobox', { name:'Display',exact:true }).isVisible(), true);
  assert.equal(await page.locator('#display-effect').inputValue(), 'retro-crt');
  assert.equal(await page.locator('#debug-toggle').isChecked(), false);
  assert.equal(await page.locator('#debug-panel').isVisible(), false);
  assert.equal(await page.locator('#help').isVisible(), false);
  assert.equal(await page.locator('#touch-controls').isVisible(), false);
  const initialScreen = await checkLayout();
  await screenshot('desktop-empty');
  await page.setViewportSize({ width:744,height:800 });
  await checkLayout();
  await page.setViewportSize({ width:1280,height:800 });
  await page.locator('#display-effect').selectOption('clean');
  await page.locator('#display-effect').focus();
  for (const id of ['fullscreen','load-rom']) {
    await page.keyboard.press('Tab');
    assert.equal(await page.evaluate(() => document.activeElement.id),id);
    assert.equal(await page.locator(`#${id}`).evaluate(el => el.matches(':focus-visible')),true);
  }
  assert.equal(await page.locator('#model').inputValue(), 'crystal');
  assert.equal(await page.locator('#model option').count(), 3);
  assert.equal(await page.locator('#screen').getAttribute('width'), '224');
  assert.equal(await page.locator('#screen').getAttribute('height'), '144');
  assert.equal(await page.locator('#rom-status').textContent(), 'No ROM loaded');
  assert.equal(await page.locator('#rom-title').textContent(), 'No ROM loaded');
  assert.equal(await page.locator('#empty-state').isVisible(), true);
  assert.equal(await page.getByRole('button', {name:'Load ROM',exact:true}).count(), 1);
  assert.equal(await page.locator('#rom').getAttribute('accept'), '.ws,.wsc');
  assert.equal(await page.locator('#pause').isDisabled(), true);
  assert.equal(await page.locator('#reset').isDisabled(), true);
  // Simulate Safari's pointer activation without native button focus.
  await page.locator('#settings').evaluate(button => {
    button.addEventListener('mousedown', event => event.preventDefault(), {once:true});
  });
  await openHelp();
  await page.locator('#debug-toggle').check();
  await closeHelp(true);
  assert.equal(await page.evaluate(() => document.activeElement.id), 'settings');
  assert.equal(await page.locator('#debug-rom').textContent(), 'None');
  assert.equal(await page.locator('#debug-bytes').textContent(), '0');
  assert.equal(await page.locator('#debug-frames').textContent(), '0');
  await page.locator('#debug-close').click();
  await cancelPicker();
  assert.equal(await page.locator('#rom-status').textContent(), 'No ROM loaded');
  const countBeforeFile = requests.length;
  const longName = `${'wonder-swan-'.repeat(18)}cartridge.wsc`;
  await load({ name:longName, mimeType:'application/octet-stream', buffer:await readFile(rom) });
  assert.equal(await page.locator('#rom-title').getAttribute('title'), longName);
  assert.equal(await page.locator('#rom-title').evaluate(title => title.scrollWidth > title.clientWidth), true);
  assert.deepEqual(await checkLayout(), initialScreen, 'filename and playback status leave screen geometry stable');
  await load();
  assert.equal(await page.locator('#empty-state').isVisible(), false);
  const effectSelect = page.locator('#display-effect');
  await effectSelect.selectOption('retro-crt');
  await screenshot('desktop-playing');
  if (process.env.WS_SMOKE_SCREENSHOT) await page.screenshot({path:process.env.WS_SMOKE_SCREENSHOT,fullPage:true});
  await page.keyboard.down('ArrowRight');
  await expectInput(0,2);
  await effectSelect.focus();
  await expectInput();
  await page.keyboard.up('ArrowRight');
  await page.keyboard.press('c');
  assert.equal(await effectSelect.inputValue(),'clean');
  assert.equal(await page.locator('#screen').getAttribute('data-effect'),'clean');
  assert.equal(await page.evaluate(() => document.activeElement.id),'display-effect');
  assert.equal(await page.locator('#rom-status').textContent(),'Running');
  await page.keyboard.press('Escape');
  assert.equal(await page.evaluate(() => document.activeElement.id),'screen');
  await page.keyboard.down('ArrowRight');
  await expectInput(0,2);
  await page.keyboard.up('ArrowRight');
  await expectInput();
  for (const model of ['crystal','mono','color']) {
    if (model !== 'crystal') await page.locator('#model').selectOption(model);
    await initial();
    await page.keyboard.down('ArrowRight');
    await expectInput(0,2);
    await page.keyboard.up('ArrowRight');
    await expectInput();
  }
  await page.locator('#pause').click();
  const paused = await frame();
  await tap('ArrowRight');
  assert.equal(await frame(), paused);
  await page.locator('#pause').click();
  await page.keyboard.down('d');
  await expectInput(0,2);
  await page.locator('#reset').click();
  await page.keyboard.up('d');
  await initial();
  await page.keyboard.down('ArrowRight');
  await expectInput(0,2);
  await page.locator('#model').selectOption('crystal');
  await page.keyboard.up('ArrowRight');
  await initial();
  await visibility(true);
  assert.equal(await page.locator('#pause').textContent(), 'Resume');
  await visibility(false);
  assert.equal(await page.locator('#pause').textContent(), 'Resume');
  await page.locator('#pause').click();
  await initial();
  await cancelPicker();
  assert.equal(await page.locator('#rom-status').textContent(), 'Running');
  await load();
  assert.equal(requests.length, countBeforeFile, 'local ROM selection must not send requests');
  assert.equal(await page.locator('#rom-title').textContent(), 'keypad-horizontal.ws');
  await rejectEmptyRom();
  assert.equal(await page.getByRole('alert').filter({ hasText:'Could not load' }).isVisible(), true);
  assert.equal(await page.locator('#rom-title').textContent(), 'keypad-horizontal.ws', 'invalid file preserves cartridge');
  assert.equal(await page.locator('#rom-status').textContent(), 'Running');
  await load();
  assert.equal(await page.locator('#error').textContent(), '');
  console.log('Empty startup/cancellation, three-model local ROM CPU/input, pause/restart/model clearing, no uploads and invalid-file recovery passed');
  // Modal input isolation, native focus confinement, Escape, and conditional resume.
  await page.locator('#settings').focus();
  await page.keyboard.press('Enter');
  await page.waitForFunction(() => document.querySelector('#help').open);
  assert.equal(await page.locator('#pause').textContent(), 'Resume');
  const dialogFrame = await frame();
  await tap('ArrowRight');
  await tap('w');
  assert.equal(await frame(), dialogFrame);
  for (let i=0; i<20; i++) {
    await page.keyboard.press('Tab');
    // Native dialogs may send Tab to browser chrome (activeElement becomes body).
    assert.equal(await page.evaluate(() => document.activeElement === document.body || document.querySelector('#help').contains(document.activeElement)), true);
  }
  await closeHelp(true);
  assert.equal(await page.locator('#pause').textContent(), 'Pause');
  assert.equal(await page.evaluate(() => document.activeElement.id), 'screen');
  await initial();
  await page.locator('#pause').click();
  await openHelp();
  await closeHelp();
  assert.equal(await page.locator('#pause').textContent(), 'Resume', 'manually paused stays paused');
  assert.equal(await page.evaluate(() => document.activeElement.id), 'settings');
  await page.locator('#pause').click();
  await openHelp();
  await visibility(true);
  await visibility(false);
  await closeHelp(true);
  assert.equal(await page.locator('#pause').textContent(), 'Resume', 'background-paused modal never resumes');
  await page.locator('#pause').click();

  await page.keyboard.down('ArrowRight');
  await page.waitForTimeout(40);
  await page.locator('#settings').click();
  await page.keyboard.up('ArrowRight');
  await closeHelp();
  await expectInput();
  const releasedFrame = await frame();
  await page.waitForTimeout(200);
  assert.equal(await frame(), releasedFrame, 'overlay clears held keys');
  await page.locator('#reset').click();
  await initial();
  await page.keyboard.down('ArrowRight');
  await page.waitForTimeout(40);
  await page.locator('#model').focus();
  await page.keyboard.up('ArrowRight');
  await page.waitForTimeout(50);
  await expectInput();
  const focusFrame = await frame();
  await tap('d'); // no model label starts with D; native select typeahead does not change models
  assert.equal(await frame(), focusFrame, 'select focus clears and blocks gameplay keys');
  await page.locator('#reset').click();
  await initial();

  await openHelp();
  await page.locator('#debug-toggle').check();
  await closeHelp();
  assert.equal(await page.locator('#debug-panel').isVisible(), true);
  assert.equal(await page.locator('#debug-bytes').textContent(), '131072');
  assert.match(await page.locator('#debug-rom').textContent(), /keypad-horizontal.ws/);
  await page.waitForTimeout(550);
  assert.ok(Number(await page.locator('#debug-frames').textContent()) > 0);
  await page.locator('#debug-panel summary').click();
  assert.equal(await page.locator('#debug-panel details').getAttribute('open'), null);
  await page.locator('#debug-panel summary').click();
  await checkLayout();
  await screenshot('desktop-debug');
  await page.locator('#debug-close').click();
  assert.equal(await page.locator('#debug-panel').isVisible(), false);
  await openHelp();
  assert.equal(await page.locator('#debug-toggle').isChecked(), false);
  await page.locator('#debug-toggle').check();
  await page.locator('#debug-toggle').uncheck();
  await closeHelp(true);
  assert.equal(await page.locator('#debug-panel').isVisible(), false);
  console.log('Dialog focus/Escape, input isolation, conditional resume, and Debug Mode default/on/off passed');

  if (await page.evaluate(() => document.fullscreenEnabled)) {
    await page.locator('#fullscreen').click();
    await page.waitForFunction(() => !!document.fullscreenElement);
    const fullscreenFrame = await checkLayout();
    await screenshot('desktop-fullscreen');
    await openHelp();
    await closeHelp(true);
    assert.equal(await page.locator('#pause').textContent(), 'Pause');
    await page.locator('#pause').click();
    assert.deepEqual(await checkLayout(), fullscreenFrame, 'pausing preserves fullscreen geometry');
    await page.locator('#pause').click();
    await page.evaluate(() => document.exitFullscreen());
    await page.waitForFunction(() => !document.fullscreenElement && document.querySelector('#fullscreen').textContent === 'Fullscreen');
    assert.equal(await page.locator('#fullscreen').textContent(), 'Fullscreen');
    assert.equal(await page.evaluate(() => document.activeElement.id), 'screen');
    console.log('Native Chromium fullscreen entry/exit and aspect ratio passed');
  } else console.log('Native fullscreen unavailable; graceful fallback tested below');
  // Deterministically exercise unsupported and rejected Fullscreen API branches.
  await page.evaluate(() => Object.defineProperty(document, 'fullscreenEnabled', { configurable:true, value:false }));
  await page.locator('#fullscreen').click();
  assert.match(await page.locator('#notice').textContent(), /unavailable/);
  await page.evaluate(() => {
    Object.defineProperty(document, 'fullscreenEnabled', { configurable:true, value:true });
    document.querySelector('#player').requestFullscreen = () => Promise.reject(new Error('test rejection'));
  });
  await page.locator('#fullscreen').click();
  await page.waitForFunction(() => document.querySelector('#notice').textContent.includes('could not open'));
  assert.equal(await page.locator('#pause').textContent(), 'Pause');

  // A checksum-valid unsupported opcode exercises the real CPU error path.
  const broken = Buffer.from(await readFile(rom));
  broken[65536] = 0x0f;
  await page.locator('#rom').setInputFiles({ name:'unsupported.ws', mimeType:'application/octet-stream', buffer:checksum(broken) });
  await page.waitForFunction(() => document.querySelector('#error').textContent.includes('Execution stopped'));
  assert.equal(await page.locator('#rom-status').textContent(), 'Stopped');
  assert.equal(await page.locator('#pause').isDisabled(), true);
  await page.locator('#rom').setInputFiles(rom);
  await initial();
  console.log('Fullscreen fallback/rejection, stopped execution feedback, and recovery passed');

  assert.ok(requests.some(r => r.url.endsWith('.wasm')));
  assert.ok(requests.every(r => r.method === 'GET'));
  assert.ok(requests.every(r => !/\.(ws|wsc)(?:[?#]|$)/i.test(r.url)), 'no ROM network requests at startup or selection');
  assert.deepEqual(errors, []);
  console.log('Production base path, Wasm loading, local-only cartridges and no browser errors passed');

  // Isolated fault injection: no production network/service is modified.
  const recovery = await browser.newContext({ viewport:{width:812,height:375}, isMobile:true, hasTouch:true });
  await recovery.addInitScript(observeFrameProgress);
  await recovery.addInitScript(() => localStorage.setItem('wonderswan:display:v1','clean'));
  page = await recovery.newPage();
  page.on('pageerror', error => errors.push(error.message));
  let releaseWasm;
  const wasmGate = new Promise(resolve => { releaseWasm = resolve; });
  await page.route('**/*.wasm', async route => {
    await wasmGate;
    await route.fulfill({ status:503, body:'simulated unavailable Wasm' });
  });
  await page.goto(base);
  assert.equal(await page.locator('#rom-status').textContent(), 'Loading WebAssembly…');
  assert.equal(await page.locator('#load-rom').isDisabled(), true);
  assert.equal(await page.locator('[data-button]:enabled').count(), 0, 'touch controls disabled before initialization');
  releaseWasm();
  await page.waitForFunction(() => !document.querySelector('#retry').hidden);
  assert.match(await page.locator('#error').textContent(), /Could not initialize/);
  await checkInViewport('#error');
  await checkInViewport('#retry');
  await page.unroute('**/*.wasm');
  await page.locator('#retry').click();
  await page.waitForFunction(() => !document.querySelector('#model').disabled);
  assert.equal(await page.locator('#error').textContent(), '');

  // Gate actual local Blob reads to exercise load races without a ROM server.
  await page.evaluate(() => {
    const read = Blob.prototype.arrayBuffer;
    window.fileReads = [];
    Blob.prototype.arrayBuffer = function() {
      const file = this;
      return new Promise((resolve,reject) => window.fileReads.push({
        resolve:() => read.call(file).then(resolve,reject),
        reject:() => reject(new Error('simulated local read failure')),
      }));
    };
  });
  const localFile = {name:'local.wsc',mimeType:'application/octet-stream',buffer:await readFile(rom)};
  async function beginRead() {
    await page.locator('#rom').setInputFiles(localFile);
    await page.waitForFunction(() => window.fileReads.length > 0);
    assert.equal(await page.locator('#rom-status').textContent(), 'Loading ROM…');
  }
  const finishRead = (fail=false) => page.evaluate(fail => window.fileReads.shift()[fail ? 'reject' : 'resolve'](),fail);
  await beginRead();
  await finishRead(true);
  await page.waitForFunction(() => document.querySelector('#error').textContent.includes('simulated local read failure'));
  assert.equal(await page.locator('#rom-status').textContent(), 'No ROM loaded');
  assert.equal(await page.locator('#empty-state').isVisible(), true);
  await beginRead();
  await openHelp();
  await finishRead();
  await page.waitForFunction(() => document.querySelector('#rom-title').textContent === 'local.wsc');
  assert.equal(await page.locator('#pause').textContent(), 'Resume', 'late load stays paused behind modal');
  assert.equal(await page.evaluate(() => document.activeElement.id), 'help-close', 'late load preserves dialog focus');
  await closeHelp(true);
  await initial();
  await beginRead();
  await finishRead(true);
  await page.waitForFunction(() => document.querySelector('#error').textContent.includes('simulated local read failure'));
  assert.equal(await page.locator('#rom-title').textContent(), 'local.wsc', 'failed read preserves current ROM');
  assert.equal(await page.locator('#rom-status').textContent(), 'Running');
  await page.locator('#pause').click();
  await beginRead();
  await finishRead(true);
  await page.waitForFunction(() => document.querySelector('#error').textContent.includes('simulated local read failure'));
  assert.equal(await page.locator('#rom-status').textContent(), 'Paused', 'failure preserves prior pause');
  await page.locator('#pause').click();
  await beginRead();
  await visibility(true);
  await visibility(false);
  await finishRead();
  await page.waitForFunction(() => document.querySelector('#rom-status').textContent === 'Paused');
  await page.locator('#pause').click();
  await initial();

  // Restart/model changes and newer selections invalidate an older read.
  for (const action of ['reset','model','newer']) {
    await beginRead();
    if (action === 'newer') {
      await page.locator('#rom').setInputFiles({...localFile,name:'newer.ws'});
      await page.waitForFunction(() => window.fileReads.length === 2);
      await page.evaluate(() => window.fileReads.pop().resolve());
    } else if (action === 'model') await page.locator('#model').selectOption('mono');
    else await page.locator('#reset').click();
    await initial();
    const title = await page.locator('#rom-title').textContent();
    await finishRead(action === 'reset');
    await page.waitForTimeout(80);
    assert.equal(await page.locator('#rom-title').textContent(), title);
    assert.equal(await page.locator('#error').textContent(), '');
    assert.equal(await page.locator('#rom-status').textContent(), 'Running');
  }
  assert.deepEqual(errors, []);
  await recovery.close();
  console.log('Wasm initialization/retry, failed local reads, cartridge preservation and asynchronous read/modal/background/restart/replacement races passed');

  const mobile = await browser.newContext({ viewport:{width:375,height:812}, isMobile:true, hasTouch:true });
  await mobile.addInitScript(observeFrameProgress);
  await mobile.addInitScript(() => localStorage.setItem('wonderswan:display:v1','clean'));
  page = await mobile.newPage();
  page.on('pageerror', error => errors.push(error.message));
  await page.goto(base);
  await page.waitForFunction(() => !document.querySelector('#model').disabled);
  await checkLayout({ kind:'portrait' });
  assert.equal(await page.locator('#touch-controls').isVisible(), true);
  await screenshot('mobile-empty');
  assert.equal(await page.getByRole('slider',{name:'Volume',exact:true}).inputValue(),'70');
  await page.locator('#sound').tap();
  assert.equal(await page.locator('#sound').getAttribute('aria-pressed'),'true');
  await page.locator('#sound').tap();
  assert.equal(await page.locator('#sound').getAttribute('aria-pressed'),'false');
  await page.locator('#volume').tap();
  await load();
  await screenshot('mobile-playing');
  const cdp = await mobile.newCDPSession(page);
  async function touch(button, type) {
    const box = await page.locator(`[data-button="${button}"]`).boundingBox();
    await cdp.send('Input.dispatchTouchEvent', { type,
      touchPoints:type === 'touchEnd' || type === 'touchCancel' ? [] : [{x:box.x+box.width/2, y:box.y+box.height/2, id:1}] });
  }
  for (const [button,y,x,a] of [
    [16,0,1,0],[32,0,2,0],[64,0,4,0],[128,0,8,0],
    [256,1,0,0],[512,2,0,0],[1024,4,0,0],[2048,8,0,0],
    [4,0,0,4],[8,0,0,8],[2,0,0,2],
  ]) {
    await touch(button,'touchStart');
    await expectInput(y,x,a);
    assert.equal(await page.locator(`[data-button="${button}"]`).evaluate(b => b.classList.contains('is-pressed')),true);
    await touch(button,'touchEnd');
    await expectInput();
    assert.equal(await page.locator('.is-pressed').count(),0);
  }
  await touch(32,'touchStart');
  await expectInput(0,2);
  await page.waitForTimeout(200);
  await expectInput(0,2);
  await touch(32,'touchCancel');
  await expectInput();
  await touch(32,'touchStart');
  await expectInput(0,2);
  await page.evaluate(() => window.dispatchEvent(new Event('blur')));
  await touch(32,'touchEnd');
  await expectInput();
  await page.locator('#settings').tap();
  await page.locator('#debug-toggle').check();
  await screenshot('mobile-settings');
  const closeBox = await page.locator('#help-close').boundingBox();
  assert.ok(closeBox.y >= 0 && closeBox.y + closeBox.height <= 812, 'dialog close stays visible when scrolled');
  await closeHelp(true);
  await checkLayout({ kind:'portrait' });
  await screenshot('mobile-debug');
  await page.locator('#debug-close').tap();
  assert.equal(await page.locator('#debug-panel').isVisible(), false);
  await page.reload();
  await page.waitForFunction(() => !document.querySelector('#model').disabled);
  assert.equal(await page.locator('#debug-toggle').isChecked(), false);
  assert.equal(await page.locator('#debug-panel').isVisible(), false);
  const verticalRom = fileURLToPath(new URL('../../fixtures/generated/keypad-vertical.ws', import.meta.url));
  await load(verticalRom);
  await page.locator('#display-effect').selectOption('retro-crt');
  await page.evaluate(() => window.scrollTo(0,0));
  await checkLayout({ orientation:'vertical', kind:'portrait' });
  await screenshot('mobile-vertical');
  await page.setViewportSize({ width:375,height:736 });
  await page.evaluate(() => window.scrollTo(0,0));
  await checkLayout({ orientation:'vertical', kind:'portrait' });
  await checkInViewport('#screen');
  for (const button of await page.locator('#touch-controls button').all()) {
    await checkInViewport(`[data-button="${await button.getAttribute('data-button')}"]`);
  }
  await screenshot('mobile-vertical-reduced-height', false);
  await touch(2,'touchStart');
  await expectInput(0,0,2);
  await touch(2,'touchEnd');
  await expectInput();
  await page.setViewportSize({ width:375,height:812 });
  if (await page.evaluate(() => document.fullscreenEnabled)) {
    await page.locator('#fullscreen').tap();
    await page.waitForFunction(() => document.fullscreenElement?.id === 'player');
    await checkLayout({ orientation:'vertical', kind:'portrait' });
    const image = await page.locator('#screen').boundingBox();
    await rejectEmptyRom();
    await checkLayout({ orientation:'vertical', kind:'portrait' });
    assert.deepEqual(await page.locator('#screen').boundingBox(), image, 'feedback preserves portrait fullscreen picture geometry');
    await checkInViewport('#screen');
    await checkInViewport('#error');
    await screenshot('mobile-vertical-fullscreen-feedback', false);
    await page.locator('#fullscreen').tap();
    await page.waitForFunction(() => !document.fullscreenElement);
    await load(verticalRom);
  }
  await page.setViewportSize({ width:812,height:375 });
  await page.evaluate(() => window.scrollTo(0,0));
  await checkLayout({ orientation:'vertical', kind:'landscape' });
  await screenshot('mobile-landscape-vertical', false);
  await rejectEmptyRom();
  assert.equal(await page.evaluate(() => window.scrollY), 0, 'invalid ROM does not require scrolling to see feedback');
  await checkInViewport('#error');
  const points = [];
  for (const [index, button] of [16,512,4].entries()) {
    const box = await page.locator(`[data-button="${button}"]`).boundingBox();
    assert.ok(box.y >= 0 && box.y + box.height <= 375, 'landscape controls fit without scrolling');
    points.push({ id:index+1,x:box.x+box.width/2,y:box.y+box.height/2 });
    await cdp.send('Input.dispatchTouchEvent', { type:'touchStart',touchPoints:points });
  }
  await expectInput(2,1,4);
  assert.equal(await page.locator('.is-pressed').count(),3);
  await cdp.send('Input.dispatchTouchEvent', { type:'touchCancel',touchPoints:[] });
  await expectInput();
  assert.equal(await page.locator('.is-pressed').count(),0);
  await load();
  await page.evaluate(() => window.scrollTo(0,0));
  await checkLayout({ kind:'landscape' });
  await screenshot('mobile-landscape', false);
  if (await page.evaluate(() => document.fullscreenEnabled)) {
    await page.locator('#fullscreen').tap();
    await page.waitForFunction(() => document.fullscreenElement?.id === 'player');
    await checkLayout({ kind:'landscape' });
    await screenshot('mobile-landscape-fullscreen', false);
    await touch(2048,'touchStart');
    await expectInput(8);
    await touch(2048,'touchEnd');
    await expectInput();
    await page.locator('#fullscreen').tap();
    await page.waitForFunction(() => !document.fullscreenElement);
  }
  assert.deepEqual(errors, []);
  console.log('375×812 portrait and 812×375 landscape layout, both cartridge orientations, fullscreen, all touch inputs and combined presses, and mobile Help/Debug passed');
  await mobile.close();

} finally {
  await browser.close();
}
