/**
 * Generates the monochrome application icons used by Tauri bundling.
 * Run with `node scripts/make-icons.mjs` after changing the design.
 */
import { deflateSync } from "node:zlib";
import { mkdirSync, writeFileSync } from "node:fs";

const SIZE = 512;

function crc32(buffer) {
  let crc = 0xffffffff;
  for (const byte of buffer) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = crc & 1 ? (crc >>> 1) ^ 0xedb88320 : crc >>> 1;
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([length, body, crc]);
}

function png(width, height, rgba) {
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header[8] = 8;
  header[9] = 6;
  const raw = Buffer.alloc((width * 4 + 1) * height);
  for (let y = 0; y < height; y += 1) {
    raw[y * (width * 4 + 1)] = 0;
    rgba.copy(raw, y * (width * 4 + 1) + 1, y * width * 4, (y + 1) * width * 4);
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", header),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

function roundedRectDistance(x, y, left, top, right, bottom, radius) {
  const cx = Math.max(left + radius, Math.min(x, right - radius));
  const cy = Math.max(top + radius, Math.min(y, bottom - radius));
  return Math.hypot(x - cx, y - cy) - radius;
}

/** Renders the icon at full size: black canvas, thin white frame, dispatch bars. */
function renderBase() {
  const buffer = Buffer.alloc(SIZE * SIZE * 4);
  const paint = (x, y, value, alpha = 255) => {
    const index = (y * SIZE + x) * 4;
    buffer[index] = value;
    buffer[index + 1] = value;
    buffer[index + 2] = value;
    buffer[index + 3] = alpha;
  };

  for (let y = 0; y < SIZE; y += 1) {
    for (let x = 0; x < SIZE; x += 1) {
      paint(x, y, 10);
    }
  }

  // Frame
  for (let y = 0; y < SIZE; y += 1) {
    for (let x = 0; x < SIZE; x += 1) {
      const distance = Math.abs(roundedRectDistance(x + 0.5, y + 0.5, 40, 40, SIZE - 40, SIZE - 40, 84));
      if (distance < 7) paint(x, y, 244);
      const inner = Math.abs(roundedRectDistance(x + 0.5, y + 0.5, 58, 58, SIZE - 58, SIZE - 58, 70));
      if (inner < 1.5) paint(x, y, 40);
    }
  }

  // Dispatch bars: three stacked lines with a live indicator dot.
  const bars = [
    { top: 190, left: 132, right: 352, dot: true },
    { top: 252, left: 132, right: 292, dot: false },
    { top: 314, left: 132, right: 250, dot: false },
  ];
  for (const bar of bars) {
    for (let y = 0; y < SIZE; y += 1) {
      for (let x = 0; x < SIZE; x += 1) {
        const distance = roundedRectDistance(x + 0.5, y + 0.5, bar.left, bar.top, bar.right, bar.top + 34, 17);
        if (distance < 0) paint(x, y, 250);
        if (bar.dot && Math.hypot(x + 0.5 - 396, y + 0.5 - 207) < 17) paint(x, y, 250);
      }
    }
  }

  return buffer;
}

function resize(base, size) {
  const out = Buffer.alloc(size * size * 4);
  const ratio = SIZE / size;
  for (let y = 0; y < size; y += 1) {
    for (let x = 0; x < size; x += 1) {
      const sourceX = Math.min(SIZE - 1, Math.floor((x + 0.5) * ratio));
      const sourceY = Math.min(SIZE - 1, Math.floor((y + 0.5) * ratio));
      base.copy(out, (y * size + x) * 4, (sourceY * SIZE + sourceX) * 4, (sourceY * SIZE + sourceX) * 4 + 4);
    }
  }
  return png(size, size, out);
}

function ico(images) {
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(images.length, 4);
  const entries = [];
  let offset = 6 + images.length * 16;
  for (const { size, data } of images) {
    const entry = Buffer.alloc(16);
    entry[0] = size >= 256 ? 0 : size;
    entry[1] = size >= 256 ? 0 : size;
    entry.writeUInt16LE(1, 4);
    entry.writeUInt16LE(32, 6);
    entry.writeUInt32LE(data.length, 8);
    entry.writeUInt32LE(offset, 12);
    entries.push(entry);
    offset += data.length;
  }
  return Buffer.concat([header, ...entries, ...images.map((image) => image.data)]);
}

const base = renderBase();
mkdirSync("src-tauri/icons", { recursive: true });
const sizes = { 32: null, 128: null, 256: null };
for (const key of Object.keys(sizes)) {
  sizes[key] = resize(base, Number(key));
}
writeFileSync("src-tauri/icons/32x32.png", sizes[32]);
writeFileSync("src-tauri/icons/128x128.png", sizes[128]);
writeFileSync("src-tauri/icons/128x128@2x.png", sizes[256]);
writeFileSync("src-tauri/icons/icon.png", sizes[256]);
writeFileSync(
  "src-tauri/icons/icon.ico",
  ico([
    { size: 16, data: resize(base, 16) },
    { size: 32, data: sizes[32] },
    { size: 48, data: resize(base, 48) },
    { size: 256, data: sizes[256] },
  ]),
);
console.log("Icons written to src-tauri/icons");
