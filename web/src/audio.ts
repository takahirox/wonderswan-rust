// Contiguous PCM with a bounded reserve for main-thread scheduling jitter.
const preferenceKey = "wonderswan:audio:v1";
const reserveSeconds = 0.1;
const rateLimit = 0.005;
const bound = (value: number, limit: number) => Math.max(-limit, Math.min(limit, value));
export class Sound {
  private context: AudioContext | null = null;
  private filter: BiquadFilterNode | null = null;
  private gain: GainNode | null = null;
  private sources = new Map<AudioBufferSourceNode, number>();
  private next = 0;
  private underruns = 0;
  private overruns = 0;
  private filteredLead = reserveSeconds;
  private rateBias = 0;
  private outputRate = 1;
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
  get timing() {
    return { underruns: this.underruns, overruns: this.overruns, sources: this.sources.size,
      queuedSeconds: Math.max(0, this.next - (this.context?.currentTime ?? 0)),
      playbackRate: this.outputRate };
  }
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
    for (const source of this.sources.keys()) {
      source.onended = null;
      try { source.stop(); } catch { /* Already ended. */ }
      source.disconnect();
    }
    this.sources.clear();
    this.next = 0;
    this.resetRate();
  }
  private resetRate() {
    this.filteredLead = reserveSeconds;
    this.rateBias = 0;
    this.outputRate = 1;
  }
  private adjustRate(lead: number, seconds: number) {
    // Two-second low-pass plus PI feedback in processed PCM time. Do not chase
    // individual timer callbacks or burst arrivals after a short main-thread stall.
    this.filteredLead += (lead - this.filteredLead) * seconds / (2 + seconds);
    const error = this.filteredLead - reserveSeconds;
    const requested = this.rateBias + 0.2 * error;
    // Do not wind the integral up against a saturated correction after a burst.
    if (Math.abs(requested) < rateLimit || requested * error < 0)
      this.rateBias = bound(this.rateBias + 0.01 * error * seconds, rateLimit);
    const wanted = 1 + bound(this.rateBias + 0.2 * error, rateLimit);
    // Slew by 500 ppm per PCM second, then float32 rounding, within +/-0.5%.
    // Each source keeps its chosen rate forever.
    this.outputRate += bound(wanted - this.outputRate, 0.0005 * seconds);
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
    const now = context.currentTime;
    // Audio completion precedes delivery of onended on a busy main thread.
    for (const [source, end] of this.sources) {
      if (end > now) continue;
      source.onended = null;
      try { source.stop(); } catch { /* Already ended. */ }
      source.disconnect();
      this.sources.delete(source);
    }
    // A clock/producer discontinuity must recover within bounded latency rather
    // than silently discard the incoming, already-drained PCM indefinitely.
    if (this.sources.size >= 32 || this.next > now + 0.25) {
      this.overruns++;
      this.clearQueue();
    }
    if (this.next <= now) {
      if (this.next !== 0) this.underruns++;
      this.next = now + reserveSeconds;
      this.resetRate();
    } else this.adjustRate(this.next - now, samples.length / 2 / rate);
    const length = samples.length / 2;
    const buffer = context.createBuffer(2, length, rate);
    for (let side = 0; side < 2; side++) {
      const output = buffer.getChannelData(side);
      for (let i = 0; i < length; i++) output[i] = samples[i * 2 + side];
    }
    const source = context.createBufferSource();
    source.buffer = buffer;
    source.playbackRate.value = this.outputRate;
    // AudioParam stores float32: use the accepted value, including rounding,
    // for every reservation/end timestamp, not the desired double precision rate.
    this.outputRate = source.playbackRate.value;
    const duration = length / rate / this.outputRate;
    source.connect(this.filter!);
    source.onended = () => { this.sources.delete(source); source.disconnect(); };
    this.sources.set(source, this.next + duration);
    source.start(this.next);
    this.next += duration;
  }
}
