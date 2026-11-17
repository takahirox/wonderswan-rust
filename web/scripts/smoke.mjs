// Production-preview browser acceptance. Run after building and starting Vite preview.
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { fileURLToPath } from 'node:url';
import { readFile, mkdir } from 'node:fs/promises';
import { join } from 'node:path';
import { createHash } from 'node:crypto';

const base = process.env.WS_PREVIEW_URL ?? 'http://127.0.0.1:4173/wonderswan-rust/';
const rom = fileURLToPath(new URL('../../fixtures/generated/keypad-horizontal.ws', import.meta.url));
const browser = await chromium.launch({ headless: true });
let page = await browser.newPage({ viewport: { width:1440, height:900 } }); // fresh, unauthenticated context
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
}
// Wait for two elapsed rendering intervals with input held/released. A wall
// timeout can expire without a tick, and the first callback after load/resume
// only initializes the clock. Three callbacks allow the ROM to observe input.
async function inputFrames() {
  await page.evaluate(() => new Promise(resolve => {
    requestAnimationFrame(() => requestAnimationFrame(() => requestAnimationFrame(resolve)));
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
async function screenshot(name) {
  if (artifacts) await page.screenshot({ path:join(artifacts, `${name}.png`), fullPage:true });
}
async function checkLayout() {
  const layout = await page.locator('#screen').evaluate(canvas => {
    const box = canvas.getBoundingClientRect();
    return { width:box.width, height:box.height, overflow:document.documentElement.scrollWidth > innerWidth,
      pixels:getComputedStyle(canvas).imageRendering };
  });
  assert.equal(layout.overflow, false, 'no horizontal overflow');
  assert.ok(Math.abs(layout.width / layout.height - 224/144) < 0.01, 'screen aspect ratio');
  assert.equal(layout.pixels, 'pixelated');
  for (const id of ['model','pause','reset','load-rom','fullscreen','settings','sound','volume']) {
    const box = await page.locator(`#${id}`).boundingBox();
    const viewport = page.viewportSize();
    assert.ok(box.x >= 0 && box.x + box.width <= viewport.width, `${id} fits viewport`);
    assert.ok(box.height >= 44, `${id} has practical touch target`);
  }
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
  assert.equal(await page.locator('#pause').textContent(), 'Resume');
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
  await page.locator('#settings').click();
  await page.locator('#display-effect').selectOption('clean');
  await closeHelp();
  assert.equal(await page.locator('#debug-toggle').isChecked(), false);
  assert.equal(await page.locator('#debug-panel').isVisible(), false);
  assert.equal(await page.locator('#help').isVisible(), false);
  assert.equal(await page.locator('#touch-controls').isVisible(), false);
  await checkLayout();
  await screenshot('desktop-empty');
  assert.equal(await page.locator('#model').inputValue(), 'crystal');
  assert.equal(await page.locator('#model option').count(), 3);
  assert.equal(await page.locator('#screen').getAttribute('width'), '224');
  assert.equal(await page.locator('#screen').getAttribute('height'), '144');
  assert.equal(await page.locator('#rom-status').textContent(), 'No ROM loaded');
  assert.equal(await page.locator('#empty-state').isVisible(), true);
  assert.equal(await page.getByRole('button', {name:'Load ROM',exact:true}).count(), 1);
  assert.equal(await page.locator('#rom').getAttribute('accept'), '.ws,.wsc');
  assert.equal(await page.locator('#pause').isDisabled(), true);
  assert.equal(await page.locator('#reset').isDisabled(), true);
  await openHelp();
  await page.locator('#debug-toggle').check();
  await closeHelp();
  assert.equal(await page.locator('#debug-rom').textContent(), 'None');
  assert.equal(await page.locator('#debug-bytes').textContent(), '0');
  assert.equal(await page.locator('#debug-frames').textContent(), '0');
  await page.locator('#debug-close').click();
  await cancelPicker();
  assert.equal(await page.locator('#rom-status').textContent(), 'No ROM loaded');
  const countBeforeFile = requests.length;
  await load();
  assert.equal(await page.locator('#empty-state').isVisible(), false);
  await screenshot('desktop-playing');
  if (process.env.WS_SMOKE_SCREENSHOT) await page.screenshot({path:process.env.WS_SMOKE_SCREENSHOT,fullPage:true});
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
  await page.locator('#rom').setInputFiles({name:'empty.ws',mimeType:'application/octet-stream',buffer:Buffer.alloc(0)});
  await page.waitForFunction(() => document.querySelector('#error').textContent.includes('Expected a 128 KiB'));
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
    await checkLayout();
    await screenshot('desktop-fullscreen');
    await openHelp();
    await closeHelp(true);
    assert.equal(await page.locator('#pause').textContent(), 'Pause');
    await page.locator('#fullscreen').click();
    await page.waitForFunction(() => !document.fullscreenElement && document.querySelector('#fullscreen').textContent === 'Fullscreen');
    assert.equal(await page.locator('#fullscreen').textContent(), 'Fullscreen');
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
  const recovery = await browser.newContext({ viewport:{width:1440,height:900} });
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
  await mobile.addInitScript(() => localStorage.setItem('wonderswan:display:v1','clean'));
  page = await mobile.newPage();
  page.on('pageerror', error => errors.push(error.message));
  await page.goto(base);
  await page.waitForFunction(() => !document.querySelector('#model').disabled);
  await checkLayout();
  assert.equal(await page.locator('#touch-controls').isVisible(), true);
  await screenshot('mobile-empty');
  assert.equal(await page.getByRole('slider',{name:'Volume',exact:true}).inputValue(),'70');
  await page.locator('#sound').tap();
  assert.equal(await page.locator('#sound').textContent(),'Unmute');
  await page.locator('#sound').tap();
  assert.equal(await page.locator('#sound').textContent(),'Mute');
  await page.locator('#volume').tap();
  await load();
  await screenshot('mobile-playing');
  const cdp = await mobile.newCDPSession(page);
  async function touch(button, type) {
    const box = await page.locator(`[data-button="${button}"]`).boundingBox();
    await cdp.send('Input.dispatchTouchEvent', { type,
      touchPoints:type === 'touchEnd' || type === 'touchCancel' ? [] : [{x:box.x+box.width/2, y:box.y+box.height/2, id:1}] });
  }
  for (const [button,y,x,a] of [[16,0,1,0],[32,0,2,0],[64,0,4,0],[128,0,8,0],[4,0,0,4],[8,0,0,8],[2,0,0,2]]) {
    await touch(button,'touchStart');
    await expectInput(y,x,a);
    await touch(button,'touchEnd');
    await expectInput();
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
  await checkLayout();
  await screenshot('mobile-debug');
  await page.locator('#debug-close').tap();
  assert.equal(await page.locator('#debug-panel').isVisible(), false);
  await page.reload();
  await page.waitForFunction(() => !document.querySelector('#model').disabled);
  assert.equal(await page.locator('#debug-toggle').isChecked(), false);
  assert.equal(await page.locator('#debug-panel').isVisible(), false);
  assert.deepEqual(errors, []);
  console.log('375×812 layout, real touch directions/A/B/Start, hold/release/cancel/blur, and mobile settings/debug passed');
  await mobile.close();

} finally {
  await browser.close();
}
