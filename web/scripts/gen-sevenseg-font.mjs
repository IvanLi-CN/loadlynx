import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const fontCPath = resolve("..", "docs", "assets", "fonts", "SevenSegNumFont.c");
const raw = readFileSync(fontCPath, "utf8");

const match = raw.match(
  /SevenSegNumFont\s*\[[^\]]*]\s*PROGMEM\s*=\s*{([\s\S]*?)};/,
);
if (!match) {
  throw new Error("Failed to locate SevenSegNumFont array in C file");
}

const body = match[1];
// Strip line comments like `// 0` that follow byte values so we do not
// accidentally parse the trailing `0` as another byte.
const bodyNoComments = body.replace(/\/\/.*$/gm, "");
const byteStrings = bodyNoComments
  .split(/[, \t\r\n]+/)
  .map((s) => s.trim())
  .filter((s) => s.length > 0 && !s.startsWith("//"));

// Plain JS array; keep this file valid ESM JavaScript.
const bytes = [];
for (const token of byteStrings) {
  let value;
  if (token.startsWith("0x") || token.startsWith("0X")) {
    value = Number.parseInt(token, 16);
  } else {
    value = Number.parseInt(token, 10);
  }
  if (Number.isNaN(value)) {
    continue;
  }
  bytes.push(value & 0xff);
}

if (bytes.length !== 2004) {
  throw new Error(`Expected 2004 bytes, got ${bytes.length}`);
}

const width = bytes[0];
const height = bytes[1];
const firstChar = bytes[2];
const charCount = bytes[3];
const glyphBytes = ((width * height + 7) >> 3) | 0;

if (glyphBytes * charCount !== bytes.length - 4) {
  throw new Error(
    `Header/body mismatch: width=${width}, height=${height}, glyphBytes=${glyphBytes}, dataBytes=${
      bytes.length - 4
    }`,
  );
}

const dataSlice = bytes.slice(4);

const outPath = resolve("src", "fonts", "sevenSegFont.ts");
const out =
  "// Auto-generated from docs/assets/fonts/SevenSegNumFont.c\n" +
  `// Font: numeric, ${width}x${height}, firstChar=${firstChar} ('${String.fromCharCode(
    firstChar,
  )}'), chars=${charCount}\n` +
  `export const sevenSegFontWidth = ${width} as const;\n` +
  `export const sevenSegFontHeight = ${height} as const;\n` +
  `export const sevenSegFontFirstChar = ${firstChar} as const;\n` +
  `export const sevenSegFontCharCount = ${charCount} as const;\n` +
  `export const sevenSegFontData = new Uint8Array([\n  ${dataSlice.join(
    ",",
  )}\n]);\n` +
  "export function isSevenSegPixel(code: number, x: number, y: number): boolean {\n" +
  "  if (code < sevenSegFontFirstChar || code >= sevenSegFontFirstChar + sevenSegFontCharCount) return false;\n" +
  "  const glyphIndex = code - sevenSegFontFirstChar;\n" +
  "  if (x < 0 || x >= sevenSegFontWidth || y < 0 || y >= sevenSegFontHeight) return false;\n" +
  "  const bitIndex = y * sevenSegFontWidth + x;\n" +
  `  const byteIndex = glyphIndex * ${glyphBytes} + (bitIndex >> 3);\n` +
  "  const bitMask = 0x80 >> (bitIndex & 7);\n" +
  "  return (sevenSegFontData[byteIndex] & bitMask) !== 0;\n" +
  "}\n";

writeFileSync(outPath, out, "utf8");
console.log(`Generated ${outPath} from ${fontCPath}`);
