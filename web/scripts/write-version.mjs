import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

try {
  const pkgPath = join(__dirname, "..", "package.json");
  const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));

  const envVersionRaw =
    process.env.APP_EFFECTIVE_VERSION ?? process.env.VITE_APP_VERSION ?? "";
  const envVersion = envVersionRaw.trim();

  const isCi =
    process.env.CI === "true" ||
    process.env.GITHUB_ACTIONS === "true" ||
    process.env.APP_EFFECTIVE_VERSION != null;

  let baseVersion;
  if (envVersion.length > 0) {
    baseVersion = envVersion;
  } else if (typeof pkg.version === "string" && pkg.version.length > 0) {
    baseVersion = pkg.version;
  } else {
    baseVersion = "0.0.0";
  }

  let version = baseVersion;
  if (!isCi && envVersion.length === 0) {
    if (!/-dev($|\\.)/.test(baseVersion)) {
      version = `${baseVersion}-dev`;
    }
  }

  const projectRoot = join(__dirname, "..");
  const distDir = join(projectRoot, "dist");
  const publicDir = join(projectRoot, "public");
  mkdirSync(distDir, { recursive: true });
  mkdirSync(publicDir, { recursive: true });

  const payload = {
    version,
    builtAt: new Date().toISOString(),
  };

  const distTarget = join(distDir, "version.json");
  const publicTarget = join(publicDir, "version.json");

  writeFileSync(distTarget, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
  writeFileSync(publicTarget, `${JSON.stringify(payload, null, 2)}\n`, "utf8");

  console.log(
    `[version] wrote ${version} to dist/version.json and public/version.json`,
  );
} catch (error) {
  console.error("[version] failed to write version.json:", error);
  process.exit(1);
}
