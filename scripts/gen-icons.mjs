#!/usr/bin/env node
/**
 * HerbReady — gen-icons.mjs
 *
 * Generate platform icons for Tauri from a master SVG.
 *
 * - Reads:  scripts/herbready-master.svg
 * - Writes: src-tauri/icons/*
 *
 * Primary libraries used:
 *  - @svg2img   (rasterize SVG -> PNG buffer)
 *  - sharp      (image resize/encode, write PNG)
 *
 * Optional libs (if installed) to produce container formats:
 *  - png-to-ico  -> creates Windows .ico from PNG buffers
 *  - icon-gen    -> creates macOS .icns (or you can use iconutil on macOS)
 *
 * Usage:
 *   node scripts/gen-icons.mjs
 *
 * Notes:
 *  - The script will try to produce PNGs for the common sizes Tauri expects,
 *    then attempt to create icon.ico and icon.icns if the helper libs are installed.
 *  - If those helper libs are absent, the script still produces the PNG set
 *    which is sufficient for many build pipelines (or you can convert them later).
 */

import fs from "fs/promises";
import path from "path";
import process from "process";

const ROOT = process.cwd();
const MASTER_SVG = path.join(ROOT, "scripts", "herbready-master.svg");
const OUT_DIR = path.join(ROOT, "src-tauri", "icons");

async function ensureDir(dir) {
  try {
    await fs.mkdir(dir, { recursive: true });
  } catch (err) {
    // ignore if exists
  }
}

function log(...args) {
  console.log("[gen-icons]", ...args);
}

async function rasterizeSvgToPngBuffer(svgBuffer, size, svg2imgModule) {
  // @svg2img historically uses callback API: svg2img(svg, opts, cb)
  // Wrap it into a Promise so we can await.
  return new Promise((resolve, reject) => {
    try {
      // some module builds export default, some export function directly
      const svg2img = svg2imgModule && (svg2imgModule.default || svg2imgModule);
      svg2img(
        svgBuffer,
        {
          width: size,
          height: size,
          format: "png",
          // preserve aspect; our art is square/centered via viewBox
        },
        (err, buffer) => {
          if (err) return reject(err);
          resolve(buffer);
        },
      );
    } catch (err) {
      reject(err);
    }
  });
}

async function writePngUsingSharp(pngBuffer, outPath) {
  // Use sharp for canonical output (strip metadata, set proper PNG params)
  const sharp = await import("sharp").then((m) => m.default || m);
  await sharp(pngBuffer).png({ compressionLevel: 9 }).toFile(outPath);
}

function sizesForTauri() {
  // Common sizes to produce plus the special @2x
  return [
    16,
    32,
    44,
    64,
    71,
    89,
    107,
    128,
    150,
    256,
    284,
    310,
    512, // icon.png / large
  ];
}

function tauriNamedFilesFromSizes(map) {
  // Return a set of filenames Tauri config commonly references.
  // We will also write several generic `<w>x<h>.png` files for convenience.
  return {
    "32x32.png": map[32],
    "128x128.png": map[128],
    "128x128@2x.png": map[256] || map[512], // 2x of 128 is 256
    "icon.png": map[512] || map[256],
  };
}

async function tryCreateIco(pngBuffersBySize) {
  // png-to-ico expects buffers for various sizes (16, 24, 32, 48, 64, 128, 256)
  try {
    const pngToIco = await import("png-to-ico").then((m) => m.default || m);
    const want = [16, 32, 48, 64, 128, 256].filter((s) => pngBuffersBySize[s]);
    if (want.length === 0) {
      log("No suitable PNG sizes available for .ico generation, skipping .ico");
      return null;
    }
    const bufs = want.map((s) => pngBuffersBySize[s]);
    const icoBuffer = await pngToIco(bufs);
    return icoBuffer;
  } catch (err) {
    log("png-to-ico not available or failed; skipping .ico generation.");
    return null;
  }
}

async function tryCreateIcns(tempPngPaths) {
  // Attempt to use icon-gen if available (preferred cross-platform)
  try {
    const iconGen = await import("icon-gen").then((m) => m.default || m);
    // icon-gen writes files to disk, so we will create a temp output dir
    const tmpOut = path.join(OUT_DIR, "tmp_icongen_out");
    await ensureDir(tmpOut);
    await iconGen(tmpOut, {
      report: false,
      icns: { name: "icon", sizes: [16, 32, 64, 128, 256, 512, 1024] },
      modes: ["icns"],
      // Provide source PNG(s) — icon-gen can find best source(s) inside tmpOut,
      // but library accepts different signatures per version. The simplest route:
      // write the PNGs into a temporary folder and ask icon-gen to generate.
    });
    // icon-gen's APIs vary; if the import worked but invocation didn't produce an icns
    // we'll skip fallback. Caller can use system iconutil or other CLI if needed.
    // Read icon.icns if produced
    const candidate = path.join(tmpOut, "icon.icns");
    try {
      const icnsBuf = await fs.readFile(candidate);
      // cleanup tmp folder
      await fs.rm(tmpOut, { recursive: true, force: true });
      return icnsBuf;
    } catch {
      // nothing produced
      await fs.rm(tmpOut, { recursive: true, force: true });
      return null;
    }
  } catch (err) {
    log("icon-gen not available or failed; skipping .icns generation.");
    return null;
  }
}

async function main() {
  log("Starting icon generation...");

  // 1) Validate master SVG exists
  try {
    await fs.access(MASTER_SVG);
  } catch (err) {
    console.error(`Master SVG not found at ${MASTER_SVG}`);
    process.exit(1);
  }

  // 2) Load SVG
  const svgBuffer = await fs.readFile(MASTER_SVG);

  // 3) Ensure output dir exists
  await ensureDir(OUT_DIR);

  // 4) Load svg2img dynamically and sharp is used later for writing
  let svg2imgModule;
  try {
    svg2imgModule = await import("svg2img").then((m) => m.default || m);
  } catch (err) {
    console.error(
      "ERROR: svg2img not installed. Install with `npm install svg2img --save-dev` (or pnpm/yarn).",
    );
    process.exit(2);
  }

  // 5) Build raster buffers for needed sizes
  const sizes = sizesForTauri();
  const pngBuffersBySize = {};
  for (const size of sizes) {
    try {
      log(`Rasterizing ${size}x${size} ...`);
      const buf = await rasterizeSvgToPngBuffer(svgBuffer, size, svg2imgModule);
      // normalize/encode with sharp to ensure consistent output
      // sharp will accept a PNG buffer and we can re-encode to PNG canonical
      const sharpM = await import("sharp").then((m) => m.default || m);
      const canonicalBuf = await sharpM(buf)
        .png({ compressionLevel: 9 })
        .toBuffer();
      pngBuffersBySize[size] = canonicalBuf;
    } catch (err) {
      console.warn(`Failed to rasterize size ${size}:`, err.message || err);
    }
  }

  // 6) Write PNG files (both named sizes and Tauri-specific names)
  // Write explicit WxH files
  for (const [sizeStr, buf] of Object.entries(pngBuffersBySize)) {
    const size = Number(sizeStr);
    const name = `${size}x${size}.png`;
    const outPath = path.join(OUT_DIR, name);
    await fs.writeFile(outPath, buf);
    log("Wrote", name);
  }

  // Tauri canonical names
  const mapping = tauriNamedFilesFromSizes(
    Object.fromEntries(
      Object.entries(pngBuffersBySize).map(([k, v]) => [Number(k), v]),
    ),
  );

  for (const [fileName, buf] of Object.entries(mapping)) {
    if (!buf) continue;
    const outPath = path.join(OUT_DIR, fileName);
    await fs.writeFile(outPath, buf);
    log("Wrote", fileName);
  }

  // 7) Try build .ico
  const icoBuffer = await tryCreateIco(pngBuffersBySize);
  if (icoBuffer) {
    const icoPath = path.join(OUT_DIR, "icon.ico");
    await fs.writeFile(icoPath, icoBuffer);
    log("Wrote icon.ico");
  } else {
    log("icon.ico not generated (optional).");
  }

  // 8) Try build .icns (best-effort)
  // We will attempt to use 'png-to-icns' like libraries if available; fallback: skip.
  try {
    // Attempt a simple approach: if 'png2icons' is installed, use it
    const png2icons = await import("png2icons")
      .then((m) => m.default || m)
      .catch(() => null);
    if (png2icons) {
      // png2icons.exportICNS(srcBuffer, png2icons.BICUBIC, png2icons.PNG); // API varies
      // We'll try common invocation patterns; wrap in try/catch
      try {
        const srcBuf = pngBuffersBySize[512] || pngBuffersBySize[256];
        if (srcBuf) {
          const icnsBuf = png2icons.createICNS(srcBuf, png2icons.ICNS_BEST);
          if (icnsBuf) {
            await fs.writeFile(path.join(OUT_DIR, "icon.icns"), icnsBuf);
            log("Wrote icon.icns (png2icons)");
          }
        }
      } catch (err) {
        log("png2icons present but ICNS creation failed, skipping .icns");
      }
    } else {
      // try icon-gen approach
      const icnsBuf = await tryCreateIcns(
        Object.keys(pngBuffersBySize).map((k) => pngBuffersBySize[k]),
      );
      if (icnsBuf) {
        await fs.writeFile(path.join(OUT_DIR, "icon.icns"), icnsBuf);
        log("Wrote icon.icns (icon-gen)");
      } else {
        log("icon.icns not generated (optional).");
      }
    }
  } catch (err) {
    log(
      "ICNS generation step encountered an error, skipped. Error:",
      err.message || err,
    );
  }

  // 9) Final message
  log("Icon generation complete. Output directory:", OUT_DIR);
  log(
    "Produced PNGs:",
    Object.keys(pngBuffersBySize)
      .map((s) => `${s}x${s}.png`)
      .join(", "),
  );
  log("If .ico/.icns are missing: install optional helpers:");
  log(" - For .ico: `npm i -D png-to-ico`");
  log(
    " - For .icns: try `npm i -D png2icons icon-gen` (platform/tooling dependent)",
  );
  log(
    "You can now run your Tauri build; the bundler will pick icons from src-tauri/icons as configured in tauri.conf.json",
  );
}

// Run
main().catch((err) => {
  console.error("gen-icons failed:", err);
  process.exit(10);
});
