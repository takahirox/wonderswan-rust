// SPDX-License-Identifier: CC0-1.0
// Real player input -> Wasm -> cartridge CPU IN B5 -> native tile pixels.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

const vectors = [
  ['ArrowUp',16,0,1,0], ['ArrowRight',32,0,2,0],
  ['ArrowDown',64,0,4,0], ['ArrowLeft',128,0,8,0],
  ['i',256,1,0,0], ['l',512,2,0,0], ['k',1024,4,0,0], ['j',2048,8,0,0],
  ['Enter',2,0,0,2], ['z',4,0,0,4], ['x',8,0,0,8],
];
const rows = (y=0,x=0,a=0) => [0,0x10|y,0x20|x,0x30|y|x,0x40|a,0x50|y|a,0x60|x|a,0x70|y|x|a];

export async function keypadChecks(browser, base, errors) {
  const horizontal = await readFile(new URL('../../fixtures/generated/keypad-horizontal.ws', import.meta.url));
  const vertical = await readFile(new URL('../../fixtures/generated/keypad-vertical.ws', import.meta.url));
  const context = await browser.newContext({viewport:{width:1440,height:1000},hasTouch:true});
  const page = await context.newPage();
  page.setDefaultTimeout(10000);
  page.on('pageerror', e => errors.push(e.message));
  const requests = [];
  page.on('request', request => requests.push(request.method()));
  const click = id => page.locator(id).evaluate(button => button.click());
  const focus = () => page.locator('#screen').focus();
  let expectedError = '';
  async function expect(y=0,x=0,a=0) {
    const expected = rows(y,x,a);
    await page.waitForTimeout(35); // require fresh CPU/render work, including unchanged union values
    await page.waitForFunction(expected => {
      const c = document.querySelector('#screen').getContext('2d');
      return expected.every((value,row) => {
        let actual = 0;
        for (let bit=0;bit<8;bit++) {
          if (c.getImageData(bit*8+3,row*8+19,1,1).data[0] === 0) actual |= 1<<bit;
        }
        return actual === value;
      });
    }, expected).catch(async error => {
      console.error('Keypad observation', JSON.stringify({expected, state:await page.evaluate(() => ({
        focus:document.activeElement.id, status:document.querySelector('#rom-status').textContent,
        error:document.querySelector('#error').textContent,
        pressed:[...document.querySelectorAll('.is-pressed')].map(b=>b.dataset.button),
        events:window.keypadEvents,
      }))}));
      throw error;
    });
    assert.equal(await page.locator('#error').textContent(),expectedError);
  }
  async function load(bytes, name='keypad.ws') {
    expectedError = '';
    await page.locator('#rom').setInputFiles({name,mimeType:'application/octet-stream',buffer:bytes});
    await page.waitForFunction(() => document.querySelector('#rom-status').textContent === 'Running');
    await expect();
  }
  const cdp = await context.newCDPSession(page);
  let touches = [];
  async function touch(id, button) {
    const ended = touches.filter(point => point.id === id);
    if (button !== null) {
      await page.locator('#touch-controls').scrollIntoViewIfNeeded();
      const box = await page.locator(`[data-button="${button}"]`).boundingBox();
      touches.push({id,x:box.x+box.width/2,y:box.y+box.height/2});
    } else {
      touches = touches.filter(point => point.id !== id);
    }
    // Chromium ends the supplied contacts; an empty touchEnd ends the sequence.
    await cdp.send('Input.dispatchTouchEvent',{
      type:button !== null ? 'touchStart' : 'touchEnd',touchPoints:button !== null ? touches : ended,
    });
  }
  async function cancel() {
    touches=[];
    await cdp.send('Input.dispatchTouchEvent',{type:'touchCancel',touchPoints:[]});
  }
  async function visibility(hidden) {
    await page.evaluate(hidden => {
      if (hidden) Object.defineProperty(document,'hidden',{configurable:true,value:true});
      else delete document.hidden;
      document.dispatchEvent(new Event('visibilitychange'));
    }, hidden);
  }
  async function layout(orientation) {
    await page.waitForFunction(orientation => document.querySelector('#display').dataset.orientation === orientation,orientation);
    // ResizeObserver runs asynchronously after model/load/fullscreen transitions.
    await page.waitForTimeout(60);
    const result = await page.locator('#screen').evaluate(canvas => {
      const rect = element => {
        const r = element.getBoundingClientRect();
        return {x:r.x,y:r.y,width:r.width,height:r.height,right:r.right,bottom:r.bottom};
      };
      return {canvas:rect(canvas),frame:rect(document.querySelector('#display')),
        wrapper:rect(document.querySelector('#screen-presentation')),controls:rect(document.querySelector('#controls')),
        backing:[canvas.width,canvas.height],pixels:getComputedStyle(canvas).imageRendering,
        overflow:document.documentElement.scrollWidth>innerWidth};
    });
    assert.deepEqual(result.backing,[224,144]);
    assert.equal(result.pixels,'pixelated');
    assert.equal(result.overflow,false);
    const ratio = orientation === 'vertical' ? 144/224 : 224/144;
    assert.ok(Math.abs(result.canvas.width/result.canvas.height-ratio)<0.001);
    for (const side of ['width','height']) assert.ok(Math.abs(result.wrapper[side]-result.canvas[side])<0.1);
    assert.ok(result.canvas.x >= result.frame.x-0.1 && result.canvas.right <= result.frame.right+0.1);
    assert.ok(result.canvas.y >= result.frame.y-0.1 && result.canvas.bottom <= result.frame.bottom+0.1,JSON.stringify(result));
    assert.ok(result.canvas.bottom <= result.controls.y,'screen does not overlap toolbar');
    // Inspect composited screenshot pixels as well as the native canvas. This
    // catches missing/clockwise rotation, cropping, and smoothed corner markers.
    const png = await page.locator('#screen-presentation').screenshot();
    const samples = await page.evaluate(async ({png,vertical}) => {
      const image = new Image(); image.src=`data:image/png;base64,${png}`; await image.decode();
      const c = document.createElement('canvas');c.width=image.width;c.height=image.height;
      const ctx = c.getContext('2d');ctx.drawImage(image,0,0);
      const w=vertical ? 144 : 224, h=vertical ? 224 : 144;
      return [[3,3],[219,3],[3,139],[222,143],[223,142],[223,143]].map(([x,y]) => {
        const px=vertical ? y : x, py=vertical ? 223-x : y;
        return Array.from(ctx.getImageData(Math.floor((px+0.5)*c.width/w),Math.floor((py+0.5)*c.height/h),1,1).data);
      });
    }, {png:png.toString('base64'),vertical:orientation==='vertical'});
    assert.deepEqual(samples,[0,170,85,255,255,0].map(v=>[v,v,v,255]));
  }
  try {
    await page.goto(base);
    await page.evaluate(() => {
      window.keypadEvents=[];
      for (const type of ['pointerdown','pointerup','pointercancel','lostpointercapture']) {
        document.addEventListener(type,e => {
          window.keypadEvents.push([type,e.pointerId,e.target.dataset.button]);
          if (window.keypadEvents.length>15) window.keypadEvents.shift();
        },true);
      }
    });
    await page.waitForFunction(() => !document.querySelector('#model').disabled);
    assert.equal(await page.locator('[data-button]').count(),11);
    for (const [,mask] of vectors) {
      const button = page.locator(`[data-button="${mask}"]`);
      assert.ok(await button.getAttribute('aria-label') || await button.textContent(),'named touch button');
    }
    // Test exported adapter metadata in a separate real Wasm instance, including
    // rejected operations. The production player's machine is never replaced.
    const bindings = await readFile(new URL('../pkg/ws_web.js',import.meta.url),'utf8');
    const wasm = await readFile(new URL('../pkg/ws_web_bg.wasm',import.meta.url));
    const adapter = await page.evaluate(async ({bindings,wasm,horizontal,vertical}) => {
      const url=URL.createObjectURL(new Blob([bindings],{type:'text/javascript'}));
      try {
        const {default:init,WebMachine}=await import(url);
        await init({module_or_path:new Uint8Array(wasm)});
        const m=new WebMachine(); const results=[m.orientation()]; let rejections=0;
        for (const bytes of [vertical,horizontal,vertical]) {
          m.load_rom(new Uint8Array(bytes));
          const before=m.orientation();
          for (const model of ['mono','color','crystal']) {
            m.change_model(model); m.restart(); results.push(m.orientation());
          }
          try {m.load_rom(new Uint8Array([1]));} catch {rejections++;}
          results.push(m.orientation());
          if (m.orientation()!==before) throw new Error('rejected load changed orientation');
        }
        // Valid Color-only replacement rejects Mono before replacing state.
        const color=new Uint8Array(vertical);color[color.length-9]=1;
        let sum=0;for (const byte of color.subarray(0,-2)) sum=(sum+byte)&65535;
        color[color.length-2]=sum&255;color[color.length-1]=sum>>8;
        m.load_rom(color);
        try {m.change_model('mono');} catch {rejections++;}
        const preserved=[m.model(),m.orientation(),m.rom_len(),m.frame_width(),m.frame_height()];
        m.change_model('color');
        m.reset('mono');
        try {m.load_rom(color);} catch {rejections++;}
        const empty=m.orientation();
        m.free();return {results,rejections,preserved,empty};
      } finally {URL.revokeObjectURL(url);}
    }, {bindings,wasm:Array.from(wasm),horizontal:Array.from(horizontal),vertical:Array.from(vertical)});
    assert.deepEqual(adapter.results,['horizontal',...Array(4).fill('vertical'),...Array(4).fill('horizontal'),...Array(4).fill('vertical')]);
    assert.equal(adapter.rejections,5);
    assert.deepEqual(adapter.preserved,['crystal','vertical',131072,224,144]);
    assert.equal(adapter.empty,'horizontal');
    console.log('Keypad adapter orientation transitions and rejected operations passed');
    for (const model of ['mono','color','crystal']) {
      await page.locator('#model').selectOption(model);
      for (const [bytes,orientation] of [[horizontal,'horizontal'],[vertical,'vertical']]) {
        await load(bytes);
        await layout(orientation);
        for (const [key,mask,y,x,a] of vectors) {
          await page.keyboard.down(key);await expect(y,x,a);
          await page.keyboard.down(key);await expect(y,x,a); // native repeat
          await page.keyboard.up(key);await expect();
          await touch(1,mask);await expect(y,x,a);
          await touch(1,null);await expect();
        }
        for (const [key,,y,x,a] of vectors.slice(0,4).map((v,i)=>[['w','d','s','a'][i],...v.slice(1)])) {
          await page.keyboard.down(key);await expect(y,x,a);await page.keyboard.up(key);await expect();
        }
        // Alias/source union: releasing any one contributor retains the others.
        await page.keyboard.down('ArrowUp');await page.keyboard.down('w');
        await touch(1,16);await touch(2,16);await touch(3,512);await touch(4,4);
        await expect(2,1,4);
        await page.keyboard.up('ArrowUp');await touch(1,null);await expect(2,1,4);
        await page.keyboard.up('w');await expect(2,1,4);
        await touch(2,null);await expect(2,0,4);
        await touch(3,null);await expect(0,0,4);
        await touch(4,null);await expect();
        await page.keyboard.down('i');await touch(1,256);await expect(1,0,0);
        await touch(1,null);await expect(1,0,0);
        await page.keyboard.up('i');await expect();
        await focus();
        for (const key of ['ArrowUp','ArrowRight','i','j','z','Enter']) await page.keyboard.down(key);
        await expect(9,3,6); // independent diagonals plus A+Start
        for (const key of ['ArrowUp','ArrowRight','i','j','z','Enter']) await page.keyboard.up(key);
        await expect();
        await touch(1,256);await touch(2,32);await touch(3,8);await expect(1,2,8);
        await cancel();await expect();
        // Losing actual pointer capture clears only that pointer.
        await page.evaluate(() => {
          document.addEventListener('pointerdown',e => {window.keypadPointer=e.pointerId;},{once:true,capture:true});
        });
        await touch(1,1024);await expect(4,0,0);
        touches[0].x += 1;
        await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:touches});
        await page.evaluate(() => document.querySelector('[data-button="1024"]').releasePointerCapture(window.keypadPointer));
        touches[0].x += 1; // a real movement processes pending capture changes
        await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:touches});
        await expect();await touch(1,null);
        // Space and Enter activation on an accessible touch button also union.
        await page.locator('[data-button="2048"]').focus();
        await page.keyboard.down('Space');await page.keyboard.down('Enter');await expect(8,0,0);
        await page.keyboard.up('Space');await expect(8,0,0);
        await page.keyboard.up('Enter');await expect();await focus();
      }
      console.log(`${model}: eleven keyboard/touch buttons, WASD, repeats, all B5 row selections, source union, diagonals, cancel/capture and raw identities in both orientations passed`);
    }
    // Every transition begins with held X, Y and action sources, then observes
    // CPU-rendered release after execution resumes. Visibility/blur are synthetic.
    for (const action of ['blur','focus','pause','modal','background','pagehide','restart','model','load','invalid']) {
      await load(vertical);await focus();
      for (const key of ['ArrowUp','i','z']) await page.keyboard.down(key);
      await touch(1,32);await touch(2,512);await touch(3,8);await expect(3,3,12);
      if (action==='blur') await page.evaluate(() => window.dispatchEvent(new Event('blur')));
      if (action==='focus') {
        await page.locator('#volume').focus();await expect();
        for (const key of ['j','ArrowLeft','x']) await page.keyboard.press(key);
      }
      if (action==='pause') {await click('#pause');await click('#pause');}
      if (action==='modal') {
        await click('#settings');
        for (const key of ['j','ArrowLeft','x']) await page.keyboard.press(key);
        assert.equal(await page.locator('[data-button]:enabled').count(),0);
        await page.keyboard.press('Escape');
        await page.waitForFunction(() => document.querySelector('#rom-status').textContent === 'Running');
      }
      if (action==='background') {await visibility(true);await visibility(false);await click('#pause');}
      if (action==='pagehide') {await page.evaluate(() => window.dispatchEvent(new Event('pagehide')));await click('#pause');}
      if (action==='restart') await click('#reset');
      if (action==='model') await page.locator('#model').selectOption('color');
      if (action==='load') await load(vertical,'replacement.ws');
      if (action==='invalid') {
        await page.locator('#rom').setInputFiles({name:'bad.ws',mimeType:'application/octet-stream',buffer:Buffer.from([0])});
        await page.waitForFunction(() => document.querySelector('#error').textContent.includes('Could not load'));
        assert.equal(await page.locator('#display').getAttribute('data-orientation'),'vertical');
        expectedError = await page.locator('#error').textContent();
        await expect(); // the retained CPU reads cleared input before restarting
        await click('#reset');
        expectedError = '';
      }
      await expect();
      for (const key of ['ArrowUp','i','z']) await page.keyboard.up(key);
      await cancel();await expect();
    }
    // Native square-pixel geometry and direction on 375px and both fullscreen layouts.
    for (const viewport of [{width:375,height:812},{width:1440,height:1000}]) {
      await page.setViewportSize(viewport);
      for (const [bytes,orientation] of [[vertical,'vertical'],[horizontal,'horizontal']]) {
        await load(bytes);await layout(orientation);
        if (await page.evaluate(() => document.fullscreenEnabled)) {
          await page.locator('#fullscreen').click();
          await page.waitForFunction(() => !!document.fullscreenElement);
          await layout(orientation);
          await page.locator('#fullscreen').click();
          await page.waitForFunction(() => !document.fullscreenElement);
        }
      }
    }
    await load(vertical,'renamed.ws');
    await click('#pause');
    const key=`wonderswan:save:v1:${createHash('sha256').update(vertical).digest('hex')}`;
    assert.deepEqual(await page.evaluate(key=>Array.from(atob(localStorage.getItem(key)).slice(0,8),c=>c.charCodeAt(0)),key),rows());
    assert.ok(requests.every(method=>method==='GET'),'input, local cartridge selection and saving never upload');
    console.log('Adapter default/load/restart/model/rejected-load metadata, input lifecycle/focus isolation, portrait pixel direction, desktop/375px/fullscreen layout, and raw SHA-256 save identity passed');
  } finally {await context.close();}
}
