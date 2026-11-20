// Measure the production player with real Wasm and Web Audio under controlled load.
// Candidate edits are intercepted responses in an isolated browser, never disk edits.
import assert from 'node:assert/strict';
import {readFile, writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {resolve,basename} from 'node:path';
import {parseArgs} from 'node:util';
import {chromium} from '../../web/node_modules/playwright/index.mjs';

const {values}=parseArgs({options:{browser:{type:'string'},output:{type:'string'},
  url:{type:'string',default:'http://127.0.0.1:4173/wonderswan-rust/'},
  seconds:{type:'string',default:'8'},profiles:{type:'string'},check:{type:'boolean'},revision:{type:'string',default:'working-tree'},
  rom:{type:'string',default:'fixtures/generated/sram128-2003.ws'}}});
const seconds=Number(values.seconds);
if(!values.output || !Number.isFinite(seconds) || seconds<3) throw new Error('usage: audio-player.mjs --output JSON [--browser EXECUTABLE] [--seconds 8] [--profiles NAME,...]');
const profiles=[
  {name:'clean',effect:'clean'},
  {name:'lcd',effect:'lcd-light'},
  {name:'crt',effect:'retro-crt'},
  {name:'crt-10hz',effect:'retro-crt',hz:10},
  {name:'clean-cpu8',effect:'clean',cpu:8},
  {name:'lcd-cpu8',effect:'lcd-light',cpu:8},
  {name:'clean-15hz',effect:'clean',hz:15},
  {name:'clean-10hz',effect:'clean',hz:10},
  {name:'lcd-15hz',effect:'lcd-light',hz:15},
  {name:'clean-jitter80',effect:'clean',block:80},
  {name:'lcd-jitter80-cpu8',effect:'lcd-light',block:80,cpu:8},
  {name:'crt-jitter80-cpu8',effect:'retro-crt',block:80,cpu:8},
  {name:'crt-jitter120-cpu8',effect:'retro-crt',block:120,cpu:8,recovery:true},
  {name:'clean-rate-fast',effect:'clean',sampleRate:24024},
  {name:'clean-rate-slow',effect:'clean',sampleRate:23976},
  {name:'crt-rate-fast-jitter80-cpu8',effect:'retro-crt',sampleRate:24024,block:80,cpu:8},
  {name:'candidate-15hz',effect:'clean',hz:15,candidate:true},
  {name:'candidate-jitter80',effect:'clean',block:80,candidate:true},
];
const selected=values.profiles ? profiles.filter(p=>values.profiles.split(',').includes(p.name)) : profiles.filter(p=>!p.candidate&&!p.sampleRate&&!p.recovery);
assert.ok(selected.length,'no matching profiles');
if(values.profiles) assert.equal(selected.length,new Set(values.profiles.split(',')).size,'unknown profile');
const romPath=resolve(values.rom);
const rom=await readFile(romPath);
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
const browser=await chromium.launch({headless:true,...(values.browser?{executablePath:values.browser}:{})});
const rows=[];
const percentile=(a,p)=>{const s=[...a].sort((a,b)=>a-b);return s.length?s[Math.floor((s.length-1)*p)]:0;};
try {
  for(const profile of selected){
    const context=await browser.newContext({viewport:{width:1280,height:900}});
    const page=await context.newPage(),errors=[];
    page.on('pageerror',e=>errors.push(e.message));
    let bundleHash=null,patches=0;
    await page.route('**/assets/index-*.js',async route=>{
      const response=await route.fetch();let code=await response.text();bundleHash=hash(code);
      if(profile.candidate){
        for(const [from,to] of [
          ['this.next=Math.max(this.next,n.currentTime+.025)','(this.next<=n.currentTime&&(this.next=n.currentTime+.075))'],
          ['this.sources.size>=8','this.sources.size>=16'],
          ['for(;e<4;)','for(;e<8;)'],
        ]){
          assert.equal(code.split(from).length,2,`candidate anchor changed: ${from}`);
          code=code.replace(from,to);patches++;
        }
      }
      await route.fulfill({response,body:code});
    });
    await page.addInitScript(({profile})=>{
      localStorage.setItem('wonderswan:display:v1',profile.effect);
      const probe=window.audioTiming={enabled:false,packets:[],callbacks:[],wasm:[],values:0,active:0,peak:0,nonzero:0,cancelled:0,rateChanges:0,blocks:[],glMs:0,glCalls:0,uploads:0,draws:0,framebufferReads:0};
      const Native=window.AudioContext;
      window.AudioContext=class extends Native{
        constructor(...args){super(...args);window.measuredAudioContext=this;}
        createBufferSource(){
          const node=super.createBufferSource(),start=node.start.bind(node),stop=node.stop.bind(node);
          let active=false,scheduledAt=0,scheduledEnd=0,scheduledRate=1;
          const ended=()=>{if(active){active=false;probe.active--;}};
          node.addEventListener('ended',ended);
          node.addEventListener('ended',()=>{if(node.playbackRate.value!==scheduledRate)probe.rateChanges++;});
          node.stop=(...args)=>{
            if(probe.enabled&&active)probe.cancelled+=Math.max(0,scheduledEnd-Math.max(scheduledAt,this.currentTime));
            ended();return stop(...args);
          };
          node.start=(when,...args)=>{
            if(probe.enabled){
              probe.packets.push({at:this.currentTime,when,duration:node.buffer.duration/node.playbackRate.value,
                frames:node.buffer.length,rate:node.playbackRate.value,sample_rate:node.buffer.sampleRate});
              if(node.buffer.getChannelData(0).some(x=>x!==0))probe.nonzero++;
            }
            scheduledRate=node.playbackRate.value;
            scheduledAt=when;scheduledEnd=when+node.buffer.duration/scheduledRate;
            active=true;probe.active++;probe.peak=Math.max(probe.peak,probe.active);
            return start(when,...args);
          };
          return node;
        }
      };
      // Preserve the native Wasm functions/memory/tables, replacing only observed calls.
      const wrap=result=>{
        const exports={...result.instance.exports};
        for(const name of ['webmachine_run_frame','webmachine_audio_samples','webmachine_framebuffer','webmachine_sample_rate']){
          const original=exports[name];
          exports[name]=(...args)=>{
            const start=performance.now(),ret=original(...args);
            if(probe.enabled){
              if(name==='webmachine_run_frame')probe.wasm.push(performance.now()-start);
              if(name==='webmachine_audio_samples')probe.values+=ret[1];
              if(name==='webmachine_framebuffer'){
                probe.framebufferReads++;
                const pixels=new Uint8Array(exports.memory.buffer,ret[0],ret[1]);
                probe.lastPixels=[[0,0],[9,9],[16,16]].map(([x,y])=>Array.from(pixels.slice((y*224+x)*4,(y*224+x)*4+4)));
              }
            }
            return name==='webmachine_sample_rate'&&profile.sampleRate ? profile.sampleRate : ret;
          };
        }
        return {...result,instance:{exports}};
      };
      const streaming=WebAssembly.instantiateStreaming.bind(WebAssembly);
      WebAssembly.instantiateStreaming=async(...args)=>wrap(await streaming(...args));
      const instantiate=WebAssembly.instantiate.bind(WebAssembly);
      WebAssembly.instantiate=async(...args)=>{
        const result=await instantiate(...args);return result.instance?wrap(result):result;
      };
      for(const [name,count] of [['drawArrays','draws'],['texSubImage2D','uploads']]){
        const original=WebGL2RenderingContext.prototype[name];
        WebGL2RenderingContext.prototype[name]=function(...args){
          const start=performance.now(),ret=original.apply(this,args);
          if(probe.enabled){probe.glMs+=performance.now()-start;probe.glCalls++;probe[count]++;}
          return ret;
        };
      }
      // Gate owned RAF requests without blocking Web Audio's rendering thread.
      const raf=requestAnimationFrame.bind(window),cancel=cancelAnimationFrame.bind(window);
      const requests=new Map(),lastDelivered=new WeakMap();let id=0;
      window.requestAnimationFrame=callback=>{
        const key=++id;
        const deliver=time=>{
          if(!requests.has(key))return;
          const last=lastDelivered.get(callback)??-Infinity;
          if(profile.hz && time-last<1000/profile.hz-.5){requests.set(key,raf(deliver));return;}
          requests.delete(key);lastDelivered.set(callback,time);
          const start=performance.now(),frames=probe.wasm.length;
          callback(time);
          if(probe.enabled)probe.callbacks.push({time,ms:performance.now()-start,frames:probe.wasm.length-frames});
        };
        requests.set(key,raf(deliver));return key;
      };
      window.cancelAnimationFrame=key=>{cancel(requests.get(key));requests.delete(key);};
      window.beginAudioMeasurement=()=>{
        probe.enabled=true;probe.start=performance.now();probe.audioStart=window.measuredAudioContext.currentTime;
        if(profile.block)probe.blockTimer=setInterval(()=>{
          const start=performance.now(),until=start+profile.block;while(performance.now()<until){}
          probe.blocks.push(performance.now()-start);
        },500);
      };
      window.endAudioMeasurement=()=>{
        probe.enabled=false;clearInterval(probe.blockTimer);
        return {...probe,ms:performance.now()-probe.start,audioSeconds:window.measuredAudioContext.currentTime-probe.audioStart,
          contextState:window.measuredAudioContext.state,sampleRate:window.measuredAudioContext.sampleRate,
          baseLatency:window.measuredAudioContext.baseLatency,outputLatency:window.measuredAudioContext.outputLatency,
          display:document.querySelector('#display-effect').value,notice:document.querySelector('#display-notice').textContent};
      };
    },{profile});
    const cdp=await context.newCDPSession(page);
    page.setDefaultTimeout(20000);
    await cdp.send('Emulation.setCPUThrottlingRate',{rate:profile.cpu??1});
    await page.goto(values.url);
    await page.waitForFunction(()=>!document.querySelector('#model').disabled);
    await page.locator('#load-rom').click();
    await page.locator('#rom').setInputFiles({name:basename(romPath),mimeType:'application/octet-stream',buffer:rom});
    await page.waitForFunction(()=>document.querySelector('#rom-status').textContent==='Running');
    await page.waitForFunction(()=>window.audioTiming.active>0);
    await page.waitForTimeout(1000);
    await page.evaluate(()=>window.beginAudioMeasurement());
    await page.waitForTimeout(seconds*1000);
    const raw=await page.evaluate(()=>window.endAudioMeasurement());
    assert.equal(errors.length,0,errors.join('\n'));
    assert.equal(raw.contextState,'running');assert.equal(raw.display,profile.effect);
    assert.equal(raw.notice,'','requested display mode must not silently fall back');
    assert.ok(raw.nonzero>0 && raw.wasm.length>0 && raw.values>0,'missing real PCM/emulation');
    assert.deepEqual(raw.lastPixels[0],[0,255,0,255],'probe must render its green success signal');
    if(basename(romPath)==='sram128-2003.ws')assert.deepEqual(raw.lastPixels.slice(1),[[255,0,0,255],[0,0,255,255]],'missing color window/sprite');
    assert.equal(patches,profile.candidate?3:0);
    let previous=null,lastSubmission=null,gaps=0,gapMs=0,maxGap=0,starvedMs=0,earlyMs=0,maxSubmissionGap=0;
    const gapDetails=[];
    for(const packet of raw.packets){
      if(lastSubmission!==null)maxSubmissionGap=Math.max(maxSubmissionGap,(packet.at-lastSubmission)*1000);
      if(previous!==null){
        const gap=Math.max(0,packet.when-previous)*1000;
        if(gap>.01){gaps++;gapMs+=gap;maxGap=Math.max(maxGap,gap);
          if(gapDetails.length<8)gapDetails.push({gap_ms:gap,at_seconds:packet.at-raw.audioStart,submission_gap_ms:(packet.at-lastSubmission)*1000,
            prior_tail_lead_ms:(previous-lastSubmission)*1000});
          starvedMs+=Math.max(0,packet.at-previous)*1000;
          if(previous>packet.at)earlyMs+=gap;
        }
      }
      previous=packet.when+packet.duration;
      lastSubmission=packet.at;
    }
    const intervals=raw.callbacks.slice(1).map((r,i)=>r.time-raw.callbacks[i].time);
    const row={profile:profile.name,cpu_rate:profile.cpu??1,raf_limit:profile.hz??null,block_ms:profile.block??0,
      candidate:!!profile.candidate,bundle_sha256:bundleHash,seconds:raw.ms/1000,audio_seconds:raw.audioSeconds,
      injected_sample_rate:profile.sampleRate??null,
      display:raw.display,display_notice:raw.notice,context_sample_rate:raw.sampleRate,base_latency:raw.baseLatency,
      output_latency:raw.outputLatency,raf_hz:raw.callbacks.length*1000/raw.ms,frames:raw.wasm.length,
      probe_pixels:raw.lastPixels,
      emulated_fps:raw.wasm.length*1000/raw.ms,generated_pcm_seconds:raw.values/2/24000,
      scheduled_pcm_seconds:raw.packets.reduce((n,p)=>n+p.frames/24000,0),
      scheduled_playback_seconds:raw.packets.reduce((n,p)=>n+p.duration,0),
      min_playback_rate:raw.packets.reduce((v,p)=>Math.min(v,p.rate),Infinity),
      max_playback_rate:raw.packets.reduce((v,p)=>Math.max(v,p.rate),0),
      settled_mean_rate:raw.packets.slice(Math.floor(raw.packets.length*.75)).reduce((s,p)=>s+p.rate,0)/raw.packets.slice(Math.floor(raw.packets.length*.75)).length,
      rate_changes_after_start:raw.rateChanges,packets:raw.packets.length,nonzero_packets:raw.nonzero,
      max_submission_gap_ms:maxSubmissionGap,max_injected_block_ms:raw.blocks.reduce((v,n)=>Math.max(v,n),0),
      min_start_lead_ms:raw.packets.reduce((v,p)=>Math.min(v,(p.when-p.at)*1000),Infinity),
      gap_details:gapDetails,
      cancelled_pcm_seconds:raw.cancelled,
      source_peak:raw.peak,wasm_ms_p50:percentile(raw.wasm,.5),wasm_ms_p95:percentile(raw.wasm,.95),
      callback_ms_p50:percentile(raw.callbacks.map(r=>r.ms),.5),callback_ms_p95:percentile(raw.callbacks.map(r=>r.ms),.95),
      raf_interval_ms_p95:percentile(intervals,.95),gl_call_ms:raw.glMs,gl_calls:raw.glCalls,
      gl_uploads:raw.uploads,gl_draws:raw.draws,framebuffer_reads:raw.framebufferReads,
      gap_count:gaps,gap_ms:gapMs,max_gap_ms:maxGap,already_starved_ms:starvedMs,early_reanchor_gap_ms:earlyMs,errors};
    rows.push(row);
    await writeFile(values.output,JSON.stringify({revision:values.revision,browser:await browser.version(),
      rom:values.rom,rom_sha256:hash(rom),rows},null,2)+'\n');
    console.log(`${row.profile}: ${row.emulated_fps.toFixed(2)} fps; ${row.gap_ms.toFixed(1)} ms gaps; PCM ${row.scheduled_pcm_seconds.toFixed(3)}/${row.generated_pcm_seconds.toFixed(3)} s; callback p95 ${row.callback_ms_p95.toFixed(1)} ms`);
    if(values.check){
      if(profile.recovery){
        assert.ok(row.gap_count<=1,'at most one gap before the reserve adapts');
        assert.ok(row.gap_details.every(g=>g.at_seconds<2),'no gaps after the initial recovery');
      }else assert.equal(row.gap_count,0,'scheduled playback must remain contiguous');
      assert.ok(Math.abs(row.generated_pcm_seconds-row.scheduled_pcm_seconds)<1/24000,'drained PCM must not be dropped');
      assert.ok(row.cancelled_pcm_seconds<1/24000,'future PCM must not be cancelled during playback');
      assert.equal(row.rate_changes_after_start,0,'scheduled sources must keep their fixed rates');
      assert.ok(row.min_playback_rate>=Math.fround(.995)&&row.max_playback_rate<=Math.fround(1.005),'bounded output correction');
      if(profile.sampleRate&&seconds>=60)assert.ok(Math.abs(row.settled_mean_rate-24000/profile.sampleRate)<.0005,'learn injected rate mismatch');
      assert.ok(Math.abs(row.generated_pcm_seconds-row.seconds)<(profile.recovery?.17:.12),'PCM supply must track real time, allowing the final stall/packet phase');
      assert.ok(row.source_peak<=32,'source resources must remain bounded');
    }
    await context.close();
  }
} finally {await browser.close();}
