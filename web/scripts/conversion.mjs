// SPDX-License-Identifier: CC0-1.0
// Independent literals; no ALU helpers or parsing cartridge expected values.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

function expected(model,boot) {
  const mono = model === 'mono';
  const words = [0x600d,boot,mono ? 0 : 2,mono ? 1 : 7];
  // AX / full FLAGS / ZF,CF,OF,SF,PF branch bits; opposing incoming status.
  const results = [
        0x0402, 0xf002, 0x0000, 0x0402, 0xf402, 0x0000,
        0x0010, 0xf012, 0x0000, 0x0010, 0xf412, 0x0000,
        0x0000, 0xf046, 0x0011, 0x0000, 0xf446, 0x0011,
        0xff00, 0xf046, 0x0011, 0xff00, 0xf446, 0x0011,
        0x0001, 0xf002, 0x0000, 0x0001, 0xf402, 0x0000,
        0x7f01, 0xf002, 0x0000, 0x7f01, 0xf402, 0x0000,
        0x0400, 0xf046, 0x0011, 0x0400, 0xf446, 0x0011,
        0x0009, 0xf006, 0x0010, 0x0009, 0xf406, 0x0010,
        0x0f0f, 0xf006, 0x0010, 0x0f0f, 0xf406, 0x0010,
        0x007f, 0xf002, 0x0000, 0x007f, 0xf402, 0x0000,
        0x0080, 0xf082, 0x0008, 0x0080, 0xf482, 0x0008,
        0x0081, 0xf086, 0x0018, 0x0081, 0xf486, 0x0018,
        0x00fe, 0xf082, 0x0008, 0x00fe, 0xf482, 0x0008,
        0x0100, 0xf046, 0x0011, 0x0100, 0xf446, 0x0011,
        0x0080, 0xf082, 0x0008, 0x0080, 0xf482, 0x0008,
        0x0000, 0xf046, 0x0011, 0x0000, 0xf446, 0x0011,
        0x0010, 0xf012, 0x0000, 0x0010, 0xf412, 0x0000,
        0x0080, 0xf892, 0x000c, 0x0080, 0xfc92, 0x000c,
        0x0000, 0xf057, 0x0013, 0x0000, 0xf457, 0x0013,
        0x0004, 0xf002, 0x0000, 0x0004, 0xf402, 0x0000,
        0x0003, 0xf017, 0x0012, 0x0003, 0xf417, 0x0012,
        0x00ff, 0xf086, 0x0018, 0x00ff, 0xf486, 0x0018,
        0x00f0, 0xf086, 0x0018, 0x00f0, 0xf486, 0x0018,
        0x0000, 0xf847, 0x0017, 0x0000, 0xfc47, 0x0017,
        0x0081, 0xf086, 0x0018, 0x0081, 0xf486, 0x0018,
        0x0000, 0xf057, 0x0013, 0x0000, 0xf457, 0x0013,
        0x007d, 0xf017, 0x0012, 0x007d, 0xf417, 0x0012,
  ];
  for (const mode of (mono ? [0] : [0,0x80,0,0xc0,0,0xe0,0])) words.push(mode,...results);
  while (words.length < 1145) words.push(0);
  return words;
}

export async function conversionChecks(browser, base, errors) {
  const bytes = await readFile(new URL('../../fixtures/generated/conversion.ws',import.meta.url));
  const key = `wonderswan:save:v1:${createHash('sha256').update(bytes).digest('hex')}`;
  for (const model of ['mono','color','crystal']) {
    const context = await browser.newContext();
    const page = await context.newPage();
    page.on('pageerror', error => errors.push(error.message));
    const requests = [];
    try {
      await page.goto(base);
      await page.waitForFunction(() => !document.querySelector('#model').disabled);
      await page.locator('#model').selectOption(model);
      // Poison retained observations, including an old pass marker. The ROM
      // must clear them and complete a fresh boot to satisfy the assertions.
      await page.evaluate(key => {
        const stale = new Uint8Array(32768).fill(0x6d);
        stale.set([0x0d,0x60,0,0]);
        localStorage.setItem(key,btoa(String.fromCharCode(...stale)));
      },key);
      page.on('request', request => requests.push(request.url()));
      const chooser = page.waitForEvent('filechooser');
      await page.locator('#load-rom').click();
      await (await chooser).setFiles({name:'conversion.ws',mimeType:'application/octet-stream',buffer:bytes});
      for (let boot=1;boot<=2;boot++) {
        if (boot === 2) await page.locator('#reset').click();
        // Fresh retained counter prevents old SRAM/pass pixels satisfying reset.
        await page.waitForFunction(({key,boot,model}) => {
          const saved = localStorage.getItem(key);
          if (!saved || document.querySelector('#rom-status').textContent !== 'Running') return false;
          const bytes = atob(saved);
          const count = bytes.charCodeAt(2) | bytes.charCodeAt(3) << 8;
          const pixel = document.querySelector('#screen').getContext('2d').getImageData(0,0,1,1).data;
          return count === boot && (model === 'mono' ? [170,170,170] : [0,255,0]).every((v,i) => pixel[i] === v);
        },{key,boot,model});
        assert.equal(await page.locator('#error').textContent(),'');
        await page.locator('#pause').click(); // real player flush, real SRAM
        const saved = await page.evaluate(key => Array.from(atob(localStorage.getItem(key)),c => c.charCodeAt(0)),key);
        assert.equal(saved.length,32768);
        for (const [i,value] of expected(model,boot).entries()) {
          assert.equal(saved[i*2] | saved[i*2+1] << 8,value,`${model} boot ${boot} word ${i}`);
        }
      }
      assert.equal(requests.length,0,'ROM and save data stay browser-local');
    } finally { await context.close(); }
  }
  console.log('Conversion CPU probe: 27 literal vectors under opposing status, full AX/FLAGS and five branches, all models, seven runtime mode phases and fresh restart passed');
}
