// Focused queue regressions against the actual, unminified compiled Sound class.
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {resolve} from 'node:path';
import {parseArgs} from 'node:util';
import {readFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';

const {values}=parseArgs({options:{module:{type:'string'},'drift-output':{type:'string'}}});
if(!values.module)throw new Error('usage: audio-queue.mjs --module COMPILED_AUDIO_MJS [--drift-output JSON]');
const {Sound}=await import(pathToFileURL(resolve(values.module)));
class Node {
  connect(){} disconnect(){this.disconnected=true;}
}
class Context {
  constructor(){this.currentTime=1;this.state='suspended';this.nodes=[];Context.last=this;}
  resume(){this.state='running';return Promise.resolve();}
  suspend(){this.state='suspended';return Promise.resolve();}
  createBiquadFilter(){const n=new Node();n.frequency={value:0};return n;}
  createGain(){const n=new Node();n.gain={value:0,cancelScheduledValues(){},setValueAtTime(){},linearRampToValueAtTime(){}};return n;}
  createBuffer(channels,length,rate){
    const data=Array.from({length:channels},()=>new Float32Array(length));
    return {length,duration:length/rate,getChannelData:channel=>data[channel]};
  }
  createBufferSource(){
    const n=new Node(),context=this;
    n.start=function(when){
      this.when=when;this.end=when+this.buffer.duration;
      assert.equal(this.buffer.getChannelData(0)[0],.25);
      assert.equal(this.buffer.getChannelData(1)[0],-.25);
      if(context.onStart)context.onStart(this);else context.nodes.push(this);
    };
    n.stop=function(){if(!this.stopped)context.onStop?.(this);this.stopped=true;};return n;
  }
}
globalThis.AudioContext=Context;
globalThis.localStorage={getItem(){return null;},setItem(){}};
const sound=new Sound(()=>{});
sound.resume();sound.unlock();await Promise.resolve();
const context=Context.last,T=.01325,pcm=new Float32Array(636);
for(let i=0;i<pcm.length;i+=2){pcm[i]=.25;pcm[i+1]=-.25;}
// Two minutes of correctly paced PCM with no ended events delivered at all.
// Completion timestamps, rather than callback delivery, must bound occupancy.
let peak=0;
for(let i=0;i<Math.floor(120/T);i++){
  context.currentTime=1+i*T;sound.push(pcm,24000);
  peak=Math.max(peak,sound.timing.sources);
}
for(let i=1;i<context.nodes.length;i++)assert.equal(context.nodes[i].when,context.nodes[i-1].end,'adjacent packet boundary');
assert.equal(sound.timing.underruns,0);assert.equal(sound.timing.overruns,0);
assert.ok(peak<=10,'completed nodes must not exhaust the reserve');
assert.ok(context.nodes.slice(0,-10).every(n=>n.disconnected),'completed nodes reclaimed without ended tasks');
// A real starvation must recover once, with the configured bounded reserve.
context.currentTime+=.5;sound.push(pcm,24000);
assert.equal(sound.timing.underruns,1);
assert.ok(Math.abs(context.nodes.at(-1).when-context.currentTime-.1)<1e-9);
// Deliberate overproduction triggers explicit recovery and accepts the new PCM.
const before=context.nodes.length;
for(let i=0;i<100;i++)sound.push(pcm,24000);
assert.equal(context.nodes.length,before+100,'no silently rejected drained packet');
assert.ok(sound.timing.overruns>0);assert.ok(sound.timing.sources<=32);
assert.ok(sound.timing.queuedSeconds<=.25+T+1e-9);
// Lifecycle clears queued sound; old ended callbacks cannot resurrect it.
sound.pause();assert.equal(context.state,'suspended');assert.equal(sound.timing.sources,0);
assert.ok(context.nodes.every(n=>n.stopped && n.disconnected));
const paused=context.nodes.length;sound.push(pcm,24000);assert.equal(context.nodes.length,paused);
sound.resume();await Promise.resolve();sound.push(pcm,24000);
assert.equal(context.nodes.length,paused+1);
sound.setMuted(true);assert.equal(sound.timing.sources,0);assert.equal(context.state,'suspended');
console.log(`Actual Sound: 120 s contiguous PCM, deferred ended events (peak ${peak}), starvation/overflow recovery, pause/resume/mute passed`);

if(values['drift-output']){
  // Sensitivity experiment, not a measurement of any browser/device oscillator.
  // Production follows wall time; the mocked audio clock has the stated skew.
  const seconds=3600,rows=[];
  for(const ppm of [0,100,-100,1000,-1000]){
    const player=new Sound(()=>{});player.resume();player.unlock();await Promise.resolve();
    const clock=Context.last;
    let tail=null,gaps=0,gapSeconds=0,cancelledSeconds=0,packets=0,peak=0,wall=0,firstRecovery=null;
    clock.onStart=node=>{
      packets++;
      if(tail!==null&&node.when-tail>1e-8){gaps++;gapSeconds+=node.when-tail;}
      tail=node.end;
    };
    clock.onStop=node=>{
      const cancelled=Math.max(0,node.end-Math.max(node.when,clock.currentTime));
      cancelledSeconds+=cancelled;
      // Recovery flushes the entire queued tail, including a playing node.
      if(cancelled>0)tail=Math.min(tail??clock.currentTime,clock.currentTime);
    };
    for(let i=0;i<Math.floor(seconds/T);i++){
      wall=i*T;clock.currentTime=1+wall*(1+ppm/1e6);
      player.push(pcm,24000);const timing=player.timing;
      peak=Math.max(peak,timing.sources);
      if(firstRecovery===null&&(timing.underruns||timing.overruns))firstRecovery=wall;
    }
    rows.push({audio_clock_ppm:ppm,seconds,packets,peak_sources:peak,
      underruns:player.timing.underruns,overruns:player.timing.overruns,
      first_recovery_wall_seconds:firstRecovery,gap_count:gaps,gap_ms:gapSeconds*1000,
      cancelled_future_pcm_ms:cancelledSeconds*1000});
    clock.onStop=null;player.pause();
  }
  const zero=rows.find(r=>r.audio_clock_ppm===0);
  assert.equal(zero.underruns+zero.overruns+zero.gap_count,0);
  assert.ok(rows.find(r=>r.audio_clock_ppm===1000).underruns>0);
  assert.ok(rows.find(r=>r.audio_clock_ppm===-1000).overruns>0);
  assert.ok(rows.every(r=>r.peak_sources<=32));
  const sha=data=>createHash('sha256').update(data).digest('hex');
  await writeFile(values['drift-output'],JSON.stringify({project_revision:'979c75e',
    source_sha256:sha(await readFile(new URL('../../web/src/audio.ts',import.meta.url))),
    compiled_module_sha256:sha(await readFile(resolve(values.module))),
    model:'Actual compiled Sound; perfect 13.25 ms producer; independent mocked audio clock; ended events withheld; no CPU, device, render-quantum or measured oscillator model',
    rows},null,2)+'\n');
  for(const r of rows)console.log(`audio clock ${r.audio_clock_ppm} ppm: underruns=${r.underruns}, overruns=${r.overruns}, gaps=${r.gap_ms.toFixed(2)} ms, cancelled=${r.cancelled_future_pcm_ms.toFixed(2)} ms, first recovery=${r.first_recovery_wall_seconds}`);
}
