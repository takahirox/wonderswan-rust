// Focused queue regressions against the actual, unminified compiled Sound class.
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {resolve} from 'node:path';
import {parseArgs} from 'node:util';

const {values}=parseArgs({options:{module:{type:'string'}}});
if(!values.module)throw new Error('usage: audio-queue.mjs --module COMPILED_AUDIO_MJS');
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
      context.nodes.push(this);
    };
    n.stop=function(){this.stopped=true;};return n;
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
