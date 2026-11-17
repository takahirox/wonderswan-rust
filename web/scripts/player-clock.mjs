// SPDX-License-Identifier: CC0-1.0
// Control producer timers, performance time and presentation separately in the
// original CPU fixtures. Browser I/O and the real Wasm adapter remain unchanged.
export function installPlayerClock() {
  let now=0,origin=0,next=0,anchor=true,stepping=false;
  const timers=new Map(),frames=new Map();
  const timeout=window.setTimeout.bind(window),clear=window.clearTimeout.bind(window);
  Object.defineProperty(performance,'now',{configurable:true,value:()=>now});
  window.setTimeout=(callback,delay=0,...args)=>{
    if(typeof callback!=='function'||delay>8)return timeout(callback,delay,...args);
    const id=--next;timers.set(id,()=>callback(...args));
    if(!stepping)anchor=true; // A new production chain after load/resume.
    return id;
  };
  window.clearTimeout=id=>{if(!timers.delete(id))clear(id);};
  window.requestAnimationFrame=callback=>{const id=--next;frames.set(id,callback);return id;};
  window.cancelAnimationFrame=id=>frames.delete(id);
  window.playerClock={timers,frames,step(time){
    if(anchor&&timers.size){origin=now-time;anchor=false;}
    now=origin+time;stepping=true;
    try {
      const due=[...timers.values()];timers.clear();due.forEach(callback=>callback());
      const paint=[...frames.values()];frames.clear();paint.forEach(callback=>callback(now));
    } finally {stepping=false;}
  }};
}
export function stepPlayerClock(time) {window.playerClock.step(time);}
