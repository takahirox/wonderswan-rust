import { defineConfig } from "vite";

export default defineConfig(({ command, isPreview }) => ({
  // Production and its local preview share the GitHub Pages repository subpath.
  base: command === "build" || isPreview ? "/wonderswan-rust/" : "/",
}));
