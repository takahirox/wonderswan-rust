// Production-preview browser acceptance. Run after building and starting Vite preview.
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { fileURLToPath } from 'node:url';
import { readFile, mkdir } from 'node:fs/promises';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { romLibraryChecks } from './rom-library.mjs';

const base = process.env.WS_PREVIEW_URL ?? 'http://127.0.0.1:4173/wonderswan-rust/';
const rom = fileURLToPath(new URL('../../fixtures/generated/keypad-horizontal.ws', import.meta.url));
const verticalRom = fileURLToPath(new URL('../../fixtures/generated/keypad-vertical.ws', import.meta.url));
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
  await page.locator('#open-local-rom').click();
  await (await chooser).setFiles(file);
  await initial();
}
async function cancelPicker() {
  const chooser = page.waitForEvent('filechooser');
  await page.locator('#load-rom').click();
  await page.locator('#open-local-rom').click();
  await (await chooser).setFiles([]);
  assert.equal(await page.evaluate(() => document.activeElement.id), 'load-rom');
}
async function rejectEmptyRom() {
  const chooser = page.waitForEvent('filechooser');
  await page.locator('#load-rom').click();
  await page.locator('#open-local-rom').click();
  await (await chooser).setFiles(emptyRom);
  await page.waitForFunction(() => document.querySelector('#error').textContent.includes('Expected a 128 KiB'));
}
async function checkInViewport(selector) {
  const box = await page.locator(selector).boundingBox();
  const {width, height} = page.viewportSize();
  assert.ok(box && box.width > 0 && box.height > 0 && box.x >= 0 && box.y >= 0
    && box.x + box.width <= width && box.y + box.height <= height,
  `${selector} (${JSON.stringify(box)}) is fully visible in the ${width}×${height} viewport`);
  return box;
}
function luminance(color) {
  const [red, green, blue] = color.match(/\d+/g).map(value => {
    const channel = Number(value) / 255;
    return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
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
      toolbar:rect('#controls'), cartridge:rect('.cartridge'),
      xPad:rect('.x-pad'), yPad:rect('.y-pad'),
      touch:[...document.querySelectorAll('#touch-controls button')].map(button => ({
        mask:Number(button.dataset.button), ...button.getBoundingClientRect().toJSON(),
      })),
      overflow:document.documentElement.scrollWidth > innerWidth,
      backing:[canvas.width,canvas.height], pixels:getComputedStyle(canvas).imageRendering,
    };
  });
  const { image, frame, effect, toolbar, cartridge, touch, xPad, yPad } = layout;
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
    const overlaps = (a,b) => a.top < b.bottom && a.bottom > b.top && a.left < b.right && a.right > b.left;
    assert.deepEqual(touch.map(b => b.mask).sort((a,b) => a-b), [2,4,8,16,32,64,128,256,512,1024,2048]);
    const button = mask => touch.find(b => b.mask === mask);
    const a = button(4), b = button(8), start = button(2);
    for (const [index,target] of touch.entries()) {
      assert.ok(target.width >= 44 && target.height >= 44, `button ${target.mask} has a practical touch target`);
      assert.ok(target.left >= 0 && target.right <= page.viewportSize().width && target.bottom <= toolbar.top,
        `button ${target.mask} fits horizontally and precedes the toolbar`);
      assert.ok(!overlaps(target,frame), `button ${target.mask} clears the screen`);
      assert.ok(touch.slice(index+1).every(other => !overlaps(target,other)), `button ${target.mask} clears other targets`);
    }
    for (const masks of [[16,32,64,128],[256,512,1024,2048]]) {
      const order = orientation === 'horizontal' ? masks : [...masks.slice(1),masks[0]];
      const [up,right,down,left] = order.map(button);
      assert.ok(up.top < left.top && left.top === right.top && down.top > left.top);
      assert.ok(left.left < up.left && up.left === down.left && down.left < right.left);
    }
    assert.ok(a.top < b.top, 'A is above B');
    if (orientation === 'horizontal') {
      assert.ok(yPad.bottom < xPad.top && yPad.left === xPad.left && xPad.right < b.left && a.left > b.left);
      if (kind === 'landscape') {
        assert.ok(xPad.right < frame.left && frame.right < b.left && frame.right < start.left);
      } else {
        assert.ok(touch.every(target => target.top >= frame.bottom), 'horizontal controller follows the screen');
      }
    } else {
      assert.ok(yPad.right < xPad.left && yPad.top === xPad.top && a.left < b.left);
      assert.ok(frame.right <= a.left && frame.right <= start.left && b.bottom <= yPad.top && start.bottom <= xPad.top);
      if (kind !== 'landscape') assert.ok(frame.bottom <= yPad.top, 'vertical keypads are below the screen');
    }
  }
  for (const selector of ['#model','#pause','#reset','#load-rom','#fullscreen','#settings','#sound','#volume','#display-effect']) {
    const box = await page.locator(selector).boundingBox();
    const viewport = page.viewportSize();
    assert.ok(box && box.x >= 0 && box.x + box.width <= viewport.width, `${selector} fits viewport`);
    assert.ok(box.height >= 44, `${selector} has practical touch target`);
    if (kind === 'landscape' && !['#model','#load-rom'].includes(selector)) {
      assert.ok(box.y >= 0 && box.y + box.height <= viewport.height, `${selector} fits the first landscape viewport`);
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
  assert.equal(await page.getByRole('button', {name:'Insert cartridge',exact:true}).count(), 1);
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
  assert.equal(await page.locator('#rom-title').textContent(), longName, 'reimport leaves the inserted name intact');
  await rejectEmptyRom();
  assert.equal(await page.getByRole('alert').filter({ hasText:'Could not load' }).isVisible(), true);
  assert.equal(await page.locator('#rom-title').textContent(), longName, 'invalid file preserves cartridge');
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
  assert.equal(await page.locator('#debug-rom').textContent(), longName);
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
    await page.locator('#sound').click();
    await load(verticalRom);
    const verticalFrame = await checkLayout({ orientation:'vertical' });
    const image = await page.locator('#screen').boundingBox();
    await rejectEmptyRom();
    await checkInViewport('#error');
    assert.deepEqual(await checkLayout({ orientation:'vertical' }), verticalFrame, 'feedback preserves desktop fullscreen frame geometry');
    assert.deepEqual(await page.locator('#screen').boundingBox(), image, 'feedback preserves desktop fullscreen picture geometry');
    await load();
    await page.locator('#sound').click();
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
  assert.equal(await page.locator('#touch-controls .dpad:visible').count(), 2);
  await screenshot('mobile-empty');
  assert.equal(await page.getByRole('slider',{name:'Volume',exact:true}).inputValue(),'70');
  await page.locator('#sound').tap();
  assert.equal(await page.locator('#sound').getAttribute('aria-pressed'),'true');
  await page.locator('#sound').tap();
  assert.equal(await page.locator('#sound').getAttribute('aria-pressed'),'false');
  await page.locator('#volume').tap();
  await load();
  await checkLayout({ kind:'portrait' });
  await screenshot('mobile-playing', false);
  const mobileEffect = page.locator('#display-effect');
  for (const effect of ['clean','retro-crt']) {
    await mobileEffect.selectOption(effect);
    await mobileEffect.focus();
    await page.keyboard.press('Escape');
    assert.equal(await page.locator('#screen:focus-visible').getAttribute('data-effect'), effect);
    const { style, width, color, background } = await page.locator('#display').evaluate(frame => {
      const { outlineStyle, outlineWidth, outlineColor } = getComputedStyle(frame);
      return { style:outlineStyle, width:outlineWidth, color:outlineColor,
        background:getComputedStyle(document.documentElement).backgroundColor };
    });
    assert.equal(style, 'solid', `${effect} has a visible keyboard focus ring`);
    assert.ok(parseFloat(width) >= 2, `${effect} has a clear outline`);
    const contrast = [luminance(color), luminance(background)].sort((a,b) => a-b);
    assert.ok((contrast[1]+0.05)/(contrast[0]+0.05) >= 3, `${effect} focus ring contrasts with the page`);
  }
  await mobileEffect.selectOption('clean');
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
  await load(verticalRom);
  await page.locator('#display-effect').selectOption('retro-crt');
  await page.evaluate(() => window.scrollTo(0,0));
  await checkLayout({ orientation:'vertical', kind:'portrait' });
  await screenshot('mobile-vertical', false);
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
    await checkInViewport('#error');
    await checkInViewport('#load-rom');
    await page.evaluate(() => { document.fullscreenElement.scrollTop = 0; });
    await checkLayout({ orientation:'vertical', kind:'portrait' });
    assert.deepEqual(await page.locator('#screen').boundingBox(), image, 'feedback preserves portrait fullscreen picture geometry');
    await checkInViewport('#screen');
    await checkInViewport('#error');
    await screenshot('mobile-vertical-fullscreen-feedback', false);
    await page.locator('#fullscreen').tap();
    await page.waitForFunction(() => !document.fullscreenElement);
    await load(verticalRom);
  }
  await page.setViewportSize({ width:680,height:340 });
  async function checkGameplayInViewport() {
    assert.equal(await page.evaluate(() => document.activeElement.id), 'screen');
    assert.equal(await page.evaluate(() => {
      const container = document.fullscreenElement ?? document.documentElement;
      return container.scrollWidth > container.clientWidth;
    }), false, 'no horizontal overflow during landscape play');
    await checkInViewport('#screen');
    const buttons = await page.locator('#touch-controls button').all();
    assert.equal(buttons.length, 11);
    for (const button of buttons) {
      const selector = `[data-button="${await button.getAttribute('data-button')}"]`;
      const box = await checkInViewport(selector);
      assert.ok(box.width >= 44 && box.height >= 44, `${selector} has a practical touch target`);
    }
  }
  async function checkCompactLandscape(mode) {
    const screen = page.locator('#display');
    const chooser = page.waitForEvent('filechooser');
    await page.locator('#load-rom').tap();
    await page.locator('#open-local-rom').tap();
    assert.ok((await screen.boundingBox()).y < 0, `680×340 ${mode} picker opens from a clipped screen`);
    await (await chooser).setFiles(verticalRom);
    await initial();
    await checkGameplayInViewport();

    const model = page.locator('#model');
    await model.scrollIntoViewIfNeeded();
    assert.ok((await screen.boundingBox()).y < 0, `680×340 ${mode} Console is reached from a clipped screen`);
    await model.selectOption(mode === 'windowed' ? 'color' : 'crystal');
    await initial();
    await checkGameplayInViewport();

    const screenBox = await screen.boundingBox();
    const toolbar = await page.locator('#controls').boundingBox();
    assert.ok(screenBox && toolbar && screenBox.y + screenBox.height <= toolbar.y,
      `680×340 ${mode} portrait cartridge leaves the toolbar clear`);
    const mute = page.locator('#sound');
    const wasMuted = await mute.getAttribute('aria-pressed');
    // The screen previously covered the top of this button.
    await mute.tap({ position:{x:10,y:3}, timeout:3000 });
    assert.equal(await mute.getAttribute('aria-pressed'), wasMuted === 'true' ? 'false' : 'true');
  }
  await checkCompactLandscape('windowed');
  if (await page.evaluate(() => document.fullscreenEnabled)) {
    await page.locator('#fullscreen').tap();
    await page.waitForFunction(() => document.fullscreenElement?.id === 'player');
    await checkCompactLandscape('fullscreen');
    await page.evaluate(() => document.exitFullscreen());
    await page.waitForFunction(() => !document.fullscreenElement);
  }
  await page.setViewportSize({ width:640,height:360 });
  await load(verticalRom);
  await checkGameplayInViewport();
  if (await page.evaluate(() => document.fullscreenEnabled)) {
    await page.locator('#fullscreen').tap();
    await page.waitForFunction(() => document.fullscreenElement?.id === 'player');
    await checkGameplayInViewport();
    await page.evaluate(() => document.exitFullscreen());
    await page.waitForFunction(() => !document.fullscreenElement);
  }
  console.log('640×360 portrait cartridge and all 11 touch buttons fit together during landscape play');
  await page.setViewportSize({ width:812,height:375 });
  await page.evaluate(() => window.scrollTo(0,0));
  await checkLayout({ orientation:'vertical', kind:'landscape' });
  await screenshot('mobile-landscape-vertical', false);
  await rejectEmptyRom();
  assert.equal(await page.evaluate(() => document.activeElement.id), 'load-rom');
  await checkInViewport('#error');
  await checkInViewport('#load-rom');
  await page.evaluate(() => window.scrollTo(0,0));
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
  await page.setViewportSize({ width:844,height:520 });
  await page.evaluate(() => window.scrollTo(0,0));
  await checkGameplayInViewport();
  await checkLayout({ kind:'landscape' });

  await page.setViewportSize({ width:320,height:568 });
  async function checkSafeArea(orientation, inset) {
    await cdp.send('Emulation.setSafeAreaInsetsOverride', { insets:{top:24,bottom:24,left:inset,right:inset} });
    await page.evaluate(() => window.scrollTo(0,0));
    await checkLayout({ orientation, kind:'portrait' });
    assert.deepEqual(await page.locator('main').evaluate(main => {
      const style = getComputedStyle(main);
      return [style.paddingTop,style.paddingRight,style.paddingBottom,style.paddingLeft];
    }), ['24px',`${inset}px`,'24px',`${inset}px`]);
    for (const target of await page.locator('#display, #touch-controls button').all()) {
      const box = await target.boundingBox();
      assert.ok(box.x >= inset && box.x + box.width <= 320 - inset,
        `${orientation} game and touch targets respect ${inset}px side insets`);
    }
  }
  await checkSafeArea('horizontal',24);
  await load(verticalRom);
  await checkSafeArea('vertical',30);
  await page.locator('#load-rom').scrollIntoViewIfNeeded();
  await checkInViewport('#load-rom');
  assert.deepEqual(errors, []);
  console.log('375×812 and 320×568 portrait (including safe-area insets), 812×375 and 844×520 landscape, both cartridge orientations, fullscreen, touch inputs and mobile Help/Debug passed');
  await mobile.close();
  await romLibraryChecks(browser,base,errors);
  assert.deepEqual(errors, []);

} finally {
  await browser.close();
}
