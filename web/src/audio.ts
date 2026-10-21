// A bounded queue of Web Audio buffers. No independent emulation/audio timer.
const preferenceKey = "wonderswan:audio:v1";
export class Sound {
  private context: AudioContext | null = null;
  private filter: BiquadFilterNode | null = null;
  private gain: GainNode | null = null;
  private sources = new Set<AudioBufferSourceNode>();
  private next = 0;
  private playing = false;
  private attempt = 0;
  private failure = "";
  private level = 70;
  private silent = false;
  constructor(private changed: () => void) {
    try {
      const stored = JSON.parse(localStorage.getItem(preferenceKey) ?? "null");
      if (typeof stored?.muted === "boolean") this.silent = stored.muted;
      if (typeof stored?.volume === "number" && Number.isFinite(stored.volume)
        && stored.volume >= 0 && stored.volume <= 100) this.level = Math.round(stored.volume);
    } catch { /* Storage is optional; first-visit defaults are safe. */ }
  }
  get muted() { return this.silent; }
  get volume() { return this.level; }
  get needsUnlock() {
    return !this.muted && (this.failure !== "" || (this.playing && this.context?.state !== "running"));
  }
  get status() {
    const preference = this.muted ? "Muted" : this.volume === 0 ? "Volume 0%" : "Sound on";
    let output = "Audio paused.";
    if (this.failure) output = `${this.failure} ${this.muted ? "Unmute" : "Tap Enable audio"} to retry; you can keep playing.`;
    else if (!this.context) output = this.muted ? "Audio not started." : "Start a game to enable audio.";
    else if (this.context.state === "running") output = "Audio ready; cartridges may be silent.";
    else if (this.playing && !this.muted) output = "Audio suspended. Tap Enable audio to retry.";
    return `${preference} · ${output}`;
  }
  setVolume(value: number) {
    this.level = Number.isFinite(value) ? Math.round(Math.max(0, Math.min(100, value))) : 70;
    this.applyGain();
    this.store();
    this.changed();
  }
  setMuted(value: boolean) {
    this.silent = value;
    this.applyGain();
    if (value) this.stop();
    this.store();
    this.changed();
  }
  private store() {
    try { localStorage.setItem(preferenceKey, JSON.stringify({ muted: this.muted, volume: this.volume })); }
    catch { /* Retain this session's preferences if storage is denied/full. */ }
  }
  private applyGain() {
    if (!this.gain || !this.context) return;
    const parameter = this.gain.gain, now = this.context.currentTime;
    parameter.cancelScheduledValues(now);
    parameter.setValueAtTime(parameter.value, now);
    // Mute is immediate; volume changes ramp briefly to avoid clicks.
    if (this.muted) parameter.setValueAtTime(0, now);
    else parameter.linearRampToValueAtTime(this.volume / 100, now + 0.01);
  }
  // Invoke directly from a user event, before fetching/reading a cartridge.
  unlock() {
    if (this.muted) return;
    this.failure = "";
    try {
      if (this.context?.state === "closed") { this.context = null; this.filter = null; this.gain = null; }
      if (!this.context) {
        const context = new AudioContext();
        try {
          const filter = context.createBiquadFilter();
          filter.type = "highpass";
          filter.frequency.value = 20; // Hardware samples are unsigned; remove DC.
          const gain = context.createGain();
          gain.gain.value = this.muted ? 0 : this.volume / 100;
          filter.connect(gain);
          gain.connect(context.destination);
          this.context = context;
          this.filter = filter;
          this.gain = gain;
          context.onstatechange = () => {
            if (this.context !== context) return;
            if (context.state !== "running") this.clearQueue();
            this.changed();
          };
        } catch (error) {
          void context.close().catch(() => {});
          throw error;
        }
      }
      void this.start();
    } catch {
      this.failure = "Audio unavailable.";
      this.changed();
    }
  }
  private async start() {
    const attempt = ++this.attempt;
    try {
      // The call itself must be synchronous with unlock's initiating gesture.
      const pending = this.context!.resume();
      this.changed(); // A pending or resolved-but-suspended resume is not success.
      await pending;
    } catch {
      if (attempt === this.attempt) this.failure = "Audio could not resume.";
    } finally {
      // A late unlock must not resume a paused, hidden, or newly muted game.
      if (!this.playing || this.muted || this.failure) this.stop();
      this.changed();
    }
  }
  private clearQueue() {
    for (const source of this.sources) {
      source.onended = null;
      try { source.stop(); } catch { /* Already ended. */ }
      source.disconnect();
    }
    this.sources.clear();
    this.next = 0;
  }
  private stop() {
    this.clearQueue();
    if (this.context?.state === "running") void this.context.suspend().catch(() => {});
  }
  pause() { this.playing = false; this.stop(); this.changed(); }
  resume() {
    this.playing = true;
    // Automatic lifecycle resumes reuse a context; only gestures create one.
    if (!this.muted && !this.failure && this.context) void this.start();
    this.changed();
  }
  push(samples: Float32Array, rate: number) {
    try { this.enqueue(samples, rate); }
    catch {
      this.failure = "Audio playback unavailable.";
      this.stop();
      this.changed();
    }
  }
  private enqueue(samples: Float32Array, rate: number) {
    const context = this.context;
    if (!this.playing || this.muted || this.failure || !context || context.state !== "running" || !samples.length) return;
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
