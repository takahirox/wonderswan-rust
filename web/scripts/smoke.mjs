// Production-preview browser acceptance. Run after building and starting Vite preview.
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { fileURLToPath } from 'node:url';

const base = process.env.WS_PREVIEW_URL ?? 'http://127.0.0.1:4173/wonderswan-rust/';
const rom = fileURLToPath(new URL('../public/demo/beacon-trail.ws', import.meta.url));
const browser = await chromium.launch({ headless: true });
const page = await browser.newPage(); // fresh, unauthenticated context
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
try {
  await page.goto(base);
  await page.waitForFunction(() => !document.querySelector('#controls').disabled);
  assert.equal(await page.locator('#model').inputValue(), 'crystal');
  assert.equal(await page.locator('#model option').count(), 3);
  assert.equal(await page.locator('#screen').getAttribute('width'), '224');
  assert.equal(await page.locator('#screen').getAttribute('height'), '144');
  await page.locator('#demo').click();
  await initial();
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
  const paused = await page.locator('#screen').evaluate(c => c.toDataURL());
  await tap('ArrowRight');
  assert.equal(await page.locator('#screen').evaluate(c => c.toDataURL()), paused);
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
  assert.match(await page.locator('#rom-status').textContent(), /beacon-trail.ws/);
  await page.locator('#rom').setInputFiles({name:'empty.ws',mimeType:'application/octet-stream',buffer:Buffer.alloc(0)});
  assert.match(await page.locator('#error').textContent(), /Expected a 128 KiB/);
  await page.locator('#rom').setInputFiles(rom);
  await initial();
  assert.equal(await page.locator('#error').textContent(), '');
  console.log('Local cartridge playthrough, no uploads, invalid-file recovery, same-file reload passed');
  assert.ok(requests.some(r => r.url.endsWith('.wasm')));
  assert.ok(requests.some(r => r.url.endsWith('/wonderswan-rust/demo/beacon-trail.ws')));
  assert.ok(requests.every(r => r.method === 'GET'));
  assert.deepEqual(errors, []);
  const notice = await page.request.get(new URL('demo/NOTICE.txt', base).href);
  assert.equal(notice.status(),200);
  assert.match(await notice.text(), /CC0-1.0/);
  console.log('Production base path, Wasm/ROM loading, license notice, and no browser errors passed');
} finally {
  await browser.close();
}
