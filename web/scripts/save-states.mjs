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

async function clickLoad(page) {
  await page.locator('#load-state').click();
  await status(page, '#state-status', 'Save state loaded.');
  await stateIdle(page);
}

async function fullscreenChecks(page, rom) {
  const scope = `${saveSlot(rom)}:crystal`;
  const frame = () => page.locator('#screen').evaluate(canvas => canvas.toDataURL());
  const selected = () => page.locator('#state-selection').textContent();
  const active = () => page.evaluate(() => document.fullscreenElement?.id);
  const focus = () => page.evaluate(() => document.activeElement.id);
  async function play(key, time) {
    await page.locator('#screen').focus();
    await page.keyboard.down(key);
    await page.evaluate(stepPlayerClock, time);
    await page.keyboard.up(key);
  }

  await localRom(page, rom);
  const firstFrame = await frame();
  await clickSave(page);
  const firstSelection = await selected();
  const firstHistory = await history(page, 'list', scope);
  await page.locator('#game').evaluate(game => {
    const request = game.requestFullscreen;
    game.requestFullscreen = function () {
      game.requestFullscreen = request;
      const entering = request.call(this);
      document.querySelector('#select-state').click();
      return entering;
    };
  });
  await page.locator('#fullscreen').click();
  await page.waitForFunction(() => document.fullscreenElement?.id === 'game'
    && document.querySelector('.fullscreen-management #select-state'), undefined, { polling: 30 });
  await stateIdle(page);
  assert.equal(await focus(), 'state-close', 'the picker keeps focus during fullscreen entry');
  assert.ok(await page.locator('#state-close').evaluate(button => {
    const box = button.getBoundingClientRect();
    return document.elementFromPoint(box.x + box.width / 2, box.y + box.height / 2) === button;
  }), 'the dialog stays above gameplay during fullscreen entry');
  assert.equal(await selected(), firstSelection);
  await page.keyboard.press('Tab');
  assert.equal(await focus(), 'state-new');
  await page.keyboard.press('ArrowRight');
  assert.equal(await page.locator('.is-pressed').count(), 0);
  await page.keyboard.press('Escape');
  await page.locator('#state-picker').waitFor({ state: 'hidden' });
  assert.equal(await active(), 'game');
  assert.equal(await focus(), 'select-state');

  await picker(page);
  await page.locator('#state-new').click();
  assert.equal(await selected(), 'New state');
  assert.equal(await page.locator('#load-state').isDisabled(), true);
  assert.equal(await frame(), firstFrame);
  assert.deepEqual(await history(page, 'list', scope), firstHistory);
  await play('ArrowRight', 120);
  const secondFrame = await frame();
  assert.notEqual(secondFrame, firstFrame);
  await page.evaluate(() => { window.stateNow += 1000; });
  await clickSave(page);
  assert.equal(await focus(), 'save-state');
  const afterSave = await history(page, 'list', scope);
  assert.equal(afterSave.length, 2);
  await play('ArrowUp', 170);
  const overwrittenFrame = await frame();
  assert.notEqual(overwrittenFrame, secondFrame);
  await page.evaluate(() => { window.stateNow += 1000; });
  await clickSave(page);
  const afterOverwrite = await history(page, 'list', scope);
  assert.deepEqual(afterOverwrite.map(entry => entry.id), afterSave.map(entry => entry.id));
  const overwrittenSelection = await selected();

  await picker(page);
  await page.getByRole('button', { name: firstSelection, exact: true }).click();
  assert.equal(await frame(), overwrittenFrame, 'selection leaves the guest frame intact');
  await clickLoad(page);
  assert.equal(await frame(), firstFrame);
  assert.equal(await focus(), 'screen');
  assert.equal(await active(), 'game');
  await status(page, '#rom-status', 'Running');
  assert.deepEqual(await history(page, 'list', scope), afterOverwrite, 'Load creates no backup');

  await picker(page);
  const downloading = page.waitForEvent('download');
  await page.getByRole('button', { name: `Export ${overwrittenSelection}` }).click();
  const exported = await readFile(await (await downloading).path());
  await importState(page, exported);
  await status(page, '#state-picker-status', 'Save state imported.');
  const remove = page.getByRole('button', { name: /^Delete Imported state/ });
  await remove.click();
  const cancel = page.getByRole('button', { name: 'Cancel', exact: true });
  assert.equal(await cancel.evaluate(button => document.activeElement === button), true);
  await cancel.click();
  assert.equal(await choices(page).count(), 3);
  await remove.click();
  await page.getByRole('button', { name: 'Confirm delete' }).click();
  await status(page, '#state-picker-status', 'Save state deleted.');
  assert.deepEqual(await history(page, 'list', scope), afterOverwrite);
  assert.equal(await active(), 'game');

  const chooser = page.waitForEvent('filechooser');
  await page.locator('#state-import').click();
  await (await chooser).setFiles([]);
  assert.equal(await focus(), 'state-import');
  const pending = page.waitForEvent('filechooser');
  await page.locator('#state-import').click();
  const file = await pending;
  await page.evaluate(() => document.exitFullscreen());
  await page.waitForFunction(() => !document.fullscreenElement && !document.querySelector('.brand').inert,
    undefined, { polling: 30 });
  assert.equal(await focus(), 'state-import');
  await file.setFiles({ name: 'export.wsstate', mimeType: 'application/octet-stream', buffer: exported });
  await status(page, '#state-picker-status', 'Save state imported.');
  assert.equal(await selected(), firstSelection);
  assert.equal(await choices(page).count(), 3);
  await page.locator('#state-close').click();
  assert.equal(await focus(), 'select-state');
  await picker(page);
  await page.getByRole('button', { name: overwrittenSelection, exact: true }).click();
  await clickLoad(page);
  assert.equal(await frame(), overwrittenFrame, 'windowed Load restores the fullscreen overwrite');
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
  window.stateWrites = 0;
  const read = IDBObjectStore.prototype.get;
  IDBObjectStore.prototype.get = function (...args) {
    if (this.name === 'snapshots') window.stateReads++;
    return read.apply(this, args);
  };
  const put = IDBObjectStore.prototype.put;
  IDBObjectStore.prototype.put = function (...args) {
    const request = put.apply(this, args);
    if (this.name === 'snapshots') {
      window.stateWrites++;
      if (window.failStateWrite) {
        window.failStateWrite = false;
        request.addEventListener('success', () => this.transaction.abort(), { once: true });
      }
    }
    return request;
  };
}

function observeLiveRtc() {
  // Read the actual player clock without sampling wall time or triggering autosave.
  const wrap = result => {
    const exports = { ...result.instance.exports };
    const read = exports.webmachine_rtc_data;
    exports.webmachine_rtc_data = machine => {
      window.readLiveRtc = () => {
        const [ptr, length] = read(machine);
        const bytes = Array.from(new Uint8Array(exports.memory.buffer, ptr, length));
        exports.__wbindgen_free(ptr, length, 1);
        return bytes;
      };
      return read(machine);
    };
    return { ...result, instance: { exports } };
  };
  const streaming = WebAssembly.instantiateStreaming.bind(WebAssembly);
  WebAssembly.instantiateStreaming = async (...args) => wrap(await streaming(...args));
  const instantiate = WebAssembly.instantiate.bind(WebAssembly);
  WebAssembly.instantiate = async (...args) => {
    const result = await instantiate(...args);
    return result.instance ? wrap(result) : result;
  };
}

async function withPlayer(browser, base, errors, run, options) {
  const context = await browser.newContext(options);
  await context.addInitScript(installPlayerClock);
  await context.addInitScript(observeStorage);
  await context.addInitScript(observeLiveRtc);
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
  await clickLoad(page);
  assert.equal(await frame(), firstFrame);
  assert.deepEqual(await history(page, 'list', scope), initial);
  await page.locator('#pause').click();
  await page.keyboard.down('ArrowUp');
  await page.evaluate(stepPlayerClock, 0);
  await page.evaluate(stepPlayerClock, 70);
  await page.keyboard.up('ArrowUp');
  await page.locator('#pause').click();
  await clickSave(page);
  const overwrittenFrame = await frame();
  assert.notEqual(overwrittenFrame, firstFrame);
  const overwritten = await history(page, 'list', scope);
  assert.equal(overwritten.length, 1);
  assert.equal(overwritten[0].id, initial[0].id);
  assert.ok(overwritten[0].timestamp > initial[0].timestamp);

  await picker(page);
  await checkThumbnail(page, false);
  const downloading = page.waitForEvent('download');
  await page.getByRole('button', { name: /^Export / }).click();
  const exported = await readFile(await (await downloading).path());
  assert.equal(exported.subarray(0, 8).toString(), 'WSRSTATE');
  await importState(page, exported);
  await status(page, '#state-picker-status', 'Save state imported.');
  assert.equal(await choices(page).count(), 2);
  const importedHistory = await history(page, 'list', scope);
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
  await clickLoad(page);
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
  await clickLoad(page);
  assert.equal(await page.locator('#pause').textContent(), 'Resume');
  assert.equal(await frame(), overwrittenFrame);
  assert.deepEqual(await history(page, 'list', scope), importedHistory);

  await picker(page);
  await page.getByRole('button', { name: /^Delete Imported state/ }).click();
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  assert.equal(await choices(page).count(), 2);
  await page.getByRole('button', { name: /^Delete Imported state/ }).click();
  await page.getByRole('button', { name: 'Confirm delete' }).click();
  await status(page, '#state-picker-status', 'Save state deleted.');
  assert.equal(await choices(page).count(), 1);
  await page.locator('#state-close').click();
  assert.equal(await page.locator('#load-state').isDisabled(), true);

  await localRom(page, vertical);
  assert.deepEqual(await history(page, 'list', scope), overwritten);
  await picker(page);
  assert.equal(await choices(page).count(), 0);
  await page.locator('#state-new').click();
  await clickSave(page);
  assert.equal((await history(page, 'list', `${saveSlot(vertical)}:crystal`)).length, 1);
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
  const beforeInvalidLoad = await frame();
  await page.locator('#load-state').click();
  await status(page, '#state-status', 'Invalid save-state file');
  assert.equal(await page.locator('#pause').textContent(), 'Resume');
  assert.equal(await frame(), beforeInvalidLoad);
  assert.deepEqual(await history(page, 'list', scope), overwritten);

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

async function rtcLoadChecks(page, file) {
  const romKey = saveSlot(file);
  const scope = `${romKey}:crystal`;
  const rtcKey = romKey.replace(':save:', ':rtc:');
  const storedSave = () => page.evaluate(key => localStorage.getItem(key), romKey);
  const storedClock = () => page.evaluate(key => {
    const data = atob(JSON.parse(localStorage.getItem(key)).data);
    return Array.from(data, byte => byte.charCodeAt(0));
  }, rtcKey);
  const liveClock = () => page.evaluate(() => window.readLiveRtc());
  const autosave = () => page.evaluate(() => window.stateAutosave());

  await localRom(page, file);
  await page.locator('#pause').click();
  await clickSave(page);
  const initial = await history(page, 'list', scope);
  const capturedSave = await storedSave();
  const capturedClock = await storedClock();
  assert.deepEqual(capturedClock.slice(8, 11), [0, 0, 0]);

  await page.evaluate(() => {
    Object.defineProperty(document, 'hidden', { configurable: true, value: true });
    document.dispatchEvent(new Event('visibilitychange'));
    window.stateNow += 60_500;
    delete document.hidden;
    document.dispatchEvent(new Event('visibilitychange'));
    window.failStateWrite = true;
  });
  assert.deepEqual(await liveClock(), capturedClock, 'the paused clock has yet to sample the elapsed time');
  await clickLoad(page);
  const rebasedClock = await liveClock();
  assert.deepEqual(rebasedClock.slice(0, 16), capturedClock.slice(0, 16), 'Load restores the guest clock');
  assert.notDeepEqual(rebasedClock.slice(16), capturedClock.slice(16), 'Load rebases the host anchor');
  assert.equal(await page.locator('#pause').textContent(), 'Resume');
  assert.deepEqual(await storedClock(), capturedClock);
  await page.evaluate(() => { window.stateNow += 2_000; });
  await autosave();
  assert.deepEqual((await storedClock()).slice(8, 11), [0, 0, 2], 'only elapsed time since Load advances the clock');

  // Reset and play until the original fixture writes a newer boot count to SRAM.
  await page.locator('#reset').click();
  await page.evaluate(stepPlayerClock, 0);
  await page.evaluate(stepPlayerClock, 70);
  await autosave();
  const newerSave = await storedSave();
  assert.equal(Buffer.from(newerSave, 'base64')[0], Buffer.from(capturedSave, 'base64')[0] + 1);
  await page.evaluate(key => { window.failNativeKey = key; }, romKey);
  await clickLoad(page);
  assert.equal(await page.locator('#pause').textContent(), 'Pause');
  assert.deepEqual((await liveClock()).slice(0, 16), capturedClock.slice(0, 16));
  assert.equal(await storedSave(), newerSave);
  await autosave();
  await status(page, '#notice', 'Local save storage is unavailable or full.');
  assert.equal(await storedSave(), newerSave);
  await autosave();
  assert.equal(await storedSave(), capturedSave);
  assert.deepEqual(await storedClock(), await liveClock());
  assert.deepEqual(await history(page, 'list', scope), initial);
  assert.deepEqual(await page.evaluate(() => [window.stateWrites, window.failStateWrite]), [1, true]);
}

async function eepromLoadChecks(page, file, key, offset) {
  const scope = `${saveSlot(file)}:crystal`;
  const data = () => page.evaluate(key => localStorage.getItem(key), key);
  const count = record => Buffer.from(record, 'base64')[offset];
  await localRom(page, file);
  await page.locator('#pause').click();
  await clickSave(page);
  const initial = await history(page, 'list', scope);
  const captured = await data();
  assert.equal(count(captured), 1);
  await page.locator('#reset').click();
  await page.evaluate(stepPlayerClock, 0);
  await page.evaluate(stepPlayerClock, 70);
  await page.locator('#pause').click();
  const newer = await data();
  assert.equal(count(newer), 2);
  await clickLoad(page);
  assert.equal(await data(), newer);
  await page.evaluate(() => window.stateAutosave());
  assert.equal(await data(), captured);
  assert.deepEqual(await history(page, 'list', scope), initial);
  assert.equal(await page.evaluate(() => window.stateWrites), 1);
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

async function fullscreenTouchChecks(page, rom) {
  const targets = page.locator('#screen, #touch-controls button');
  const geometry = () => targets.evaluateAll(elements => elements.map(element => element.getBoundingClientRect().toJSON()));
  await localRom(page, rom);
  await page.locator('#fullscreen').tap();
  await page.waitForFunction(() => document.fullscreenElement?.id === 'game', undefined, { polling: 30 });
  await picker(page, true);
  const start = page.locator('#touch-controls .start');
  const box = await start.boundingBox();
  await start.evaluate(button => {
    window.gameplayPointerDown = 0;
    button.addEventListener('pointerdown', () => window.gameplayPointerDown++);
  });
  await page.touchscreen.tap(box.x + box.width / 2, box.y + box.height / 2);
  await page.locator('#state-picker').waitFor({ state: 'hidden' });
  assert.equal(await page.evaluate(() => window.gameplayPointerDown), 0, 'backdrop taps never reach the controller');
  assert.equal(await page.evaluate(() => document.activeElement.id), 'select-state');
  await picker(page, true);
  await page.locator('#state-new').tap();
  const gameplayGeometry = await geometry();
  await page.locator('#save-state').tap();
  await status(page, '#state-status', 'Save state saved.');
  assert.deepEqual(await geometry(), gameplayGeometry);
  const feedback = page.locator('#state-status');
  assert.ok(await feedback.isVisible());
  assert.equal(await feedback.evaluate(element => getComputedStyle(element).pointerEvents), 'none');
  await page.evaluate(() => { window.failStateWrite = true; });
  await page.locator('#save-state').tap();
  await status(page, '#state-status', 'aborted');
  assert.ok(await feedback.isVisible());
  assert.deepEqual(await geometry(), gameplayGeometry);
  await start.tap();
  assert.equal(await page.evaluate(() => window.gameplayPointerDown), 1);
  await page.locator('#exit-fullscreen').tap();
  await page.waitForFunction(() => !document.fullscreenElement && document.activeElement.id === 'fullscreen',
    undefined, { polling: 30 });
}

export async function saveStateChecks(browser, base, errors) {
  const [horizontal, vertical, rtc, cartridgeEeprom, internalEeprom] = await Promise.all([
    fixture('keypad-horizontal'), fixture('keypad-vertical'), fixture('sram128-2003'),
    fixture('eeprom128'), fixture('internal-eeprom'),
  ]);
  await withPlayer(browser, base, errors, page => pickerChecks(page, horizontal, vertical));
  await withPlayer(browser, base, errors, page => fullscreenChecks(page, horizontal));
  await withPlayer(browser, base, errors, page => rtcLoadChecks(page, rtc));
  await withPlayer(browser, base, errors, page => eepromLoadChecks(page, cartridgeEeprom, saveSlot(cartridgeEeprom), 0));
  await withPlayer(browser, base, errors, page => eepromLoadChecks(page, internalEeprom, 'wonderswan:console:v1:crystal', 2));
  const mobile = { isMobile: true, hasTouch: true, viewport: { width: 375, height: 812 } };
  await withPlayer(browser, base, errors, page => touchChecks(page, horizontal, vertical), mobile);
  await withPlayer(browser, base, errors, page => fullscreenTouchChecks(page, horizontal), mobile);
  console.log('Save states: windowed and native fullscreen workflows, atomic overwrite, rotated screenshots, import/export, paging, identity, direct Load with no history growth, native SRAM/EEPROM/RTC persistence, keyboard and touch passed');
}
