// Production-preview browser acceptance. Run after building and starting Vite preview.
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { fileURLToPath } from 'node:url';
import { readFile, mkdir } from 'node:fs/promises';
import { join } from 'node:path';
import { createHash } from 'node:crypto';

const base = process.env.WS_PREVIEW_URL ?? 'http://127.0.0.1:4173/wonderswan-rust/';
const rom = fileURLToPath(new URL('../public/demo/beacon-trail.ws', import.meta.url));
const browser = await chromium.launch({ headless: true });
let page = await browser.newPage({ viewport: { width:1440, height:900 } }); // fresh, unauthenticated context
const artifacts = process.env.WS_SMOKE_ARTIFACTS;
if (artifacts) await mkdir(artifacts, { recursive:true });
const errors = [];
const requests = [];
page.on('pageerror', error => errors.push(error.message));
page.on('request', request => requests.push({ url: request.url(), method: request.method() }));
page.on('response', response => { if (response.status() >= 400) errors.push(`${response.status()} ${response.url()}`); });

const player = [0,126,66,90,90,66,126,0];
const glyphs = {
  '0': ['111','101','101','101','111'], '1': ['010','110','010','010','111'],
  '2': ['110','001','010','100','111'], '3': ['110','001','010','001','110'],
  'A': ['010','101','111','101','101'],
};
function glyph(char) {
  return [0, ...glyphs[char].map(row => parseInt(`0${[...row].map(b => b+b).join('')}0`, 2)), 0, 0];
}
async function tile(x, y) {
  return page.locator('#screen').evaluate((canvas, {x,y}) => {
    const data = canvas.getContext('2d').getImageData(x*8,y*8,8,8).data;
    return Array.from({length:8}, (_, row) => {
      let bits = 0;
      for (let col=0; col<8; col++) if (data[(row*8+col)*4] < 128) bits |= 128 >> col;
      return bits;
    });
  }, {x,y});
}
async function expectTile(x, y, expected) {
  // Allow startup/rendering time, but fail rather than skipping an absent frame.
  for (let attempt=0; attempt<100; attempt++) {
    if (JSON.stringify(await tile(x,y)) === JSON.stringify(expected)) return;
    const error = await page.locator('#error').textContent();
    assert.equal(error, '', `runtime error: ${error}`);
    await page.waitForTimeout(20);
  }
  assert.deepEqual(await tile(x,y), expected, `tile ${x},${y}`);
}
async function tap(key) {
  await page.keyboard.down(key);
  await page.waitForTimeout(45);
  await page.keyboard.up(key);
  await page.waitForTimeout(45);
}
async function initial() {
  await expectTile(3,8,player);
  await expectTile(21,1,glyph('0'));
}
async function complete() {
  for (let i=0;i<6;i++) await tap('ArrowRight');
  await expectTile(21,1,glyph('1'));
  for (let i=0;i<3;i++) await tap('ArrowUp');
  await expectTile(21,1,glyph('2'));
  for (let i=0;i<9;i++) await tap('ArrowRight');
  await expectTile(18,5,player);
  await expectTile(21,1,glyph('3'));
  await expectTile(9,10,glyph('A')); // ALL FOUND! is rendered by ROM tile writes
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
  await page.locator('#demo').click();
  await initial();
  await screenshot('desktop-playing');
  for (const model of ['crystal','mono','color']) {
    if (model !== 'crystal') await page.locator('#model').selectOption(model);
    await initial();
    await complete();
    if (model === 'crystal' && process.env.WS_SMOKE_SCREENSHOT) {
      await page.screenshot({ path: process.env.WS_SMOKE_SCREENSHOT, fullPage:true });
    }
    await tap('Enter');
    await initial();
    console.log(`${model}: demo boot, move, score 1/2/3, win, cartridge Start restart passed`);
  }
  await tap('ArrowRight');
  await expectTile(4,8,player);
  await page.locator('#pause').click();
  const paused = await frame();
  await tap('ArrowRight');
  assert.equal(await frame(), paused);
  await page.locator('#pause').click();
  await tap('ArrowDown');
  await expectTile(4,9,player);
  await page.locator('#reset').click();
  await initial();
  await page.keyboard.down('ArrowRight');
  await page.waitForTimeout(45);
  await page.locator('#model').selectOption('crystal');
  await page.keyboard.up('ArrowRight');
  await initial();
  await page.waitForTimeout(150);
  await initial(); // held key must not leak across reset
  // Exercise the visibility handler deterministically in headless Chromium.
  // This is synthetic visibility, not a claim of testing a physical tab switch.
  await page.evaluate(() => {
    Object.defineProperty(document, 'hidden', { configurable:true, value:true });
    document.dispatchEvent(new Event('visibilitychange'));
  });
  assert.equal(await page.locator('#pause').textContent(), 'Resume');
  await page.evaluate(() => {
    delete document.hidden;
    document.dispatchEvent(new Event('visibilitychange'));
  });
  assert.equal(await page.locator('#pause').textContent(), 'Resume');
  await page.locator('#pause').click();
  await tap('d');
  await expectTile(4,8,player);
  await page.locator('#reset').click();
  await initial();
  console.log('Pause/resume, UI restart, model-switch input clearing, WASD, and synthetic visibility pause passed');

  const countBeforeFile = requests.length;
  await page.locator('#rom').setInputFiles(rom);
  await initial();
  await complete();
  assert.equal(requests.length, countBeforeFile, 'local ROM selection must not send requests');
  assert.match(await page.locator('#rom-title').textContent(), /beacon-trail.ws/);
  await page.locator('#rom').setInputFiles({name:'empty.ws',mimeType:'application/octet-stream',buffer:Buffer.alloc(0)});
  // File reading and ROM identity hashing finish after the change event.
  await page.waitForFunction(() => document.querySelector('#error').textContent.includes('Expected a 128 KiB'));
  assert.match(await page.locator('#error').textContent(), /Expected a 128 KiB/);
  await page.locator('#rom').setInputFiles(rom);
  await initial();
  assert.equal(await page.locator('#error').textContent(), '');
  console.log('Local cartridge playthrough, no uploads, invalid-file recovery, same-file reload passed');
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
  assert.match(await page.locator('#debug-rom').textContent(), /beacon-trail.ws/);
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

  // Real cartridge execution error, using the same checksum-valid mutation as Rust tests.
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
  assert.ok(requests.some(r => r.url.endsWith('/wonderswan-rust/demo/beacon-trail.ws')));
  assert.ok(requests.every(r => r.method === 'GET'));
  assert.deepEqual(errors, []);
  const notice = await page.request.get(new URL('demo/NOTICE.txt', base).href);
  assert.equal(notice.status(),200);
  assert.match(await notice.text(), /CC0-1.0/);
  console.log('Production base path, Wasm/ROM loading, license notice, and no browser errors passed');

  // Isolated fault injection: no production network/service is modified.
  const recovery = await browser.newContext({ viewport:{width:1440,height:900} });
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
  assert.equal(await page.locator('#demo').isDisabled(), true);
  assert.equal(await page.locator('[data-button]:enabled').count(), 0, 'touch controls disabled before initialization');
  releaseWasm();
  await page.waitForFunction(() => !document.querySelector('#retry').hidden);
  assert.match(await page.locator('#error').textContent(), /Could not initialize/);
  await page.unroute('**/*.wasm');
  await page.locator('#retry').click();
  await page.waitForFunction(() => !document.querySelector('#model').disabled);
  assert.equal(await page.locator('#error').textContent(), '');

  await page.route('**/demo/beacon-trail.ws', route => route.fulfill({ status:503, body:'simulated unavailable demo' }));
  await page.locator('#demo').click();
  await page.waitForFunction(() => document.querySelector('#error').textContent.includes('Demo request failed'));
  assert.equal(await page.locator('#demo').isDisabled(), false);
  await page.unroute('**/demo/beacon-trail.ws');

  let releaseDemo;
  let demoGate = new Promise(resolve => { releaseDemo = resolve; });
  await page.route('**/demo/beacon-trail.ws', async route => { await demoGate; await route.continue(); });
  await page.locator('#demo').click();
  assert.equal(await page.locator('#rom-status').textContent(), 'Loading ROM…');
  await openHelp();
  releaseDemo();
  await page.waitForFunction(() => document.querySelector('#rom-title').textContent === 'Beacon Trail');
  assert.equal(await page.locator('#pause').textContent(), 'Resume', 'late load stays paused behind modal');
  assert.equal(await page.evaluate(() => document.activeElement.id), 'help-close', 'late load preserves dialog focus');
  await closeHelp(true);
  assert.equal(await page.locator('#pause').textContent(), 'Pause');
  await initial();
  // The settings demo action must close the dialog before starting a fresh load.
  await openHelp();
  await page.locator('#demo-again').click();
  await page.waitForFunction(() => !document.querySelector('#help').open && document.querySelector('#rom-status').textContent === 'Running');
  await initial();

  demoGate = new Promise(resolve => { releaseDemo = resolve; });
  await openHelp();
  await page.locator('#demo-again').click();
  await page.waitForFunction(() => document.querySelector('#rom-status').textContent === 'Loading ROM…');
  await visibility(true);
  await visibility(false);
  releaseDemo();
  await page.waitForFunction(() => document.querySelector('#rom-status').textContent === 'Paused');
  assert.equal(await page.locator('#pause').textContent(), 'Resume', 'late load respects background pause');
  await page.locator('#pause').click();
  await initial();
  assert.deepEqual(errors, []);
  await recovery.close();
  console.log('Wasm initialization/loading/retry, failed demo recovery, and asynchronous load/modal/background races passed');

  const mobile = await browser.newContext({ viewport:{width:375,height:812}, isMobile:true, hasTouch:true });
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
  await page.locator('#demo').tap();
  await initial();
  await screenshot('mobile-playing');
  const cdp = await mobile.newCDPSession(page);
  async function touch(button, type) {
    const box = await page.locator(`[data-button="${button}"]`).boundingBox();
    await cdp.send('Input.dispatchTouchEvent', { type,
      touchPoints:type === 'touchEnd' || type === 'touchCancel' ? [] : [{x:box.x+box.width/2, y:box.y+box.height/2, id:1}] });
  }
  async function touchTap(button) {
    await touch(button,'touchStart');
    await page.waitForTimeout(45);
    await touch(button,'touchEnd');
    await page.waitForTimeout(45);
  }
  await touchTap(32);
  await expectTile(4,8,player);
  await touchTap(16);
  await expectTile(4,7,player);
  await touchTap(128);
  await expectTile(3,7,player);
  await touchTap(64);
  await initial();
  await touch(32,'touchStart');
  await page.waitForTimeout(300);
  await touch(32,'touchEnd');
  assert.notDeepEqual(await tile(4,8), player, 'holding repeats movement');
  const touchReleased = await frame();
  await page.waitForTimeout(200);
  assert.equal(await frame(), touchReleased);
  await touchTap(2);
  await initial();
  const up = await page.locator('[data-button="16"]').boundingBox();
  const right = await page.locator('[data-button="32"]').boundingBox();
  await cdp.send('Input.dispatchTouchEvent', { type:'touchStart', touchPoints:[
    {x:up.x+22,y:up.y+22,id:1}, {x:right.x+22,y:right.y+22,id:2},
  ] });
  await page.waitForTimeout(45);
  await cdp.send('Input.dispatchTouchEvent', { type:'touchEnd',touchPoints:[] });
  await expectTile(4,7,player);
  assert.equal(await page.evaluate(() => scrollY), 0, 'touch controller does not scroll the page');
  await touchTap(2);
  await initial();
  await touch(32,'touchStart');
  await page.waitForTimeout(45);
  await touch(32,'touchCancel');
  const canceled = await frame();
  await page.waitForTimeout(200);
  assert.equal(await frame(), canceled, 'pointer cancel releases');
  await touchTap(2);
  await initial();
  await touch(32,'touchStart');
  await page.waitForTimeout(45);
  await page.evaluate(() => window.dispatchEvent(new Event('blur')));
  await touch(32,'touchEnd');
  const blurred = await frame();
  await page.waitForTimeout(200);
  assert.equal(await frame(), blurred, 'blur releases touch');

  // A/B are unused by Beacon Trail. A checksum-valid test-only variant reads the
  // action row for movement, so the real keypad path is observable in canvas pixels.
  const actions = Buffer.from(await readFile(rom));
  const directionScan = Buffer.from([0xb0,0x20,0xe6,0xb5,0x90,0x90,0xe4,0xb5]);
  const offset = actions.indexOf(directionScan, 65536);
  assert.ok(offset >= 65536, 'find the cartridge direction scan');
  actions[offset+1] = 0x40; // action row: A=4 (down), B=8 (left)
  await page.locator('#rom').setInputFiles({ name:'action-input-test.ws', mimeType:'application/octet-stream', buffer:checksum(actions) });
  await initial();
  await touchTap(4);
  await expectTile(3,9,player);
  await touchTap(8);
  await expectTile(2,9,player);
  await tap('Enter');
  await initial();
  await tap('z');
  await expectTile(3,9,player);
  await tap('x');
  await expectTile(2,9,player);
  await page.locator('#rom').setInputFiles(rom);
  await initial();
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
