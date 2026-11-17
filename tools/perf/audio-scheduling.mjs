// Replay the real compiled Sound class against a deterministic Web Audio clock.
// Diagnostic only: candidates are temporary module copies, not shipped changes.
import assert from 'node:assert/strict';
import {readFile, writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {parseArgs} from 'node:util';
const {values}=parseArgs({options:{module:{type:'string'},output:{type:'string'}}});
if(!values.module||!values.output) throw new Error('usage: audio-scheduling.mjs --module COMPILED_AUDIO_JS --output JSON');
const source=await readFile(resolve(values.module),'utf8');
const anchor='this.next = Math.max(this.next, context.currentTime + 0.025);';
assert.equal(source.split(anchor).length,2,'compile current audio.ts without minification');
assert.equal(source.split('this.sources.size >= 8').length,2,'source-count guard changed');
const variants=[
  {name:'current',code:source,cap:4},
  {name:'contiguous-25ms',code:source.replace(anchor,'if (this.next <= context.currentTime) this.next = context.currentTime + 0.025;'),cap:4},
  {name:'contiguous-75ms-cap8',code:source.replace(anchor,'if (this.next <= context.currentTime) this.next = context.currentTime + 0.075;'),cap:4},
  {name:'contiguous-75ms-cap16',code:source.replace(anchor,'if (this.next <= context.currentTime) this.next = context.currentTime + 0.075;').replace('this.sources.size >= 8','this.sources.size >= 16'),cap:4},
  {name:'contiguous-75ms-cap16-catchup8',code:source.replace(anchor,'if (this.next <= context.currentTime) this.next = context.currentTime + 0.075;').replace('this.sources.size >= 8','this.sources.size >= 16'),cap:8},
];
class Node {
  connect(){} disconnect(){}
}
class Context {
  constructor(){this.currentTime=0;this.state='suspended';this.nodes=[];this.undelivered=0;this.peak=0;}
  resume(){this.state='running';return Promise.resolve();}
  suspend(){this.state='suspended';return Promise.resolve();}
  close(){this.state='closed';return Promise.resolve();}
  createBiquadFilter(){const n=new Node();n.frequency={value:0};return n;}
  createGain(){const n=new Node();n.gain={value:0,cancelScheduledValues(){},setValueAtTime(){},linearRampToValueAtTime(){}};return n;}
  createBuffer(channels,length,rate){
    const data=Array.from({length:channels},()=>new Float32Array(length));
    return {duration:length/rate,length,sampleRate:rate,getChannelData:side=>data[side]};
  }
  createBufferSource(){
    const node=new Node(), context=this;
    node.start=function(when){
      assert.equal(this.buffer.length,318);assert.equal(this.buffer.getChannelData(0)[0],0.25);
      this.when=when;this.requestedAt=context.currentTime;this.duration=this.buffer.duration;
      context.nodes.push(this);context.undelivered++;context.peak=Math.max(context.peak,context.undelivered);
    };
    node.stop=function(){this.stopped=true;if(!this.delivered){this.delivered=true;context.undelivered--;}};
    return node;
  }
  ended(){
    for(const n of this.nodes) if(!n.delivered && n.when+n.duration<=this.currentTime+1e-10){
      n.delivered=true;this.undelivered--;n.onended?.();
    }
  }
}
globalThis.AudioContext=Context;
globalThis.localStorage={getItem(){return null;},setItem(){}};
const T=256*159/3072000, duration=20;
const scenarios=[
  {name:'60Hz',deltas:[1/60]}, {name:'30Hz',deltas:[1/30]},
  {name:'20Hz',deltas:[1/20]}, {name:'15Hz',deltas:[1/15]},
  {name:'10Hz',deltas:[1/10]},
  {name:'45ms-jitter',deltas:[...Array(14).fill(1/60),.045]},
  {name:'80ms-jitter',deltas:[...Array(29).fill(1/60),.080]},
  {name:'30Hz-ended-after-RAF',deltas:[1/30],endedAfter:true},
  {name:'80ms-jitter-ended-after-RAF',deltas:[...Array(29).fill(1/60),.080],endedAfter:true},
];
const rows=[];
for(const variant of variants){
  const url='data:text/javascript;base64,'+Buffer.from(variant.code).toString('base64');
  const {Sound}=await import(url);
  for(const scenario of scenarios){
    const sound=new Sound(()=>{});sound.resume();sound.unlock();await Promise.resolve();
    const context=sound.context;assert.equal(context.state,'running');
    const pcm=new Float32Array(318*2).fill(.25);
    let now=0,elapsed=0,callbacks=0,generated=0;
    while(now<duration-1e-9){
      const delta=Math.min(scenario.deltas[callbacks%scenario.deltas.length],duration-now);
      now+=delta;context.currentTime=now;
      if(!scenario.endedAfter)context.ended();
      elapsed+=Math.min(delta,.1);
      for(let k=0;k<variant.cap && elapsed+1e-10>=T;k++){
        sound.push(pcm,24000);generated++;elapsed-=T;
      }
      elapsed=Math.min(elapsed,T);
      if(scenario.endedAfter)context.ended();
      callbacks++;
    }
    let previous=null,gapMs=0,steadyGapMs=0,earlyGapMs=0,starvedMs=0,maxGapMs=0,gaps=0;
    for(const n of context.nodes){
      if(previous!==null){
        const gap=Math.max(0,n.when-previous);
        if(gap>1e-8){
          gaps++;gapMs+=gap*1000;maxGapMs=Math.max(maxGapMs,gap*1000);
          if(previous>1)steadyGapMs+=gap*1000;
          if(previous>n.requestedAt)earlyGapMs+=gap*1000;
          starvedMs+=Math.max(0,n.requestedAt-previous)*1000;
        }
      }
      previous=n.when+n.duration;
    }
    rows.push({variant:variant.name,scenario:scenario.name,seconds:duration,callbacks,
      emulated_fps:generated/duration,generated_frames:generated,scheduled_packets:context.nodes.length,
      dropped_packets:generated-context.nodes.length,catchup_lost_frames:Math.floor((duration+1e-8)/T)-generated,
      scheduled_pcm_seconds:context.nodes.length*T,source_peak:context.peak,
      initial_lead_ms:(context.nodes[0].when-context.nodes[0].requestedAt)*1000,
      gap_count:gaps,gap_ms:gapMs,steady_gap_ms:steadyGapMs,early_reanchor_gap_ms:earlyGapMs,
      already_starved_ms:starvedMs,max_gap_ms:maxGapMs});
    sound.pause();
  }
}
// Independent analytic sanity checks detect harness/clock mistakes.
const pick=(v,s)=>rows.find(r=>r.variant===v&&r.scenario===s);
assert.equal(pick('current','15Hz').emulated_fps,60);
assert.equal(pick('current','10Hz').emulated_fps,40);
assert.equal(pick('contiguous-25ms','30Hz').catchup_lost_frames,0);
assert.ok(pick('current','30Hz').early_reanchor_gap_ms>0);
assert.ok(pick('contiguous-25ms','30Hz').gap_ms<1e-6);
assert.equal(pick('contiguous-75ms-cap16-catchup8','15Hz').catchup_lost_frames,0);
await writeFile(values.output,JSON.stringify({baseline:'9edf5b6',seconds:duration,frame_seconds:T,
  model:'Actual compiled Sound class; deterministic clock and RAF arrivals; no audio device/CPU cost model',rows},null,2)+'\n');
for(const r of rows)console.log(`${r.variant.padEnd(34)} ${r.scenario.padEnd(23)} fps=${r.emulated_fps.toFixed(2)} lost=${r.catchup_lost_frames} drops=${r.dropped_packets} gaps=${r.gap_ms.toFixed(1)}ms steady=${r.steady_gap_ms.toFixed(1)}ms peak=${r.source_peak}`);
