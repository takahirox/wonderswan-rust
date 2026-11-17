// Headless Chromium measurement of the real Wasm adapter, including RGBA/PCM copies.
import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { parseArgs } from 'node:util';
import { chromium } from '../../web/node_modules/playwright/index.mjs';

const {values} = parseArgs({options: {
  baseline:{type:'string'}, optimized:{type:'string'}, browser:{type:'string'},
  output:{type:'string'}, frames:{type:'string',default:'1000'}, repeats:{type:'string',default:'5'},
}});
if (!values.baseline || !values.optimized || !values.output) {
  throw new Error('usage: wasm-throughput.mjs --baseline PKG --optimized PKG --output JSON [--browser EXECUTABLE] [--frames 1000] [--repeats 5]');
}
const frames=Number(values.frames), repeats=Number(values.repeats);
const replay=process.env.WS_PERF_INPUT_REPLAY !== undefined;
if (![frames,repeats].every(n=>Number.isInteger(n)&&n>0)) throw new Error('frames/repeats must be positive integers');
const packages=[];
for (const label of ['baseline','optimized']) {
  const dir=resolve(values[label]);
  packages.push({label,bindings:await readFile(`${dir}/ws_web.js`,'utf8'),wasm:[...await readFile(`${dir}/ws_web_bg.wasm`)]});
}
const root=new URL('../../',import.meta.url);
const roms=[];
for (const path of ['web/public/demo/beacon-trail.ws','fixtures/generated/sram128-2003.ws','fixtures/generated/sound-hyper.ws']) {
  roms.push({path,bytes:[...await readFile(new URL(path,root))]});
}
const browser=await chromium.launch({headless:true,...(values.browser ? {executablePath:values.browser} : {})});
try {
  const page=await browser.newPage();
  const samples=await page.evaluate(async ({packages,roms,frames,repeats,replay})=>{
    const runners=[];
    for (const pkg of packages) {
      const url=URL.createObjectURL(new Blob([pkg.bindings],{type:'text/javascript'}));
      try {
        const {default:init,WebMachine}=await import(url);
        await init({module_or_path:new Uint8Array(pkg.wasm)});
        runners.push({label:pkg.label,WebMachine});
      } finally {URL.revokeObjectURL(url);}
    }
    const hash=bytes=>{
      let h=0x811c9dc5;
      for (const b of bytes) h=Math.imul(h^b,0x1000193);
      return (h>>>0).toString(16).padStart(8,'0');
    };
    const rows=[];
    for (const rom of roms) {
      for (let trial=0;trial<repeats;trial++) {
        for (const runner of trial%2 ? [...runners].reverse() : runners) {
          const m=new runner.WebMachine();
          try {
            m.load_rom(new Uint8Array(rom.bytes));
            for (let f=0;f<120;f++) {m.run_frame();m.audio_samples();m.framebuffer();}
            let pcm,rgba,sampleValues=0,frameBytes=0;
            const start=performance.now();
            for (let f=0;f<frames;f++) {
              if (replay) m.set_buttons([32,64,128,16][Math.floor(f/64)%4]);
              m.run_frame();pcm=m.audio_samples();rgba=m.framebuffer();
              sampleValues+=pcm.length;frameBytes+=rgba.length;
            }
            const ms=performance.now()-start;
            if (m.frame_number()!==frames+120 || frameBytes!==frames*224*144*4 || !sampleValues) throw new Error('missing frame/audio work');
            rows.push({stage:runner.label,rom:rom.path,trial,frames,us_per_frame:ms*1000/frames,
              sample_values:sampleValues,frame_bytes:frameBytes,video_hash:hash(rgba),
              audio_hash:hash(new Uint8Array(pcm.buffer,pcm.byteOffset,pcm.byteLength))});
          } finally {m.free();}
        }
      }
    }
    return rows;
  },{packages,roms,frames,repeats,replay});
  const summary=[];
  for (const rom of roms) {
    const matching=samples.filter(s=>s.rom===rom.path);
    for (const key of ['video_hash','audio_hash','sample_values','frame_bytes']) {
      if (new Set(matching.map(s=>s[key])).size!==1) throw new Error(`Wasm output changed: ${rom.path}/${key}`);
    }
    for (const stage of ['baseline','optimized']) {
      const times=matching.filter(s=>s.stage===stage).map(s=>s.us_per_frame).sort((a,b)=>a-b);
      const median=(times[Math.floor((times.length-1)/2)]+times[Math.floor(times.length/2)])/2;
      summary.push({stage,rom:rom.path,median_us:median,min_us:times[0],max_us:times.at(-1)});
    }
  }
  await writeFile(values.output,JSON.stringify({browser:await browser.version(),frames,warmup:120,repeats,input_replay:replay,summary,samples},null,2)+'\n');
  for (const s of summary) console.log(`${s.stage}: ${s.rom}: ${s.median_us.toFixed(3)} us/frame`);
} finally {await browser.close();}
