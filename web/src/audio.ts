// A bounded queue of Web Audio buffers. No independent emulation/audio timer.
export class Sound {
  private context: AudioContext | null = null;
  private filter: BiquadFilterNode | null = null;
  private sources = new Set<AudioBufferSourceNode>();
  private next = 0;
  enabled = false;
  constructor(private report: (message: string) => void) {}
  async enable(): Promise<boolean> {
    try {
      if (this.context?.state === "closed") { this.context = null; this.filter = null; }
      if (!this.context) {
        const context = new AudioContext();
        try {
          const filter = context.createBiquadFilter();
          filter.type = "highpass";
          filter.frequency.value = 20; // Hardware samples are unsigned; remove DC.
          filter.connect(context.destination);
          this.context = context;
          this.filter = filter;
        } catch (error) {
          void context.close().catch(() => {});
          throw error;
        }
      }
      await this.context.resume();
      this.enabled = true;
      return true;
    } catch {
      this.enabled = false;
      this.report("Sound is unavailable in this browser. You can keep playing.");
      return false;
    }
  }
  pause() {
    for (const source of this.sources) {
      source.onended = null;
      try { source.stop(); } catch { /* Already ended. */ }
      source.disconnect();
    }
    this.sources.clear();
    this.next = 0;
    if (this.context?.state === "running") void this.context.suspend().catch(() => {});
  }
  resume() {
    if (this.enabled) void this.context?.resume().catch(() => {
      this.disable();
      this.report("Sound could not resume. Toggle sound to try again.");
    });
  }
  disable() { this.enabled = false; this.pause(); }
  push(samples: Float32Array, rate: number) {
    try { this.enqueue(samples, rate); }
    catch {
      this.disable();
      this.report("Sound playback became unavailable. You can keep playing.");
    }
  }
  private enqueue(samples: Float32Array, rate: number) {
    const context = this.context;
    if (!this.enabled || !context || context.state !== "running" || !samples.length) return;
    if (this.sources.size >= 8 || this.next > context.currentTime + 0.15) return;
    const length = samples.length / 2;
    const buffer = context.createBuffer(2, length, rate);
    for (let side = 0; side < 2; side++) {
      const output = buffer.getChannelData(side);
      for (let i = 0; i < length; i++) output[i] = samples[i * 2 + side];
    }
    const source = context.createBufferSource();
    source.buffer = buffer;
    source.connect(this.filter!);
    source.onended = () => { this.sources.delete(source); source.disconnect(); };
    this.sources.add(source);
    this.next = Math.max(this.next, context.currentTime + 0.025);
    source.start(this.next);
    this.next += length / rate;
  }
}
