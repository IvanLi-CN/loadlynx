import { spawn } from "node:child_process";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const playwrightCli = require.resolve("@playwright/test/cli");

const env = { ...process.env };
delete env.NO_COLOR;

const child = spawn(
  process.execPath,
  [playwrightCli, ...process.argv.slice(2)],
  {
    stdio: "inherit",
    env,
  },
);

child.on("error", (error) => {
  console.error("[playwright] failed to start:", error);
  process.exit(1);
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code ?? 1);
});
