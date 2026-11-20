// Focused queue regressions against the actual, unminified compiled Sound class.
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {resolve} from 'node:path';
import {parseArgs} from 'node:util';
import {readFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';

const {values}=parseArgs({options:{module:{type:'string'},'drift-output':{type:'string'},revision:{type:'string',default:'working-tree'}}});
if(!values.module)throw new Error('usage: audio-queue.mjs --module COMPILED_AUDIO_MJS [--drift-output JSON] [--revision LABEL]');
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
    let rate=1,started=false;
    n.playbackRate={get value(){return rate;},set value(value){
      assert.equal(started,false,'a scheduled packet must keep its chosen rate');rate=Math.fround(value);
    }};
    n.start=function(when){
      started=true;this.when=when;this.end=when+this.buffer.duration/rate;
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
assert.ok(Math.abs(context.nodes.at(-1).when-context.currentTime-.16)<1e-9);
assert.equal(sound.catchUpMs,160,'a real underrun increases the catch-up window');
// After its first underrun the same session can span repeated 120 ms losses
// of producer access, while each burst supplies every elapsed packet.
for(let burst=0;burst<30;burst++){
  context.currentTime+=10*T;
  for(let i=0;i<10;i++)sound.push(pcm,24000);
}
assert.equal(sound.timing.underruns,1,'enlarged reserve spans recurring stalls');
assert.equal(sound.timing.overruns,0,'recovery bursts remain within the queue bounds');
// Deliberate overproduction triggers explicit recovery and accepts the new PCM.
const before=context.nodes.length;
for(let i=0;i<100;i++)sound.push(pcm,24000);
assert.equal(context.nodes.length,before+100,'no silently rejected drained packet');
assert.ok(sound.timing.overruns>0);assert.ok(sound.timing.sources<=32);
assert.ok(sound.timing.queuedSeconds<=.25+T+1e-9);
// Lifecycle clears queued sound; old ended callbacks cannot resurrect it.
sound.pause();assert.equal(context.state,'suspended');assert.equal(sound.timing.sources,0);
assert.equal(sound.timing.playbackRate,1,'pause resets rate feedback');
assert.equal(sound.catchUpMs,100,'the next session starts with normal latency');
assert.ok(context.nodes.every(n=>n.stopped && n.disconnected));
const paused=context.nodes.length;sound.push(pcm,24000);assert.equal(context.nodes.length,paused);
sound.resume();await Promise.resolve();sound.push(pcm,24000);
assert.equal(context.nodes.length,paused+1);
sound.setMuted(true);assert.equal(sound.timing.sources,0);assert.equal(context.state,'suspended');
assert.equal(sound.timing.playbackRate,1,'mute resets rate feedback');
console.log(`Actual Sound: 120 s contiguous PCM, deferred ended events (peak ${peak}), starvation/overflow recovery, pause/resume/mute passed`);

if(values['drift-output']){
  // Sensitivity experiment, not a measurement of any browser/device oscillator.
  // Production follows wall time; the mocked audio clock has the stated skew.
  const seconds=3600,rows=[];
  const scenarios=[...[0,100,-100,1000,-1000].map(ppm=>({name:`constant-${ppm}`,ppm})),
    {name:'reverse-drift',ppm:1000,reverse:true},
    {name:'fast-with-stalls',ppm:1000,jitter:true},
    {name:'slow-with-stalls',ppm:-1000,jitter:true}];
  for(const scenario of scenarios){
    const {ppm}=scenario;
    const player=new Sound(()=>{});player.resume();player.unlock();await Promise.resolve();
    const clock=Context.last;
    let tail=null,gaps=0,gapSeconds=0,cancelledSeconds=0,packets=0,peak=0,wall=0,firstRecovery=null;
    let minRate=Infinity,maxRate=-Infinity,minLead=Infinity,maxLead=0,lastRate=1,maxRateStep=0,rateSum=0,rateCount=0;
    clock.onStart=node=>{
      packets++;
      const rate=node.playbackRate.value;
      minRate=Math.min(minRate,rate);maxRate=Math.max(maxRate,rate);
      maxRateStep=Math.max(maxRateStep,Math.abs(rate-lastRate));lastRate=rate;
      if(wall>seconds-60){rateSum+=rate;rateCount++;}
      minLead=Math.min(minLead,node.when-clock.currentTime);maxLead=Math.max(maxLead,node.end-clock.currentTime);
      if(tail!==null&&node.when-tail>1e-8){gaps++;gapSeconds+=node.when-tail;}
      tail=node.end;
    };
    clock.onStop=node=>{
      const cancelled=Math.max(0,node.end-Math.max(node.when,clock.currentTime));
      cancelledSeconds+=cancelled;
      // Recovery flushes the entire queued tail, including a playing node.
      if(cancelled>0)tail=Math.min(tail??clock.currentTime,clock.currentTime);
    };
    let generated=0;
    const count=Math.floor(seconds/T);
    for(let i=0;i<count;i++){
      // Seven packets arrive together every 38 packets, following an 80 ms
      // producer stall. All PCM is retained; no simulation of CPU work cost.
      if(scenario.jitter&&i%38<6)continue;
      wall=i*T;
      const first=Math.min(wall,seconds/2),second=Math.max(0,wall-seconds/2);
      clock.currentTime=1+first*(1+ppm/1e6)+second*(1+(scenario.reverse?-ppm:ppm)/1e6);
      while(generated<=i){player.push(pcm,24000);generated++;}
      const timing=player.timing;
      peak=Math.max(peak,timing.sources);
      if(firstRecovery===null&&(timing.underruns||timing.overruns))firstRecovery=wall;
    }
    rows.push({scenario:scenario.name,audio_clock_ppm:ppm,seconds,packets,peak_sources:peak,
      underruns:player.timing.underruns,overruns:player.timing.overruns,
      first_recovery_wall_seconds:firstRecovery,gap_count:gaps,gap_ms:gapSeconds*1000,
      cancelled_future_pcm_ms:cancelledSeconds*1000,min_rate:minRate,max_rate:maxRate,
      max_rate_step:maxRateStep,final_rate:player.timing.playbackRate,settled_mean_rate:rateSum/rateCount,
      min_start_lead_ms:minLead*1000,max_tail_lead_ms:maxLead*1000});
    clock.onStop=null;clock.onStart=null;player.pause();
    assert.equal(player.timing.playbackRate,1,'pause discards learned correction');
    player.resume();await Promise.resolve();player.push(pcm,24000);
    assert.equal(clock.nodes.at(-1).playbackRate.value,1,'resume starts with neutral rate');
    player.setMuted(true);assert.equal(player.timing.sources,0);
  }
  for(const row of rows){
    assert.equal(row.packets,Math.floor(row.seconds/T),'all PCM packets must be scheduled');
    assert.equal(row.underruns+row.overruns+row.gap_count,0,row.scenario);
    assert.equal(row.cancelled_future_pcm_ms,0,row.scenario);
    assert.ok(row.peak_sources<=32);
    assert.ok(row.min_rate>=Math.fround(.995)&&row.max_rate<=Math.fround(1.005),'bounded rate');
    assert.ok(row.max_rate_step<=.0005*T+2e-7,'bounded slew including float32 rounding');
    const expected=1/(1+(row.scenario==='reverse-drift'?-row.audio_clock_ppm:row.audio_clock_ppm)/1e6);
    assert.ok(Math.abs(row.settled_mean_rate-expected)<.0001,`${row.scenario}: learn clock ratio`);
  }
  const sha=data=>createHash('sha256').update(data).digest('hex');
  await writeFile(values['drift-output'],JSON.stringify({project_revision:values.revision,
    source_sha256:sha(await readFile(new URL('../../web/src/audio.ts',import.meta.url))),
    compiled_module_sha256:sha(await readFile(resolve(values.module))),
    model:'Actual compiled Sound with float32 fixed source rates; periodic/burst PCM producer and independent mocked audio clock; ended events withheld; no CPU, device, render-quantum or measured oscillator model',
    rows},null,2)+'\n');
  for(const r of rows)console.log(`${r.scenario}: underruns=${r.underruns}, overruns=${r.overruns}, gaps=${r.gap_ms.toFixed(2)} ms, cancelled=${r.cancelled_future_pcm_ms.toFixed(2)} ms, rate=${r.settled_mean_rate.toFixed(7)}`);
}
