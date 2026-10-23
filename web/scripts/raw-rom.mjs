// SPDX-License-Identifier: CC0-1.0
// Original raw-image CPU fixtures through the production local file flow.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

export async function rawRomChecks(browser, base, errors) {
  const probes = await Promise.all(['raw3-common','raw3-2003','raw6-common','raw6-2003'].map(async name => {
    const bytes = await readFile(new URL(`../../fixtures/generated/${name}.ws`,import.meta.url));
    const digest = bytes => createHash('sha256').update(bytes).digest('hex');
    const capacity = name.startsWith('raw3') ? 4*1024*1024 : 8*1024*1024;
    // Explicitly distinguish the exact raw identity from synthetic leading fill.
    return {name,bytes,key:`wonderswan:save:v1:${digest(bytes)}`,
      paddedKey:`wonderswan:save:v1:${digest(Buffer.concat([Buffer.alloc(capacity-bytes.length,255),bytes]))}`};
  }));
  const context = await browser.newContext();
  await context.addInitScript(() => {
    // Distinct console records survive cartridge replacements and model changes.
    for (const [index,model] of ['mono','color','crystal'].entries()) {
      const key = `wonderswan:console:v1:${model}`;
      if (localStorage.getItem(key) !== null) continue;
      const data = new Uint8Array(model === 'mono' ? 128 : 2048).fill(255);
      data[0] = index+20;
      localStorage.setItem(key,btoa(String.fromCharCode(...data)));
    }
  });
  let page;
  const requests = [];
  async function open() {
    page = await context.newPage();
    page.on('pageerror', error => errors.push(error.message));
    await page.goto(base);await ready();
    page.on('request', request => requests.push(request.url()));
  }
  async function ready() { await page.waitForFunction(() => !document.querySelector('#model').disabled); }
  async function success(model) {
    await page.waitForFunction(model => {
      const p = document.querySelector('#screen').getContext('2d').getImageData(0,0,1,1).data;
      const expected = model === 'mono' ? [170,170,170] : [0,255,0];
      return document.querySelector('#rom-status').textContent === 'Running' && expected.every((v,i) => p[i] === v);
    },model);
    assert.equal(await page.locator('#error').textContent(),'');
  }
  async function load(probe, model, filename='same-name.ws') {
    const chooser = page.waitForEvent('filechooser');
    await page.locator('#load-rom').click();
    await (await chooser).setFiles({name:filename,mimeType:'application/octet-stream',buffer:probe.bytes});
    await success(model);
  }
  async function pause() {
    if (await page.locator('#pause').textContent() === 'Pause') await page.locator('#pause').click();
  }
  async function saved(probe, expected) {
    await page.waitForFunction(({key,expected}) => {
      const value = localStorage.getItem(key);
      return value !== null && atob(value).charCodeAt(0) === expected;
    },{key:probe.key,expected});
    assert.equal(await page.evaluate(key => atob(localStorage.getItem(key)).length,probe.key),32768);
    assert.equal(await page.evaluate(key => localStorage.getItem(key),probe.paddedKey),null);
  }
  try {
    await open();
    const originalConsole = await page.evaluate(() => ['mono','color','crystal'].map(model => localStorage.getItem(`wonderswan:console:v1:${model}`)));
    for (const probe of probes) {
      if (probe !== probes[0]) { await page.close();await open(); }
      await page.locator('#model').selectOption('mono');
      await load(probe,'mono');await pause();await saved(probe,1);
      let count = 1;
      for (const model of ['mono','color','crystal']) {
        if (model !== 'mono') {
          await page.locator('#model').selectOption(model);await success(model);await pause();await saved(probe,++count);
        }
        await page.locator('#reset').click();await success(model);await pause();await saved(probe,++count);
      }
      assert.equal(count,6);
    }
    assert.equal(requests.length,0,'local raw images/saves never make network requests');
    assert.equal(await page.evaluate(() => Object.keys(localStorage).filter(key => key.startsWith('wonderswan:save:')).length),4);
    assert.deepEqual(await page.evaluate(() => ['mono','color','crystal'].map(model => localStorage.getItem(`wonderswan:console:v1:${model}`))),originalConsole);
    await page.reload();await ready();await page.locator('#model').selectOption('crystal');
    await load(probes[0],'crystal','renamed.ws');await pause();await saved(probes[0],7);
    await load(probes[2],'crystal');await pause();await saved(probes[2],7);
    await load(probes[1],'crystal');await pause();await saved(probes[1],7);
    await saved(probes[3],6);
    await page.close();await open();
    await load(probes[3],'crystal','reopened.ws');await pause();await saved(probes[3],7);
    assert.deepEqual(await page.evaluate(() => ['mono','color','crystal'].map(model => localStorage.getItem(`wonderswan:console:v1:${model}`))),originalConsole);
    console.log('Raw 3/6 MiB common/2003 CPU probes: local file load, all models, DMA/bank assertions, restart, exact raw SHA-256 keys, reload/reopen/save isolation and separate console records passed');
  } finally { await context.close(); }
}
