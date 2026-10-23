// SPDX-License-Identifier: CC0-1.0
// Independent literal rows from the source-included line-polling cartridge.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

const vectors = [
  [0,0,255,[0,255,0,255]], [4,0,255,[255,0,0,255]],
  [0,38,255,[0,255,0,255]], [4,38,255,[255,0,0,255]],
  [0,39,170,[255,0,255,255]], [4,39,0,[255,255,0,255]],
  [4,40,0,[255,255,0,255]], [16,40,85,[0,255,255,255]],
  [4,46,0,[255,255,0,255]], [4,47,85,[0,0,255,255]],
  [16,47,85,[0,255,255,255]], [20,48,0,[255,255,0,255]],
  [4,142,0,[255,255,0,255]], [0,143,153,[68,85,102,255]],
  [4,143,119,[170,85,51,255]],
];

const rows = [
  [0,[0,255,0,255],[255,0,0,255],255,255],
  [38,[0,255,0,255],[255,0,0,255],255,255],
  [39,[255,0,255,255],[255,255,0,255],170,0],
  [40,[255,0,255,255],[255,255,0,255],170,0],
  [46,[255,0,255,255],[255,255,0,255],170,0],
  [47,[255,0,255,255],[0,0,255,255],170,85],
  [48,[255,0,255,255],[255,255,0,255],170,0],
  [142,[255,0,255,255],[255,255,0,255],170,0],
  [143,[68,85,102,255],[170,85,51,255],153,119],
];

export async function lcdOutputChecks(browser, base, errors) {
  const bytes = await readFile(new URL('../../fixtures/generated/lcd-output.ws',import.meta.url));
  const key = `wonderswan:save:v1:${createHash('sha256').update(bytes).digest('hex')}`;
  for (const model of ['mono','color','crystal']) {
    const context = await browser.newContext();
    await context.addInitScript(() => localStorage.setItem('wonderswan:audio:v1','{"muted":true,"volume":70}'));
    const page = await context.newPage();
    page.on('pageerror', error => errors.push(error.message));
    const requests = [];
    try {
      await page.goto(base);
      await page.waitForFunction(() => !document.querySelector('#model').disabled);
      await page.locator('#model').selectOption(model);
      page.on('request', request => requests.push(request.url()));
      const chooser = page.waitForEvent('filechooser');
      await page.locator('#load-rom').click();
      await (await chooser).setFiles({name:'lcd-output.ws',mimeType:'application/octet-stream',buffer:bytes});
      for (let boot=1;boot<=2;boot++) {
        if (boot === 2) await page.locator('#reset').click();
        // Fresh boot + completed raster passes prevent stale SRAM/frame pixels
        // from satisfying restart. No test-only core or browser API is needed.
        await page.waitForFunction(({key,boot}) => {
          const saved = localStorage.getItem(key);
          if (!saved || document.querySelector('#rom-status').textContent !== 'Running') return false;
          const data = atob(saved);
          const word = i => data.charCodeAt(i) | data.charCodeAt(i+1) << 8;
          return word(0) === boot && word(2) >= 3 && word(8) === 0x600d;
        },{key,boot});
        assert.equal(await page.locator('#error').textContent(),'');
        await page.locator('#pause').click();
        const saved = await page.evaluate(key => Array.from(atob(localStorage.getItem(key)),c => c.charCodeAt(0)),key);
        assert.equal(saved.length,32768);
        for (const [offset,value] of [[0,boot],[4,40],[6,144],[8,0x600d]]) {
          assert.equal(saved[offset] | saved[offset+1] << 8,value,`${model} boot ${boot} CPU word ${offset}`);
        }
        const actual = await page.locator('#screen').evaluate((canvas,vectors) => {
          const ctx = canvas.getContext('2d');
          return vectors.map(([x,y]) => Array.from(ctx.getImageData(x,y,1,1).data));
        },vectors);
        for (const [i,[x,y,mono,color]] of vectors.entries()) {
          assert.deepEqual(actual[i],model === 'mono' ? [mono,mono,mono,255] : color,
            `${model} boot ${boot} (${x},${y})`);
        }
        const pixels = await page.locator('#screen').evaluate((canvas,rows) => {
          const ctx = canvas.getContext('2d');
          return rows.map(([y]) => Array.from(ctx.getImageData(0,y,224,1).data));
        },rows);
        for (const [i,[y,fg,bg,monoFg,monoBg]] of rows.entries()) {
          const expected = [];
          for (let x=0;x<224;x++) {
            const sprite = y >= 40 && y < 48 && x >= 16 && x < 24;
            const v = sprite ? 85 : x%8 < 4 ? monoFg : monoBg;
            expected.push(...(model === 'mono' ? [v,v,v,255] : sprite ? [0,255,255,255] : x%8 < 4 ? fg : bg));
          }
          assert.deepEqual(pixels[i],expected,`${model} boot ${boot} entire row ${y}`);
        }
      }
      assert.equal(requests.length,0,'ROM and saves stay browser-local');
    } finally { await context.close(); }
  }
  console.log('LCD output probe: literal adjacent rows 38/39/40 and 142/143, retained tile identities, transparency/sprites, line40/144 palette writes and fresh restart CPU markers passed on all three production Wasm models');
}
