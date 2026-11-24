// Production browser checks with original CC0 fixtures and real IndexedDB.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';

const ready = page => page.waitForFunction(() => !document.querySelector('#model').disabled);
const countRows = (page, count) => page.waitForFunction(count =>
  document.querySelectorAll('#cartridge-list li').length === count, count);
const status = (page, text) => page.waitForFunction(text => document.querySelector('#rom-status').textContent === text, text);
const title = (page, text) => page.waitForFunction(text => document.querySelector('#rom-title').textContent === text, text);
const textIncludes = (page, selector, text) => page.waitForFunction(({selector,text}) =>
  document.querySelector(selector).textContent.includes(text), {selector,text});

export async function openCartridges(page) {
  await page.locator('#load-rom').click();
  return page.getByRole('dialog', {name:'Insert cartridge'});
}

export async function chooseRemembered(page, name, key) {
  const picker = await openCartridges(page);
  const description = key ? {description:key.slice(-64,-52)} : {};
  await picker.getByRole('button', {name, exact:true, ...description}).click();
}

async function importFile(page, file) {
  const picker = await openCartridges(page);
  const [chooser] = await Promise.all([
    page.waitForEvent('filechooser'), picker.getByRole('button', {name:'Open local ROM…'}).click(),
  ]);
  await chooser.setFiles(file);
}

async function editRecord(page, {key, name, body, missingMetadata}) {
  await page.evaluate(async ({key,name,body,missingMetadata}) => {
    const db = await new Promise((resolve,reject) => {
      const request = indexedDB.open('wonderswan-rust:cartridges',1);
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
    try {
      await new Promise((resolve,reject) => {
        const tx = db.transaction(['cartridges','roms'],'readwrite');
        tx.oncomplete = resolve;
        tx.onabort = () => reject(tx.error);
        if (name !== undefined) tx.objectStore('cartridges').put({name},key);
        if (missingMetadata) tx.objectStore('cartridges').delete(key);
        if (body === 'missing') tx.objectStore('roms').delete(key);
        if (body === 'mismatch') tx.objectStore('roms').put(new Uint8Array(131072),key);
      });
    } finally { db.close(); }
  },{key,name,body,missingMetadata});
}

export async function romLibraryChecks(browser, base, errors) {
  const fixture = async (fixtureName, name=fixtureName+'.ws') => ({
    name, mimeType:'application/octet-stream',
    buffer:await readFile(new URL(`../../fixtures/generated/${fixtureName}.ws`,import.meta.url)),
  });
  const [first, second, color] = await Promise.all([
    fixture('keypad-horizontal','same.ws'), fixture('keypad-vertical','same.ws'), fixture('startup-color-common'),
  ]);
  const identity = file => `wonderswan:save:v1:${createHash('sha256').update(file.buffer).digest('hex')}`;
  const firstKey = identity(first), secondKey = identity(second);
  const context = await browser.newContext({viewport:{width:1280,height:800},hasTouch:true});
  await context.addInitScript(() => {
    window.romReads = 0;
    window.abortOperation = null;
    window.holdWrite = false;
    for (const method of ['get','getAll','openCursor','put','delete']) {
      const original = IDBObjectStore.prototype[method];
      IDBObjectStore.prototype[method] = function (...args) {
        const request = original.apply(this,args);
        if (this.name !== 'roms') return request;
        if (method === 'put' && window.holdWrite) {
          const keepAlive = () => {
            window.writePending = true;
            if (window.holdWrite) this.transaction.objectStore('cartridges').count().onsuccess = keepAlive;
          };
          request.addEventListener('success',keepAlive,{once:true});
        }
        if (['put','delete'].includes(method)) {
          if (window.abortOperation === method) {
            window.abortOperation = null;
            request.addEventListener('success',() => this.transaction.abort(),{once:true});
          }
        } else window.romReads++;
        return request;
      };
    }
  });
  const page = await context.newPage();
  page.on('pageerror',error => errors.push(error.message));
  const requests = [], choosers = [];
  page.on('filechooser',chooser => choosers.push(chooser));
  try {
    await page.goto(base); await ready(page);
    page.on('request',request => requests.push(request.url()));
    assert.equal(await page.locator('#rom-title').textContent(),'No ROM loaded');
    await page.locator('#load-rom').focus();
    await page.keyboard.press('Enter');
    await page.locator('#cartridge-picker').waitFor();
    assert.equal(await page.locator('#my-cartridges').isVisible(),false);
    await page.keyboard.press('Tab');
    assert.equal(await page.evaluate(() => document.activeElement.id),'open-local-rom');
    await page.keyboard.press('Escape');
    assert.equal(await page.evaluate(() => document.activeElement.id),'load-rom');
    await openCartridges(page);
    await page.mouse.click(2,2);
    await page.locator('#cartridge-picker').waitFor({state:'hidden'});
    assert.equal(await page.evaluate(() => document.activeElement.id),'load-rom');

    await page.evaluate(() => { window.holdWrite = true; });
    await importFile(page,first); await status(page,'Running');
    await page.waitForFunction(() => window.writePending);
    await openCartridges(page); await countRows(page,0);
    await page.evaluate(() => { window.holdWrite = false; });
    await countRows(page,1);
    await page.keyboard.press('Escape');
    await page.locator('#pause').click();
    const longName = `${'long-filename-'.repeat(16)}.wsc`;
    await importFile(page,{...first,name:longName}); await status(page,'Paused');
    assert.equal(await page.locator('#rom-title').textContent(),first.name);
    const picker = await openCartridges(page);
    await picker.getByRole('button',{name:longName,exact:true}).waitFor();
    await countRows(page,1);
    await page.keyboard.press('Escape');
    await importFile(page,second); await status(page,'Running');
    await importFile(page,first); await status(page,'Running');
    await title(page,first.name);
    await openCartridges(page); await countRows(page,2);
    assert.equal(await page.evaluate(() => window.romReads),0,'picker opens without reading ROM bodies');
    await page.keyboard.press('Escape');
    assert.deepEqual(requests,[],'local imports make no network requests');

    await page.reload(); await ready(page);
    requests.length = 0;
    const numChoosers = choosers.length;
    await status(page,'No ROM loaded');
    await chooseRemembered(page,second.name,secondKey); await status(page,'Running');
    assert.equal(await page.locator('#game').getAttribute('data-orientation'),'vertical');
    assert.equal(await page.evaluate(() => window.romReads),1,'stored selection reads one ROM body');
    assert.equal(choosers.length,numChoosers);
    assert.deepEqual(requests,[],'stored ROM and progress stay local');

    // The running guest keeps executing while the picker is open or a removal commits.
    await page.locator('#settings').click();
    await page.locator('#debug-toggle').check();
    await page.locator('#help-close').click(); await status(page,'Running');
    await page.waitForFunction(() => Number(document.querySelector('#debug-frames').textContent) > 10);
    const frames = Number(await page.locator('#debug-frames').textContent());
    await openCartridges(page); await countRows(page,2);
    const removal = picker.getByRole('button',{name:'Remove same.ws',description:secondKey.slice(-64,-52)});
    await page.evaluate(() => { window.abortOperation = 'delete'; });
    await removal.click();
    await textIncludes(page,'#library-operation','Could not remove');
    await countRows(page,2);
    await page.keyboard.press('Escape');
    await page.evaluate(() => { window.holdWrite = true; window.writePending = false; });
    await importFile(page,second); await status(page,'Running');
    await page.waitForFunction(() => window.writePending);
    await openCartridges(page);
    // Remove queues behind the in-flight reimport of this same cartridge.
    await removal.click();
    assert.equal(await removal.getAttribute('aria-disabled'),'true');
    await countRows(page,2);
    await page.evaluate(() => { window.holdWrite = false; });
    await countRows(page,1);
    assert.equal(await page.locator('#library-operation').textContent(),'');
    await status(page,'Running');
    assert.equal(await page.locator('#game').getAttribute('data-orientation'),'vertical');
    await page.waitForFunction(frames => Number(document.querySelector('#debug-frames').textContent) > frames,frames);
    await page.keyboard.press('Escape');

    // A stored selection whose identity check finishes after a newer import cannot replace it.
    await page.evaluate(() => {
      const digest = crypto.subtle.digest.bind(crypto.subtle);
      window.holdDigest = true;
      crypto.subtle.digest = async (...args) => {
        if (!window.holdDigest) return digest(...args);
        window.holdDigest = false;
        await new Promise(resolve => { window.finishStoredRead = resolve; });
        const result = await digest(...args);
        window.staleDigestDone = true;
        return result;
      };
    });
    await chooseRemembered(page,first.name);
    await page.waitForFunction(() => typeof window.finishStoredRead === 'function');
    await importFile(page,color); await title(page,color.name);
    await page.evaluate(() => window.finishStoredRead());
    await page.waitForFunction(() => window.staleDigestDone);
    assert.equal(await page.locator('#rom-title').textContent(),color.name);
    await page.locator('#model').selectOption('mono');
    await textIncludes(page,'#error','Could not reset');
    // Switch to a compatible cartridge before testing mono rejection.
    await importFile(page,second); await title(page,second.name);
    await page.locator('#model').selectOption('mono');
    await chooseRemembered(page,color.name);
    await textIncludes(page,'#error','Could not load');
    assert.equal(await page.locator('#model').inputValue(),'mono');
    assert.equal(await page.locator('#game').getAttribute('data-orientation'),'vertical');
    await importFile(page,{...color,name:'incompatible.ws'});
    await textIncludes(page,'#error','incompatible.ws');
    await importFile(page,{...first,name:'invalid.ws',buffer:Buffer.alloc(16)});
    await textIncludes(page,'#error','invalid.ws');
    await page.evaluate(() => Object.defineProperty(File.prototype,'size',{
      configurable:true, get() { return this.name === 'oversized.ws' ? 64*1024*1024+1 : Reflect.get(Blob.prototype,'size',this); },
    }));
    await importFile(page,{...first,name:'oversized.ws'});
    await textIncludes(page,'#error','at most 64 MiB');
    await openCartridges(page); await countRows(page,3);
    assert.equal(await picker.getByRole('button',{name:color.name,exact:true}).count(),1,'incompatible import does not rename its entry');
    await page.keyboard.press('Escape');
    await page.evaluate(() => { delete File.prototype.size; });

    for (const body of ['missing','mismatch']) {
      await editRecord(page,{key:firstKey,body});
      await chooseRemembered(page,first.name,firstKey);
      await textIncludes(page,'#cartridge-notice-message','Reopen the original file or remove the entry');
      assert.equal(await page.locator('#game').getAttribute('data-orientation'),'vertical');
      await importFile(page,first);
      await page.waitForFunction(() => document.querySelector('#game').dataset.orientation === 'horizontal');
      await page.waitForFunction(() => document.querySelector('#cartridge-notice').hidden);
      await importFile(page,second);
      await page.waitForFunction(() => document.querySelector('#game').dataset.orientation === 'vertical');
    }
    await editRecord(page,{key:firstKey,name:7});
    await editRecord(page,{key:19,name:'invalid-key.ws'});
    await openCartridges(page);
    await picker.getByRole('button',{name:'Unnamed cartridge',exact:true}).waitFor();
    const reads = await page.evaluate(() => window.romReads);
    await picker.getByRole('button',{name:'invalid-key.ws',exact:true}).click();
    await textIncludes(page,'#cartridge-notice-message','Could not read');
    assert.equal(await page.evaluate(() => window.romReads),reads,'malformed key is rejected before fetching bytes');
    await openCartridges(page);
    await picker.getByRole('button',{name:'Remove invalid-key.ws',exact:true}).click();
    await countRows(page,3);
    await picker.getByRole('button',{name:'Remove Unnamed cartridge',exact:true}).focus();
    await page.keyboard.down('Enter');
    await countRows(page,2);
    await page.keyboard.down('Enter'); await page.keyboard.up('Enter');
    assert.equal(await picker.isVisible(),true,'held Enter does not activate the next focused control');
    await page.keyboard.press('Escape');
    await status(page,'Running');

    // Discover a stored body with no metadata after reload, without reading it.
    await importFile(page,first); await title(page,first.name);
    await openCartridges(page);
    const namedEntry = picker.getByRole('button',{
      name:first.name,exact:true,description:firstKey.slice(-64,-52),
    });
    await namedEntry.waitFor();
    await page.keyboard.press('Escape');
    await editRecord(page,{key:firstKey,missingMetadata:true});
    await page.reload(); await ready(page);
    await openCartridges(page);
    const unnamed = picker.getByRole('button',{name:'Unnamed cartridge',exact:true});
    await unnamed.waitFor();
    await countRows(page,3);
    assert.equal(await page.evaluate(() => window.romReads),0,'discovering an orphan does not read its body');
    await unnamed.click();
    await textIncludes(page,'#cartridge-notice-message','Reopen the original file or remove the entry');
    await title(page,'No ROM loaded'); await status(page,'No ROM loaded');
    await importFile(page,first); await title(page,first.name);
    await openCartridges(page); await namedEntry.waitFor();
    await countRows(page,3);
    assert.equal(await picker.locator('#library-operation').textContent(),'');
    await page.keyboard.press('Escape');

    await editRecord(page,{key:firstKey,missingMetadata:true});
    await openCartridges(page);
    await picker.getByRole('button',{name:'Remove Unnamed cartridge',exact:true}).click();
    await countRows(page,2);
    await title(page,first.name); await status(page,'Running');
    await page.reload(); await ready(page);
    await openCartridges(page); await countRows(page,2);
    await page.keyboard.press('Escape');

    // A deletion blocked in another tab queues opens without notifying them.
    const otherTab = await context.newPage();
    await otherTab.goto(base);
    await otherTab.evaluate(async () => {
      window.libraryConnection = await new Promise((resolve,reject) => {
        const request = indexedDB.open('wonderswan-rust:cartridges',1);
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
      });
      await new Promise((resolve,reject) => {
        const deletion = indexedDB.deleteDatabase('wonderswan-rust:cartridges');
        window.libraryDeletion = new Promise(done => { deletion.onsuccess = done; });
        deletion.onblocked = resolve;
        deletion.onerror = () => reject(deletion.error);
      });
    });
    await page.bringToFront();
    await page.evaluate(() => {
      const open = indexedDB.open.bind(indexedDB);
      window.queuedOpens = [];
      indexedDB.open = (...args) => {
        const request = open(...args);
        window.queuedOpens.push(new Promise(resolve => {
          request.addEventListener('success',resolve,{once:true});
          request.addEventListener('error',resolve,{once:true});
        }));
        return request;
      };
    });
    const queued = {...first,name:'queued-import.ws'};
    await importFile(page,queued); await title(page,queued.name); await status(page,'Running');
    await openCartridges(page);
    await page.waitForFunction(() =>
      document.querySelector('#library-list-status').textContent.includes('Reopen the picker to retry') &&
      document.querySelector('#library-retention').textContent.includes('reopen the original file to retry'),
    null,{timeout:5000});
    assert.equal(await picker.locator('#cartridge-list li').count(),0,'failed refresh clears stale rows');
    await otherTab.evaluate(async () => { window.libraryConnection.close(); await window.libraryDeletion; });
    await page.evaluate(async () => { await Promise.all(window.queuedOpens); });
    assert.equal(await page.evaluate(async () => (await indexedDB.databases())
      .some(db => db.name === 'wonderswan-rust:cartridges')),false,'expired opens cannot recreate the database or write');
    await otherTab.close();
    await page.keyboard.press('Escape');
    await importFile(page,queued);
    await openCartridges(page); await countRows(page,1);
    assert.equal(await picker.locator('#library-list-status').textContent(),'');
    assert.equal(await picker.locator('#library-retention').textContent(),'');
  } finally { await context.close(); }
  console.log('Real IndexedDB cartridge commits, deduplication, raw identity, reload, corruption, model validation, removal, stale reads, blocked deletion recovery and local-only requests passed');

  const mobile = await browser.newContext({viewport:{width:375,height:812},isMobile:true,hasTouch:true});
  const phone = await mobile.newPage();
  phone.on('pageerror',error => errors.push(error.message));
  const cdp = await mobile.newCDPSession(phone);
  const save = await fixture('sram32-common','battery-save.ws');
  const origin = new URL(base).origin;
  try {
    await phone.goto(base); await ready(phone);
    await phone.evaluate(key => {
      localStorage.setItem(key,'invalid!');
      window.originalIndexedDB = Object.getOwnPropertyDescriptor(window,'indexedDB');
    },identity(save));
    // A throwing accessor and a missing API both leave the same local file route usable.
    for (const mode of ['denied','unavailable']) {
      await phone.evaluate(mode => Object.defineProperty(window,'indexedDB',{
        configurable:true,
        get() {
          if (mode === 'denied') throw new DOMException('Blocked','SecurityError');
          return undefined;
        },
      }),mode);
      await openCartridges(phone);
      await textIncludes(phone,'#library-list-status','Could not read My cartridges');
      await phone.keyboard.press('Escape');
      const name = mode === 'denied' ? save.name : 'unavailable.ws';
      await importFile(phone,{...save,name});
      await status(phone,'Running');
      await textIncludes(phone,'#cartridge-notice-message','Could not remember '+name);
      assert.match(await phone.locator('#notice').textContent(),/local save could not be read/);
      let failedName = name;
      if (mode === 'denied') {
        failedName = 'keypad-vertical.ws';
        await importFile(phone,{...second,name:failedName});
        await title(phone,failedName);
        await status(phone,'Running');
        await textIncludes(phone,'#cartridge-notice-message',`Could not remember ${failedName}`);
        const buttons = await phone.locator('#touch-controls button:enabled').all();
        assert.equal(buttons.length,11);
        const {width,height} = phone.viewportSize();
        const cue = phone.getByText('Storage warning',{exact:true});
        for (const element of [phone.locator('#screen'),cue,...buttons]) {
          const box = await element.boundingBox();
          assert.ok(box && box.width > 0 && box.height > 0 && box.x >= 0 && box.y >= 0
            && box.x+box.width <= width && box.y+box.height <= height,
            `${element} fits alongside the picture, storage warning cue and touch controls (${JSON.stringify(box)})`);
        }
        for (const button of buttons) await button.tap({trial:true});
      }
      await phone.getByRole('button',{name:'Insert cartridge',exact:true,description:/Could not remember/}).tap();
      await countRows(phone,0);
      await textIncludes(phone,'#library-retention',`Could not remember ${failedName}. Allow or free browser storage`);
      await phone.keyboard.press('Escape');
      await phone.evaluate(() => Object.defineProperty(window,'indexedDB',window.originalIndexedDB));
    }
    await openCartridges(phone);
    await phone.waitForFunction(() => document.querySelector('#library-list-status').textContent === '');
    await phone.keyboard.press('Escape');
    await cdp.send('Storage.overrideQuotaForOrigin',{origin,quotaSize:1});
    const longName = `${'local-cartridge-'.repeat(15)}tail-name-end.wsc`;
    await importFile(phone,{...save,name:longName});
    await textIncludes(phone,'#cartridge-notice-message','Could not remember local-cartridge-');
    const saveWarning = phone.locator('#notice');
    assert.match(await saveWarning.textContent(),/local save could not be read/);
    await saveWarning.scrollIntoViewIfNeeded();
    const progressBounds = await saveWarning.boundingBox();
    const storageBounds = await phone.locator('#cartridge-notice').boundingBox();
    assert.ok(storageBounds.y >= progressBounds.y+progressBounds.height,'storage warning leaves progress feedback clear');
    assert.ok(storageBounds.height < 120,'long filenames keep the warning compact');
    await status(phone,'Running');
    const picker = await openCartridges(phone); await countRows(phone,0);
    await phone.keyboard.press('Escape');
    await cdp.send('Storage.overrideQuotaForOrigin',{origin});
    await importFile(phone,second); await title(phone,second.name);
    await phone.setViewportSize({width:680,height:340});
    await openCartridges(phone); await countRows(phone,1);
    await textIncludes(phone,'#library-retention','Allow or free browser storage');
    assert.ok(await phone.locator('#cartridge-list').evaluate(list => list.clientHeight >= 52));
    await picker.getByRole('button',{name:'Open local ROM…'}).tap({trial:true});
    await picker.getByRole('button',{name:`Remove ${second.name}`,exact:true}).tap();
    await countRows(phone,0);
    await phone.keyboard.press('Escape');
    await phone.getByRole('button',{name:'Dismiss ROM storage warning'}).tap();
    assert.equal(await phone.locator('#cartridge-notice').isVisible(),false);

    // Force an abort after a successful binary put. Both stores must roll back.
    await phone.evaluate(() => {
      for (const method of ['put','delete']) {
        const original = IDBObjectStore.prototype[method];
        IDBObjectStore.prototype[method] = function (...args) {
          const request = original.apply(this,args);
          if (this.name === 'roms' && window.abortOperation === method) {
            window.abortOperation = null;
            request.addEventListener('success',() => this.transaction.abort(),{once:true});
          }
          return request;
        };
      }
      window.abortOperation = 'put';
    });
    await importFile(phone,{...first,name:'aborted.ws'}); await status(phone,'Running');
    await textIncludes(phone,'#cartridge-notice-message','Could not remember aborted.ws');
    await openCartridges(phone); await countRows(phone,0);
    await phone.keyboard.press('Escape');
    await importFile(phone,{...first,name:longName}); await status(phone,'Running');
    await importFile(phone,save); await title(phone,save.name);
    await phone.waitForFunction(() => document.querySelector('#library-retention').textContent === '');
    for (let i=0; i<8; i++) {
      // Keep the fixture intact in the executable region and update its checksum.
      const buffer = Buffer.from(second.buffer);
      buffer[100+i] ^= 1;
      const sum = buffer.subarray(0,-2).reduce((sum,value) => sum+value,0);
      buffer.writeUInt16LE(sum & 65535,buffer.length-2);
      await importFile(phone,{...second,name:`spare-${i}.ws`,buffer});
      await title(phone,`spare-${i}.ws`);
    }
    await chooseRemembered(phone,longName); await title(phone,longName);
    await phone.locator('#settings').click();
    await phone.locator('#debug-toggle').check();
    await phone.locator('#help-close').click(); await status(phone,'Running');
    await phone.locator('#pause').click(); await status(phone,'Paused');
    const frames = await phone.locator('#debug-frames').textContent();
    await chooseRemembered(phone,longName); await status(phone,'Paused');
    assert.equal(await phone.locator('#debug-frames').textContent(),frames);
    const remove = picker.getByRole('button',{name:`Remove ${longName}`,exact:true});
    for (const [width,height] of [[375,812],[680,340]]) {
      await phone.setViewportSize({width,height});
      await phone.locator('#load-rom').tap();
      await countRows(phone,10);
      const bounds = await picker.boundingBox();
      assert.ok(bounds.x >= 0 && bounds.x+bounds.width <= width
        && bounds.y >= 0 && bounds.y+bounds.height <= height);
      const fits = await phone.locator('#cartridge-list').evaluate(list => ({
        tall:list.scrollHeight > list.clientHeight, wide:list.scrollWidth > list.clientWidth,
      }));
      assert.equal(fits.wide,false);
      if (height === 340) assert.equal(fits.tall,true);
      assert.ok((await picker.getByRole('button',{name:'Open local ROM…'}).boundingBox()).y+44 <= height);
      if (height === 812) {
        await phone.evaluate(() => { window.abortOperation = 'delete'; });
        const target = await remove.boundingBox();
        const x = target.x+target.width/2, y = target.y+12;
        await phone.touchscreen.tap(x,y);
        await textIncludes(phone,'#library-operation','Could not remove');
        const underTap = await phone.evaluate(({x,y}) =>
          document.elementFromPoint(x,y)?.closest('button')?.getAttribute('aria-label'),{x,y});
        assert.equal(underTap,`Remove ${save.name}`,'the warning moves another Remove under the first tap');
        await phone.touchscreen.tap(x,y);
        assert.equal(await picker.getByRole('button',{name:`Remove ${save.name}`,exact:true}).getAttribute('aria-disabled'),null);
        assert.equal(await picker.locator('#cartridge-list li').count(),10);
        await phone.keyboard.press('Escape');
      }
    }
    await remove.scrollIntoViewIfNeeded();
    const bounds = await remove.boundingBox();
    assert.ok(bounds.height >= 44);
    const x = bounds.x+bounds.width/2, y = bounds.y+bounds.height/2;
    await phone.touchscreen.tap(x,y);
    await countRows(phone,9);
    assert.equal(await picker.locator('#library-operation').textContent(),'');
    await phone.touchscreen.tap(x,y);
    assert.equal(await picker.isVisible(),true,'repeat tap does not launch or dismiss');
    await countRows(phone,9);
    await phone.keyboard.press('Escape');
    assert.equal(await phone.locator('#debug-frames').textContent(),frames);
    await title(phone,longName); await status(phone,'Paused');
  } finally { await mobile.close(); }
  console.log('Denied/unavailable IndexedDB, quota, transaction rollback and retries, independent progress notices, touch removal, long filenames and compact scroll layout passed');
}
