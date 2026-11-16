// Presentation only: the core's 224×144 RGBA image is never changed.
export type DisplayEffect = "clean" | "lcd-light" | "lcd-authentic" | "retro-crt";
const preferenceKey = "wonderswan:display:v1";
const effects: DisplayEffect[] = ["clean", "lcd-light", "lcd-authentic", "retro-crt"];
const width = 224, height = 144;

const vertex = `#version 300 es
void main() {
  vec2 p = vec2((gl_VertexID << 1) & 2, gl_VertexID & 2);
  gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}`;
const historyFragment = `#version 300 es
precision highp float;
uniform sampler2D sourceImage;
uniform sampler2D previousImage;
uniform float decay;
out vec4 color;
void main() {
  ivec2 p = ivec2(gl_FragCoord.xy);
  color = mix(texelFetch(sourceImage, p, 0), texelFetch(previousImage, p, 0), decay);
}`;
const panelFragment = `#version 300 es
precision highp float;
uniform sampler2D panelImage;
uniform vec2 outputSize;
uniform float grid;
uniform float softness;
uniform float strength;
uniform int model;
uniform bool crt;
out vec4 color;
vec4 samplePixel(ivec2 p) {
  return texelFetch(panelImage, clamp(p, ivec2(0), ivec2(223,143)), 0);
}
void main() {
  // Texture row zero is the source's top row; flip only at final presentation.
  vec2 native = vec2(gl_FragCoord.x, outputSize.y - gl_FragCoord.y)
    / outputSize * vec2(224.0,144.0);
  ivec2 p = ivec2(native);
  vec4 center = samplePixel(p);
  vec3 neighbors = (samplePixel(p+ivec2(1,0)).rgb + samplePixel(p-ivec2(1,0)).rgb
    + samplePixel(p+ivec2(0,1)).rgb + samplePixel(p-ivec2(0,1)).rgb) * 0.25;
  vec3 rgb = mix(center.rgb, neighbors, softness);
  // Fade subpixel detail out at small scales, avoiding matrix moiré at 1×.
  float detail = smoothstep(1.5, 3.0, min(outputSize.x/224.0, outputSize.y/144.0));
  vec2 cell = abs(fract(native) - 0.5) * 2.0;
  float edge = smoothstep(0.55, 1.0, max(cell.x, cell.y));
  if (crt) {
    float scan = 0.5 + 0.5*cos(native.y*6.2831853);
    float mask = 0.5 + 0.5*cos(native.x*6.2831853);
    rgb *= 1.0 - detail*(0.075*scan + 0.025*mask);
  } else {
    // Tasteful panel approximations, not measured colorimetry.
    vec3 panel;
    if (model == 0) panel = (rgb*0.91 + 0.035)*vec3(0.96,1.0,0.91);
    else if (model == 1) panel = rgb*0.92 + 0.025;
    else panel = rgb*0.98 + 0.008;
    rgb = mix(rgb, panel, strength);
    rgb *= 1.0 - grid*detail*edge;
  }
  color = vec4(rgb, center.a);
}`;

// Previous-frame contribution per *emulated* frame: mono, Color, SwanCrystal.
const response = {
  "lcd-light": [0.10, 0.08, 0.04],
  "lcd-authentic": [0.32, 0.25, 0.13],
};

class ShaderPanel {
  private source: WebGLTexture;
  private history: WebGLTexture[];
  private targets: WebGLFramebuffer[];
  private historyProgram: WebGLProgram;
  private panelProgram: WebGLProgram;
  private uniforms: Record<string, WebGLUniformLocation | null> = {};
  private current = 0;
  private seeded = false;

  constructor(private gl: WebGL2RenderingContext) {
    const program = (fragment: string) => {
      const result = gl.createProgram();
      if (!result) throw new Error("Could not create display program");
      for (const [type, text] of [[gl.VERTEX_SHADER, vertex], [gl.FRAGMENT_SHADER, fragment]] as const) {
        const shader = gl.createShader(type);
        if (!shader) throw new Error("Could not create display shader");
        gl.shaderSource(shader, text);
        gl.compileShader(shader);
        gl.attachShader(result, shader);
        gl.deleteShader(shader);
        if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error("Display shader compilation failed");
      }
      gl.linkProgram(result);
      if (!gl.getProgramParameter(result, gl.LINK_STATUS)) throw new Error("Display shader linking failed");
      return result;
    };
    this.historyProgram = program(historyFragment);
    this.panelProgram = program(panelFragment);
    for (const name of ["sourceImage", "previousImage", "decay"]) {
      this.uniforms[name] = gl.getUniformLocation(this.historyProgram, name);
    }
    for (const name of ["panelImage", "outputSize", "grid", "softness", "strength", "model", "crt"]) {
      this.uniforms[name] = gl.getUniformLocation(this.panelProgram, name);
    }
    const texture = () => {
      const value = gl.createTexture();
      if (!value) throw new Error("Could not create display texture");
      gl.bindTexture(gl.TEXTURE_2D, value);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, width, height, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
      return value;
    };
    this.source = texture();
    this.history = [texture(), texture()];
    this.targets = this.history.map(value => {
      const target = gl.createFramebuffer();
      if (!target) throw new Error("Could not create display history");
      gl.bindFramebuffer(gl.FRAMEBUFFER, target);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, value, 0);
      if (gl.checkFramebufferStatus(gl.FRAMEBUFFER) !== gl.FRAMEBUFFER_COMPLETE) throw new Error("Display history unavailable");
      return target;
    });
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    if (gl.getError() !== gl.NO_ERROR) throw new Error("Display resources unavailable");
  }

  reset() { this.seeded = false; }

  advance(pixels: Uint8ClampedArray, effect: DisplayEffect, model: number) {
    const gl = this.gl;
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.source);
    gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, width, height, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, this.history[this.current]);
    const next = 1 - this.current;
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.targets[next]);
    gl.viewport(0, 0, width, height);
    gl.useProgram(this.historyProgram);
    gl.uniform1i(this.uniforms.sourceImage, 0);
    gl.uniform1i(this.uniforms.previousImage, 1);
    const decay = effect === "lcd-light" || effect === "lcd-authentic" ? response[effect][model] : 0;
    gl.uniform1f(this.uniforms.decay, this.seeded ? decay : 0);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    this.current = next;
    this.seeded = true;
  }

  present(effect: DisplayEffect, model: number) {
    const gl = this.gl;
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.viewport(0, 0, gl.drawingBufferWidth, gl.drawingBufferHeight);
    gl.useProgram(this.panelProgram);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.history[this.current]);
    gl.uniform1i(this.uniforms.panelImage, 0);
    gl.uniform2f(this.uniforms.outputSize, gl.drawingBufferWidth, gl.drawingBufferHeight);
    const authentic = effect === "lcd-authentic";
    gl.uniform1f(this.uniforms.grid, authentic ? 0.20 : 0.09);
    gl.uniform1f(this.uniforms.softness, authentic ? 0.055 : 0.025);
    gl.uniform1f(this.uniforms.strength, authentic ? 1 : 0.35);
    gl.uniform1i(this.uniforms.model, model);
    gl.uniform1i(this.uniforms.crt, effect === "retro-crt" ? 1 : 0);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }
}

export class Display {
  effect: DisplayEffect = "lcd-light";
  private context: CanvasRenderingContext2D;
  private panel: ShaderPanel | null = null;
  private gl: WebGL2RenderingContext | null = null;
  private pixels: Uint8ClampedArray<ArrayBuffer> = new Uint8ClampedArray(width * height * 4);
  private model = 2;
  private scale = 1;

  constructor(private clean: HTMLCanvasElement, private output: HTMLCanvasElement,
    private notice: (message: string) => void) {
    const context = clean.getContext("2d");
    if (!context) throw new Error("This browser does not support a 2D canvas.");
    this.context = context;
    let saved = false;
    try {
      const value = localStorage.getItem(preferenceKey) as DisplayEffect;
      if (effects.includes(value)) { this.effect = value; saved = true; }
    } catch { /* Preferences must never prevent play. */ }
    this.initialize();
    if (!saved && !this.panel) this.effect = "clean";
    output.addEventListener("webglcontextlost", event => {
      event.preventDefault();
      this.fallback("Display graphics context lost. Clean fallback active.");
      this.present();
    });
    output.addEventListener("webglcontextrestored", () => {
      this.initialize();
      this.seed();
      this.present();
    });
  }

  private initialize() {
    try {
      this.gl = this.output.getContext("webgl2", { antialias: false, depth: false, stencil: false, premultipliedAlpha: false });
      if (!this.gl) throw new Error("WebGL2 unavailable");
      this.panel = new ShaderPanel(this.gl);
      this.notice("");
    } catch {
      this.fallback("Display effects unavailable. Clean fallback active.");
    }
  }

  private fallback(message: string) {
    this.panel = null;
    this.output.hidden = true;
    this.notice(message);
  }

  resetHistory() { this.panel?.reset(); }

  // Call once per emulated frame, including each frame in a catch-up tick.
  advance(pixels: Uint8ClampedArray<ArrayBuffer>, model: string) {
    this.pixels = pixels;
    this.model = model === "mono" ? 0 : model === "color" ? 1 : 2;
    if (this.effect !== "clean") this.seed();
  }

  private seed() {
    if (!this.panel || this.effect === "clean") return;
    try {
      this.panel.advance(this.pixels, this.effect, this.model);
      if (this.gl!.getError() !== this.gl!.NO_ERROR) throw new Error("Display upload failed");
    } catch { this.fallback("Display effects stopped. Clean fallback active."); }
  }

  select(effect: DisplayEffect) {
    if (!effects.includes(effect)) return;
    this.effect = effect;
    try { localStorage.setItem(preferenceKey, effect); } catch { /* Keep the live choice. */ }
    this.resetHistory();
    this.seed();
    this.present();
  }

  resize(scale: number) {
    this.clean.style.width = this.output.style.width = `${width * scale}px`;
    this.clean.style.height = this.output.style.height = `${height * scale}px`;
    // Limit output work to the screen and at most 4× native resolution. Use the
    // physical pixel scale so 1× CSS pixels on dense screens can still show cells.
    const physical = Math.max(1, Math.min(4, scale * (window.devicePixelRatio || 1)));
    if (physical !== this.scale) {
      this.scale = physical;
      this.output.width = Math.round(width * physical);
      this.output.height = Math.round(height * physical);
    }
    this.present();
  }

  // A redraw never advances history, emulator time, audio or input.
  present() {
    this.context.putImageData(new ImageData(this.pixels, width, height), 0, 0);
    if (this.panel && this.effect !== "clean") {
      try {
        this.panel.present(this.effect, this.model);
        if (this.gl!.getError() !== this.gl!.NO_ERROR) throw new Error("Display draw failed");
        this.output.hidden = false;
      } catch { this.fallback("Display effects stopped. Clean fallback active."); }
    } else this.output.hidden = true;
    this.clean.dataset.effect = this.output.hidden ? "clean" : this.effect;
  }
}
