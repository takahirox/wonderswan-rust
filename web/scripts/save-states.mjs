import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { chooseRemembered, openCartridges } from './rom-library.mjs';
import { installPlayerClock, stepPlayerClock } from './player-clock.mjs';

const databaseName = 'wonderswan-rust:save-states';
const fixture = async name => ({
  name: `${name}.ws`,
  mimeType: 'application/octet-stream',
  buffer: await readFile(new URL(`../../fixtures/generated/${name}.ws`, import.meta.url)),
});
const saveSlot = file => `wonderswan:save:v1:${createHash('sha256').update(file.buffer).digest('hex')}`;
const status = (page, selector, text) => page.waitForFunction(
  ({ selector, text }) => document.querySelector(selector).textContent.includes(text),
  { selector, text }, { polling: 30 },
);
const stateIdle = page => page.locator('#state-picker[aria-busy="false"]').waitFor({ state: 'attached' });
const choices = page => page.locator('#state-list .state-choice');

async function localRom(page, file) {
  await openCartridges(page);
  const chooser = page.waitForEvent('filechooser');
  await page.locator('#open-local-rom').click();
  await (await chooser).setFiles(file);
  await status(page, '#rom-status', 'Running');
  await page.evaluate(stepPlayerClock, 0);
  await page.evaluate(stepPlayerClock, 70);
}

async function picker(page, touch = false) {
  await page.locator('#select-state')[touch ? 'tap' : 'click']();
  await stateIdle(page);
  await page.locator('#state-picker').waitFor({ state: 'visible' });
}

async function clickSave(page) {
  await page.locator('#save-state').click();
  await status(page, '#state-status', 'Save state saved.');
  await stateIdle(page);
}

async function importState(page, bytes) {
  const chooser = page.waitForEvent('filechooser');
  await page.locator('#state-import').click();
  await (await chooser).setFiles({ name: 'export.wsstate', mimeType: 'application/octet-stream', buffer: bytes });
  await stateIdle(page);
}

async function history(page, operation, scope) {
  return page.evaluate(async ({ name, operation, scope }) => {
    const db = await new Promise((resolve, reject) => {
      const request = indexedDB.open(name, 1);
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
    try {
      const tx = db.transaction(['metadata', 'snapshots'], operation === 'list' ? 'readonly' : 'readwrite');
      const metadata = tx.objectStore('metadata');
      const bodies = tx.objectStore('snapshots');
      const all = metadata.getAll();
      if (operation === 'seed') {
        all.onsuccess = () => {
          const base = all.result.find(entry => entry.scope === scope);
          // Populate the real index with a second page. Browsing needs no bodies.
          for (let i = 1; i <= 21; i++) {
            metadata.put({ ...base, id: `test-${i}`, timestamp: base.timestamp - i });
          }
        };
      }
      if (operation === 'corrupt') {
        all.onsuccess = () => {
          const entry = all.result.find(entry => entry.scope === scope && !entry.label);
          bodies.put(new Uint8Array([1, 2, 3]), [scope, entry.id]);
        };
      }
      await new Promise((resolve, reject) => {
        tx.oncomplete = resolve;
        tx.onabort = () => reject(tx.error);
      });
      return all.result.filter(entry => entry.scope === scope)
        .map(({ id, timestamp, label, thumbnail }) => ({ id, timestamp, label, size: thumbnail.size }));
    } finally { db.close(); }
  }, { name: databaseName, operation, scope });
}

async function checkThumbnail(page, vertical) {
  const result = await page.evaluate(async vertical => {
    const image = document.querySelector('#state-list img');
    await image.decode();
    const frame = document.querySelector('#screen');
    const screenshot = document.createElement('canvas');
    screenshot.width = image.naturalWidth;
    screenshot.height = image.naturalHeight;
    const ctx = screenshot.getContext('2d');
    ctx.drawImage(image, 0, 0);
    const expected = frame.getContext('2d').getImageData(0, 0, 224, 144).data;
    const actual = ctx.getImageData(0, 0, screenshot.width, screenshot.height).data;
    return {
      width: screenshot.width, height: screenshot.height,
      varying: expected.some(byte => byte < 255),
      equal: expected.every((byte, i) => {
        if (!vertical) return byte === actual[i];
        const pixel = i >> 2;
        const rotated = ((223 - pixel % 224) * 144 + Math.floor(pixel / 224)) * 4 + (i & 3);
        return byte === actual[rotated];
      }),
    };
  }, vertical);
  assert.deepEqual(result, vertical
    ? { width: 144, height: 224, varying: true, equal: true }
    : { width: 224, height: 144, varying: true, equal: true });
}

function observeStorage() {
  window.stateNow = 10_000;
  Date.now = () => window.stateNow;
  const interval = window.setInterval;
  window.setInterval = (callback, delay, ...args) => {
    if (delay === 1000) { window.stateAutosave = callback; return 0; }
    return interval(callback, delay, ...args);
  };
  const write = Storage.prototype.setItem;
  Storage.prototype.setItem = function (key, value) {
    if (key === window.failNativeKey) {
      window.failNativeKey = null;
      throw new DOMException('Injected native write failure', 'QuotaExceededError');
    }
    write.call(this, key, value);
  };
  window.stateReads = 0;
  const read = IDBObjectStore.prototype.get;
  IDBObjectStore.prototype.get = function (...args) {
    if (this.name === 'snapshots') window.stateReads++;
    return read.apply(this, args);
  };
  const put = IDBObjectStore.prototype.put;
  IDBObjectStore.prototype.put = function (...args) {
    const request = put.apply(this, args);
    if (this.name === 'snapshots' && window.failStateWrite) {
      window.failStateWrite = false;
      request.addEventListener('success', () => this.transaction.abort(), { once: true });
    }
    return request;
  };
}

async function withPlayer(browser, base, errors, run, options) {
  const context = await browser.newContext(options);
  await context.addInitScript(installPlayerClock);
  await context.addInitScript(observeStorage);
  const page = await context.newPage();
  page.on('pageerror', error => errors.push(error.message));
  try {
    await page.goto(base);
    await page.locator('#load-rom:not(:disabled)').waitFor();
    await run(page);
  } finally { await context.close(); }
}

async function pickerChecks(page, horizontal, vertical) {
  const scope = `${saveSlot(horizontal)}:crystal`;
  const frame = () => page.locator('#screen').evaluate(canvas => canvas.toDataURL());
  assert.equal(await page.locator('#load-state').isDisabled(), true);
  await localRom(page, horizontal);
  assert.equal(await page.locator('#state-selection').textContent(), 'New state');
  const firstFrame = await frame();
  await clickSave(page);
  const initial = await history(page, 'list', scope);
  assert.equal(initial.length, 1);
  await page.locator('#select-state').focus();
  await page.keyboard.press('Enter');
  await stateIdle(page);
  assert.equal(await choices(page).getAttribute('aria-pressed'), 'true');
  await checkThumbnail(page, false);
  await page.keyboard.press('Escape');
  assert.equal(await page.locator('#select-state').evaluate(button => document.activeElement === button), true);
  await page.locator('#screen').focus();
  await page.keyboard.down('ArrowRight');
  await page.evaluate(stepPlayerClock, 120);
  await page.keyboard.up('ArrowRight');
  const changedFrame = await frame();
  assert.notEqual(changedFrame, firstFrame);
  await page.locator('#pause').click();
  await status(page, '#rom-status', 'Paused');

  // Database failure preserves ID, timestamp, and screenshot of the selected target.
  await page.evaluate(() => { window.stateNow += 1000; window.failStateWrite = true; });
  await page.locator('#save-state').click();
  await status(page, '#state-status', 'aborted');
  assert.deepEqual(await history(page, 'list', scope), initial);
  await page.locator('#load-state').click();
  await status(page, '#state-status', 'Previous progress is in the picker');
  assert.equal(await frame(), firstFrame);
  // The keypad probe also mirrors its input to SRAM. Remove this one
  // recovery entry before testing the regular state count and import flow.
  await picker(page);
  page.once('dialog', dialog => dialog.accept());
  await page.getByRole('button', { name: /^Delete Recovery/ }).click();
  await status(page, '#state-picker-status', 'Save state deleted.');
  await page.locator('#state-close').click();
  await page.locator('#pause').click();
  await page.evaluate(stepPlayerClock, 0);
  await page.evaluate(stepPlayerClock, 40);
  await page.locator('#pause').click();
  await clickSave(page);
  const overwrittenFrame = await frame();
  const overwritten = await history(page, 'list', scope);
  assert.equal(overwritten.length, 1);
  assert.equal(overwritten[0].id, initial[0].id);
  assert.ok(overwritten[0].timestamp > initial[0].timestamp);

  await picker(page);
  const downloading = page.waitForEvent('download');
  await page.getByRole('button', { name: /^Export / }).click();
  const exported = await readFile(await (await downloading).path());
  assert.equal(exported.subarray(0, 8).toString(), 'WSRSTATE');
  await importState(page, exported);
  await status(page, '#state-picker-status', 'Save state imported.');
  assert.equal(await choices(page).count(), 2);
  assert.equal(await page.locator('#state-list [aria-pressed="true"]').count(), 1);
  const selected = await page.locator('#state-selection').textContent();
  await importState(page, Buffer.from('bad snapshot'));
  await status(page, '#state-picker-status', 'Invalid save-state file');
  assert.equal(await page.locator('#state-selection').textContent(), selected);
  assert.equal(await choices(page).count(), 2);
  await page.locator('#state-new').click();
  assert.equal(await page.locator('#load-state').isDisabled(), true);
  await picker(page);
  const imported = choices(page).filter({ hasText: 'Imported state' });
  await imported.focus();
  await page.keyboard.press('Enter');
  await status(page, '#rom-status', 'Paused');
  assert.equal(await frame(), overwrittenFrame);

  // The selected destination and scope survive a console switch during the session.
  await page.locator('#model').selectOption('color');
  assert.equal(await page.locator('#state-selection').textContent(), 'New state');
  await picker(page);
  assert.equal(await choices(page).count(), 0);
  await page.locator('#state-close').click();
  await page.locator('#model').selectOption('crystal');
  assert.match(await page.locator('#state-selection').textContent(), /Imported state/);
  const resetFrame = await frame();
  assert.notEqual(resetFrame, overwrittenFrame);
  await picker(page);
  await choices(page).filter({ hasText: 'Imported state' }).click();
  assert.equal(await frame(), resetFrame, 'selection alone leaves the reset frame intact');
  await page.locator('#load-state').click();
  await status(page, '#state-status', 'Save state loaded.');
  await status(page, '#rom-status', 'Running');
  await page.evaluate(() => {
    Object.defineProperty(document, 'hidden', { configurable: true, value: true });
    document.dispatchEvent(new Event('visibilitychange'));
  });
  await status(page, '#rom-status', 'Paused');
  await page.evaluate(() => {
    delete document.hidden;
    document.dispatchEvent(new Event('visibilitychange'));
  });
  await status(page, '#rom-status', 'Running');
  await page.evaluate(stepPlayerClock, 0);
  await page.evaluate(stepPlayerClock, 40);
  await page.locator('#pause').click();
  await status(page, '#rom-status', 'Paused');
  await page.locator('#load-state').click();
  await status(page, '#state-status', 'Save state loaded.');
  assert.equal(await page.locator('#pause').textContent(), 'Resume');
  assert.equal(await frame(), overwrittenFrame);

  await picker(page);
  page.once('dialog', dialog => dialog.dismiss());
  await page.getByRole('button', { name: /^Delete Imported state/ }).click();
  await stateIdle(page);
  assert.equal(await choices(page).count(), 2);
  page.once('dialog', dialog => dialog.accept());
  await page.getByRole('button', { name: /^Delete Imported state/ }).click();
  await status(page, '#state-picker-status', 'Save state deleted.');
  assert.equal(await choices(page).count(), 1);
  await page.locator('#state-close').click();
  assert.equal(await page.locator('#load-state').isDisabled(), true);

  await localRom(page, vertical);
  await picker(page);
  assert.equal(await choices(page).count(), 0);
  await page.locator('#state-close').click();
  await clickSave(page);
  await picker(page);
  await checkThumbnail(page, true);
  await page.locator('#state-close').click();
  await page.reload();
  await chooseRemembered(page, horizontal.name);
  await status(page, '#rom-status', 'Running');
  await page.locator('#pause').click();
  await picker(page);
  assert.equal(await choices(page).count(), 1);
  await choices(page).click();
  await history(page, 'corrupt', scope);
  await page.locator('#load-state').click();
  await status(page, '#state-status', 'Invalid save-state file');
  assert.equal(await page.locator('#pause').textContent(), 'Resume');

  await history(page, 'seed', scope);
  const before = await page.evaluate(() => window.stateReads);
  await picker(page);
  assert.equal(await choices(page).count(), 20);
  await page.locator('#state-older').click();
  await stateIdle(page);
  assert.equal(await choices(page).count(), 2);
  await page.locator('#state-newer').click();
  await stateIdle(page);
  assert.equal(await choices(page).count(), 20);
  assert.equal(await page.evaluate(() => window.stateReads), before);
}

async function recoveryChecks(page, file) {
  const romKey = saveSlot(file);
  const scope = `${romKey}:crystal`;
  const rtcKey = romKey.replace(':save:', ':rtc:');
  const consoleKey = 'wonderswan:console:v1:crystal';
  const keys = [romKey, rtcKey, consoleKey];
  const read = () => page.evaluate(keys => keys.map(key => localStorage.getItem(key)), keys);
  const write = values => page.evaluate(
    ({ keys, values }) => keys.forEach((key, i) => localStorage.setItem(key, values[i])),
    { keys, values },
  );
  function native(save, second, console) {
    const clock = Buffer.from([82, 84, 67, 1, 0, 1, 1, 0, 0, 0, second, 0x82, 0x80, 0, 0, 0, 1,
      0x10, 0x27, 0, 0, 0, 0, 0, 0]);
    const data = Buffer.alloc(2048);
    data.fill(255, 0, 0x60);
    data[0x61] = console;
    return [
      Buffer.alloc(131072, save).toString('base64'),
      JSON.stringify({ version: 1, data: clock.toString('base64') }),
      data.toString('base64'),
    ];
  }
  await write(native(0x10, 7, 0x19));
  await page.reload(); // Load existing v1 console data during initialization.
  await localRom(page, file);
  await page.locator('#pause').click();
  const oldProgress = await read();
  await clickSave(page);
  await write(native(0x40, 0x40, 0x45));
  await page.reload();
  await chooseRemembered(page, file.name);
  await status(page, '#rom-status', 'Running');
  await page.evaluate(stepPlayerClock, 0);
  await page.evaluate(stepPlayerClock, 70);
  await page.locator('#pause').click();
  const live = await read();
  assert.equal(Buffer.from(live[0], 'base64')[0], 0x41);
  assert.equal(Buffer.from(live[2], 'base64')[0x61], 0x45);
  const stored = native(0x66, 0x59, 0x67);
  await write(stored);
  const before = await page.locator('#screen').evaluate(canvas => canvas.toDataURL());
  await picker(page);
  await choices(page).click();

  await page.evaluate(() => { window.failStateWrite = true; });
  await page.locator('#load-state').click();
  await status(page, '#state-status', 'Could not save recovery progress');
  await page.evaluate(() => window.stateAutosave());
  assert.deepEqual(await read(), stored);
  assert.equal(await page.locator('#screen').evaluate(canvas => canvas.toDataURL()), before);
  assert.equal((await history(page, 'list', scope)).length, 1);

  await page.evaluate(key => { window.failNativeKey = key; }, consoleKey);
  await page.locator('#load-state').click();
  await status(page, '#state-status', 'Injected native write failure');
  await page.evaluate(() => window.stateAutosave());
  assert.deepEqual(await read(), stored);
  assert.equal((await history(page, 'list', scope)).length, 3);
  assert.equal(await page.locator('#screen').evaluate(canvas => canvas.toDataURL()), before);

  await page.locator('#load-state').click();
  await status(page, '#state-status', 'Previous progress is in the picker');
  assert.deepEqual(await read(), oldProgress);
  assert.equal(await page.locator('#pause').textContent(), 'Resume');
  await picker(page);
  await choices(page).filter({ hasText: 'Recovery · live progress' }).first().click();
  await page.locator('#load-state').click();
  await status(page, '#state-status', 'Previous progress is in the picker');
  assert.deepEqual(await read(), live);
  await picker(page);
  await choices(page).filter({ hasText: 'Recovery · stored progress' }).first().click();
  await page.locator('#load-state').click();
  await status(page, '#state-status', 'Previous progress is in the picker');
  assert.deepEqual(await read(), stored);

  const retained = await history(page, 'list', scope);
  const corruptClock = [stored[0], 'unreadable native clock', stored[2]];
  await write(corruptClock);
  const recoveredFrame = await page.locator('#screen').evaluate(canvas => canvas.toDataURL());
  await page.locator('#load-state').click();
  await status(page, '#state-status', 'local clock record is unreadable');
  await page.evaluate(() => window.stateAutosave());
  assert.deepEqual(await read(), corruptClock);
  assert.equal(await page.locator('#screen').evaluate(canvas => canvas.toDataURL()), recoveredFrame);
  await write(stored);

  const cartridges = await openCartridges(page);
  page.once('dialog', dialog => dialog.accept());
  await cartridges.getByRole('button', { name: `Remove ${file.name}` }).click();
  await cartridges.locator('#cartridge-list li').waitFor({ state: 'detached' });
  assert.deepEqual(await read(), stored);
  assert.deepEqual(await history(page, 'list', scope), retained);
}

async function consoleRecovery(page, file) {
  const key = 'wonderswan:console:v1:crystal';
  const count = () => page.evaluate(key => atob(localStorage.getItem(key)).charCodeAt(2), key);
  await localRom(page, file);
  await page.locator('#pause').click();
  assert.equal(await count(), 1);
  await clickSave(page);
  await page.locator('#reset').click();
  await page.evaluate(stepPlayerClock, 0);
  await page.evaluate(stepPlayerClock, 70);
  await page.locator('#pause').click();
  assert.equal(await count(), 2);
  await page.locator('#load-state').click();
  await status(page, '#state-status', 'Previous progress is in the picker');
  assert.equal(await count(), 1);
  await picker(page);
  await choices(page).filter({ hasText: 'Recovery · live progress' }).click();
  await page.locator('#load-state').click();
  await status(page, '#state-status', 'Previous progress is in the picker');
  assert.equal(await count(), 2);
}

async function touchChecks(page, horizontal, vertical) {
  for (const [width, height] of [[375, 812], [812, 375], [320, 568]]) {
    await page.setViewportSize({ width, height });
    for (const rom of [horizontal, vertical]) {
      await localRom(page, rom);
      await picker(page, true);
      const close = page.locator('#state-close');
      const box = await close.boundingBox();
      assert.ok(box.y >= 0 && box.y + box.height <= height);
      await page.touchscreen.tap(2, 2); // Outside the dialog and away from gameplay controls.
      await page.locator('#state-picker').waitFor({ state: 'hidden' });
      assert.equal(await page.locator('#load-state').isDisabled(), true);
      const controls = await page.locator('[data-button]').all();
      assert.equal(controls.length, 11);
      for (const control of controls) {
        await control.scrollIntoViewIfNeeded();
        const hit = await control.evaluate(button => {
          const rect = button.getBoundingClientRect();
          return rect.width >= 44 && rect.height >= 44
            && document.elementFromPoint(rect.x + rect.width / 2, rect.y + rect.height / 2) === button;
        });
        assert.ok(hit, `unreachable control at ${width}×${height} with ${rom.name}`);
      }
    }
  }
}

export async function saveStateChecks(browser, base, errors) {
  const [horizontal, vertical, rtc, internalEeprom] = await Promise.all([
    fixture('keypad-horizontal'), fixture('keypad-vertical'), fixture('sram128-2003'), fixture('internal-eeprom'),
  ]);
  await withPlayer(browser, base, errors, page => pickerChecks(page, horizontal, vertical));
  await withPlayer(browser, base, errors, page => recoveryChecks(page, rtc));
  await withPlayer(browser, base, errors, page => consoleRecovery(page, internalEeprom));
  await withPlayer(browser, base, errors, page => touchChecks(page, horizontal, vertical),
    { isMobile: true, hasTouch: true, viewport: { width: 375, height: 812 } });
  console.log('Save states: selector, explicit Save/Load, atomic overwrite, rotated screenshots, import/export, paging, identity, native backups and rollback, keyboard and touch passed');
}
