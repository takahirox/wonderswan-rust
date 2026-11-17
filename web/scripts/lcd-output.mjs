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
    await context.addInitScript(() => {
      localStorage.setItem('wonderswan:audio:v1','{"muted":true,"volume":70}');
      localStorage.setItem('wonderswan:display:v1','clean');
    });
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
  await displayEffectsChecks(browser,base,errors);
  console.log('LCD output probe: literal adjacent rows 38/39/40 and 142/143, retained tile identities, transparency/sprites, line40/144 palette writes and fresh restart CPU markers passed on all three production Wasm models');
}

// Observe the production shader, injecting small synthetic RGBA patterns only at
// its upload boundary. No test API or modified core/cartridge is shipped.
async function displayEffectsChecks(browser, base, errors) {
  const context = await browser.newContext({viewport:{width:1440,height:1000}});
  await context.addInitScript(() => {
    window.displayProbe = {pattern:null, uploads:0, histories:[], draws:0, allocations:0, capture:false, samples:null};
    const p = window.displayProbe;
    const upload = WebGL2RenderingContext.prototype.texSubImage2D;
    WebGL2RenderingContext.prototype.texSubImage2D = function(...args) {
      p.uploads++;
      if (p.pattern) {
        const bytes = new Uint8Array(224*144*4);
        p.source = p.pattern === 'alternating' ? (p.uploads%2 ? 32 : 224) : 160;
        for (let y=0;y<144;y++) for (let x=0;x<224;x++) {
          const value = p.pattern === 'edge' ? (x<112 ? 32 : 224) : p.source;
          bytes.set([value,value,value,p.pattern === 'alpha' ? 128 : 255],(y*224+x)*4);
        }
        args[8] = bytes;
      }
      return upload.apply(this,args);
    };
    const allocate = WebGL2RenderingContext.prototype.texImage2D;
    WebGL2RenderingContext.prototype.texImage2D = function(...args) {
      p.allocations++;
      return allocate.apply(this,args);
    };
    const draw = WebGL2RenderingContext.prototype.drawArrays;
    WebGL2RenderingContext.prototype.drawArrays = function(...args) {
      draw.apply(this,args);
      if (this.getParameter(this.FRAMEBUFFER_BINDING)) {
        if (p.pattern === 'alternating') {
          const pixel = new Uint8Array(4);
          this.readPixels(100,64,1,1,this.RGBA,this.UNSIGNED_BYTE,pixel);
          p.histories.push({source:p.source, actual:pixel[0]});
          if (p.histories.length>32) p.histories.shift();
        }
      } else {
        p.draws++;
        if (p.capture) {
          p.capture = false;
          const sx = this.drawingBufferWidth/224, sy = this.drawingBufferHeight/144;
          p.samples = [...[100.125,100.375,100.625,100.875,110.375,111.375,112.375].map(x => [x,64.375]),[100.375,64.125],[100.375,64.875]].map(([x,y]) => {
            const pixel = new Uint8Array(4);
            this.readPixels(Math.floor(x*sx),Math.floor((144-y)*sy),1,1,this.RGBA,this.UNSIGNED_BYTE,pixel);
            return Array.from(pixel);
          });
        }
      }
    };
  });
  const page = await context.newPage();
  page.on('pageerror', error => errors.push(error.message));
  const ready = () => page.waitForFunction(() => !document.querySelector('#model').disabled);
  const actual = () => page.locator('#screen').getAttribute('data-effect');
  const select = async mode => {
    await page.locator('#display-effect').focus();
    await page.locator('#display-effect').selectOption(mode);
    assert.equal(await actual(),mode);
  };
  const snapshot = async () => {
    await page.evaluate(() => {
      window.displayProbe.capture = true;
      window.displayProbe.samples = null;
      const frame = document.querySelector('#display');
      // The initial CSS width is already 100%; change geometry on the first call.
      frame.style.width = frame.style.width === 'calc(100% - 1px)' ? '100%' : 'calc(100% - 1px)';
    });
    await page.waitForFunction(() => window.displayProbe.samples !== null);
    return page.evaluate(() => window.displayProbe.samples);
  };
  try {
    await page.goto(base);
    await ready();
    assert.equal(await page.locator('#display-effect').inputValue(),'retro-crt','fresh visitor default');
    assert.equal(await actual(),'retro-crt','shader is required in this Chromium check, never silently skipped');
    assert.equal(await page.locator('#display-notice').textContent(),'');
    assert.equal(await page.locator('#empty-state').isVisible(),true);
    await page.locator('#demo').click();
    await page.waitForFunction(() => document.querySelector('#rom-status').textContent === 'Running');
    await page.locator('#settings').click();
    await page.locator('#debug-toggle').check();
    const counter = await page.locator('#debug-frames').textContent();
    const sound = await page.locator('#sound-status').textContent();
    // Flat cells isolate matrix detail from game content; the edge isolates softness.
    await page.evaluate(() => { window.displayProbe.pattern = 'flat'; });
    await select('lcd-light');
    const light = await snapshot();
    assert.ok(light[1][0] > light[0][0]+2,'native cell center brighter than edge');
    assert.deepEqual(light[1],light[2],'flat pixel center symmetry');
    assert.ok(light.every(pixel => pixel[3] === 255),'opaque framebuffer stays opaque');
    await select('lcd-authentic');
    const authentic = await snapshot();
    assert.ok(authentic[1][0]-authentic[0][0] > light[1][0]-light[0][0],'stronger authentic cell grid');
    await select('retro-crt');
    const crt = await snapshot();
    assert.notDeepEqual(crt,light);
    assert.notDeepEqual(crt,authentic);
    await page.evaluate(() => { window.displayProbe.pattern = 'alpha'; });
    await select('lcd-light');
    assert.ok((await snapshot()).every(pixel => pixel[3] === 128),'source alpha survives the shader');
    await page.evaluate(() => { window.displayProbe.pattern = 'edge'; });
    await select('lcd-authentic');
    const edge = await snapshot();
    assert.ok(edge[5][0] > edge[4][0],'restrained adjacent-cell softness');
    await select('clean');
    assert.equal(await page.locator('#screen-effect').isVisible(),false);
    assert.equal(await page.locator('#debug-frames').textContent(),counter,'paused switches do not run cycles');
    assert.equal(await page.locator('#sound-status').textContent(),sound,'paused switches preserve audio');
    assert.equal(await page.evaluate(() => document.activeElement.id),'display-effect');
    await page.locator('#help-close').click();
    await page.waitForFunction(() => !document.querySelector('#help').open && document.querySelector('#rom-status').textContent === 'Running');
    assert.equal(await page.evaluate(() => document.activeElement.id),'screen','dialog restores gameplay focus');

    // Every actual shader history update must follow a source upload. Repeated
    // paused resizing/presentation must leave that recurrence and texture count alone.
    for (const [mode,weight] of [['lcd-light',0.04],['lcd-authentic',0.13]]) {
      await page.locator('#settings').click();
      await page.evaluate(() => { window.displayProbe.pattern='alternating'; window.displayProbe.histories=[]; });
      await select(mode);
      await page.locator('#help-close').click();
      await page.waitForFunction(() => window.displayProbe.histories.length>=7);
      await page.locator('#pause').click();
      const samples = await page.evaluate(() => window.displayProbe.histories);
      assert.equal(samples[0].actual,samples[0].source,'mode/resume seeds current frame');
      // Resume resets history too, so its first new source is seeded; subsequent
      // frames, including catch-up frames, follow the same bounded recurrence.
      assert.equal(samples[1].actual,samples[1].source,'resume seeds first new frame');
      for (let i=2;i<samples.length;i++) {
        const expected = samples[i].source*(1-weight)+samples[i-1].actual*weight;
        assert.ok(Math.abs(samples[i].actual-expected)<=1,`${mode} emulated-frame response ${i}`);
      }
      const before = await page.evaluate(() => ({uploads:window.displayProbe.uploads, histories:window.displayProbe.histories,allocations:window.displayProbe.allocations}));
      const first = await snapshot();
      assert.deepEqual(await snapshot(),first,'repeated paused redraw is identical');
      assert.deepEqual(await page.evaluate(() => ({uploads:window.displayProbe.uploads,histories:window.displayProbe.histories,allocations:window.displayProbe.allocations})),before,'no paused history decay or texture allocations');
      await page.locator('#settings').click();
      await select('clean');
      await page.locator('#help-close').click();
      await page.locator('#pause').click();
    }
    // Same synthetic content, actual physical console models, different panels.
    await page.evaluate(() => { window.displayProbe.pattern='flat'; });
    const panels = [];
    for (const model of ['mono','color','crystal']) {
      await page.locator('#model').selectOption(model);
      await page.locator('#pause').click();
      await page.locator('#settings').click();
      await select('lcd-authentic');
      panels.push(await snapshot());
      await page.evaluate(() => { window.displayProbe.pattern='alternating'; window.displayProbe.histories=[]; });
      await select('lcd-authentic');
      await page.locator('#help-close').click();
      await page.locator('#pause').click();
      await page.waitForFunction(() => window.displayProbe.histories.length>=5);
      await page.locator('#pause').click();
      const history = await page.evaluate(() => window.displayProbe.histories);
      const weight = {mono:0.32,color:0.25,crystal:0.13}[model];
      for (let i=2;i<history.length;i++) {
        assert.ok(Math.abs(history[i].actual-(history[i].source*(1-weight)+history[i-1].actual*weight))<=1,`${model} panel response`);
      }
      await page.evaluate(() => { window.displayProbe.pattern='flat'; });
    }
    assert.notDeepEqual(panels[0],panels[1]);
    assert.notDeepEqual(panels[1],panels[2]);
    assert.ok(panels[0][1][1] > panels[0][1][2],'mono reflective tint');
    // A native 1× output suppresses grid detail rather than aliasing it.
    await page.setViewportSize({width:274,height:812});
    const small = await snapshot();
    assert.deepEqual(small[0],small[1],'grid fades away at native scale');
    await page.setViewportSize({width:375,height:812});
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth>innerWidth),false);
    await page.setViewportSize({width:1440,height:1000});

    // Successful replacement/restart/model changes seed, rather than blending old games.
    const replacement = await readFile(new URL('../../fixtures/generated/lcd-output.ws',import.meta.url));
    for (const action of ['replacement','model','restart','hidden']) {
      await page.evaluate(() => { window.displayProbe.pattern='alternating'; window.displayProbe.histories=[]; });
      if (action === 'replacement') {
        await page.locator('#rom').setInputFiles({name:'replacement.ws',mimeType:'application/octet-stream',buffer:replacement});
      } else if (action === 'model') await page.locator('#model').selectOption('color');
      else if (action === 'restart') await page.locator('#reset').click();
      else {
        await page.evaluate(() => {
          Object.defineProperty(document,'hidden',{configurable:true,value:true});
          document.dispatchEvent(new Event('visibilitychange'));
          delete document.hidden;
          document.dispatchEvent(new Event('visibilitychange'));
        });
        await page.locator('#pause').click();
      }
      await page.waitForFunction(() => window.displayProbe.histories.length>=3);
      assert.equal(await page.evaluate(() => window.displayProbe.histories[0].actual === window.displayProbe.histories[0].source),true,`${action} seeds history`);
      await page.locator('#pause').click();
    }
    const previous = await page.locator('#screen').evaluate(c => c.toDataURL());
    await page.locator('#rom').setInputFiles({name:'empty.ws',mimeType:'application/octet-stream',buffer:Buffer.alloc(0)});
    await page.waitForFunction(() => document.querySelector('#error').textContent.includes('Could not load'));
    assert.equal(await page.locator('#screen').evaluate(c => c.toDataURL()),previous,'failed load preserves image');
    const colorOnly = Buffer.from(replacement);
    colorOnly[colorOnly.length-9]=1;
    colorOnly.writeUInt16LE(colorOnly.subarray(0,-2).reduce((sum,b)=>(sum+b)&65535,0),colorOnly.length-2);
    await page.locator('#rom').setInputFiles({name:'color-only.ws',mimeType:'application/octet-stream',buffer:colorOnly});
    await page.waitForFunction(() => document.querySelector('#rom-status').textContent === 'Running');
    await page.locator('#pause').click();
    const retained = await snapshot();
    const uploads = await page.evaluate(() => window.displayProbe.uploads);
    await page.locator('#model').selectOption('mono');
    assert.match(await page.locator('#error').textContent(),/Could not restart/);
    assert.equal(await page.evaluate(() => window.displayProbe.uploads),uploads,'rejected model does not change history');
    assert.deepEqual(await snapshot(),retained,'rejected model preserves panel pixels');
    await page.locator('#settings').click();
    await select('clean');
    await page.locator('#help-close').click();
    await page.reload();
    await ready();
    assert.equal(await page.locator('#display-effect').inputValue(),'clean','explicit Clean persists');
    assert.equal(await actual(),'clean');
    await page.locator('#settings').click();
    await select('lcd-authentic');
    await page.locator('#help-close').click();
    await page.reload();
    await ready();
    assert.equal(await actual(),'lcd-authentic','LCD preference persists');

    // Real context loss/restoration, with a playable native canvas during loss.
    await page.locator('#demo').click();
    await page.waitForFunction(() => document.querySelector('#rom-status').textContent === 'Running');
    await page.evaluate(() => {
      const gl=document.querySelector('#screen-effect').getContext('webgl2');
      window.loseDisplay=gl.getExtension('WEBGL_lose_context');
      if (!window.loseDisplay) throw new Error('WEBGL_lose_context required for this test');
      window.loseDisplay.loseContext();
    });
    await page.waitForFunction(() => document.querySelector('#screen').dataset.effect === 'clean');
    assert.match(await page.locator('#display-notice').textContent(),/Clean fallback/);
    assert.equal(await page.locator('#rom-status').textContent(),'Running');
    assert.equal(await page.locator('#display-effect').inputValue(),'lcd-authentic','loss preserves choice');
    await page.evaluate(() => window.loseDisplay.restoreContext());
    await page.waitForFunction(() => document.querySelector('#screen').dataset.effect === 'lcd-authentic');
    assert.equal(await page.locator('#display-notice').textContent(),'');
  } finally { await context.close(); }

  // Faults share one short production page per independent boundary. These also
  // prove Clean remains functional when shader creation cannot succeed at all.
  for (const fault of ['unavailable','compile','link','draw','invalid-storage','denied-storage']) {
    const isolated = await browser.newContext();
    await isolated.addInitScript(fault => {
      if (fault === 'invalid-storage') localStorage.setItem('wonderswan:display:v1','invalid');
      if (fault === 'denied-storage') {
        Storage.prototype.getItem = () => { throw new Error('denied'); };
        Storage.prototype.setItem = () => { throw new Error('denied'); };
      }
      const get = HTMLCanvasElement.prototype.getContext;
      HTMLCanvasElement.prototype.getContext = function(type,...args) {
        if (type === 'webgl2' && fault === 'unavailable') return null;
        return get.call(this,type,...args);
      };
      if (fault === 'draw') WebGL2RenderingContext.prototype.drawArrays = function() { throw new Error('simulated GPU failure'); };
      if (fault === 'compile') WebGL2RenderingContext.prototype.compileShader = function() {};
      if (fault === 'link') WebGL2RenderingContext.prototype.linkProgram = function() {};
    },fault);
    const page = await isolated.newPage();
    page.on('pageerror', error => errors.push(error.message));
    try {
      await page.goto(base);
      await page.waitForFunction(() => !document.querySelector('#model').disabled);
      const fallback = ['unavailable','compile','link','draw'].includes(fault);
      assert.equal(await page.locator('#screen').getAttribute('data-effect'),fallback ? 'clean' : 'retro-crt');
      if (fallback) assert.match(await page.locator('#display-notice').textContent(),/Clean fallback/);
      await page.locator('#demo').click();
      await page.waitForFunction(() => {
        const ctx=document.querySelector('#screen').getContext('2d');
        return ctx.getImageData(27,65,1,1).data[0]===0 && document.querySelector('#rom-status').textContent === 'Running';
      });
      await page.locator('#settings').click();
      await page.locator('#display-effect').selectOption('clean');
      assert.equal(await page.locator('#screen').getAttribute('data-effect'),'clean');
    } finally { await isolated.close(); }
  }
  console.log('Display effects: real native-cell/softness/preset pixels, per-emulated-frame response, paused redraw/allocation stability, model tint, scaling, persistence, restart, invalid load, context restore and shader/storage fallback passed');
}
