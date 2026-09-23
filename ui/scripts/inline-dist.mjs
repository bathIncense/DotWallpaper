import { readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const dist = resolve(here, "../dist");
let html = await readFile(resolve(dist, "index.html"), "utf8");

const css = html.match(/<link[^>]+href=["']([^"']+\.css)["'][^>]*>/i);
if (css) {
  const cssPath = resolve(dist, css[1]);
  const cssText = await readFile(cssPath, "utf8");
  html = html.replace(css[0], () => `<style data-vite-inline>${cssText}</style>`);
}

const js = html.match(/<script[^>]+src=["']([^"']+\.js)["'][^>]*><\/script>/i);
if (js) {
  const jsPath = resolve(dist, js[1]);
  let jsText = await readFile(jsPath, "utf8");
  // Prevent a literal </script> in compiled template strings from ending the inline element.
  jsText = jsText.replaceAll("</script", "<\\/script");
  // The Vite production bundle is self-contained after bundling. Remove the
  // module attribute so WKWebView can execute it from a file:// document
  // without a module-CORS request for a sibling resource.
  html = html.replace(js[0], () => `<script data-vite-inline>${jsText}</script>`);
}

await writeFile(resolve(dist, "index.html"), html);
console.log("Inlined Vite assets into dist/index.html for WKWebView file:// loading");
