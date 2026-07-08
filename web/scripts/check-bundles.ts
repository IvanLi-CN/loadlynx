import { readdir, stat } from "node:fs/promises";
import * as path from "node:path";

type Target = "all" | "app" | "storybook";

type FileBudget = {
  label: string;
  relativePath: string;
  limitBytes: number;
};

type DirectoryBudget = {
  label: string;
  relativeDir: string;
  limitBytes: number;
};

type BudgetCheckResult = {
  label: string;
  files: Array<{ relativePath: string; sizeBytes: number; limitBytes: number }>;
};

const APP_CHUNK_LIMIT_BYTES = 252_000;
const STORYBOOK_PREVIEW_CHUNK_LIMIT_BYTES = 252_000;
const STORYBOOK_FRAMEWORK_RUNTIME_LIMIT_BYTES = 1_200_000;

const APP_DIRECTORY_BUDGETS: DirectoryBudget[] = [
  {
    label: "app JS chunk",
    relativeDir: path.join("dist", "assets"),
    limitBytes: APP_CHUNK_LIMIT_BYTES,
  },
];

const STORYBOOK_DIRECTORY_BUDGETS: DirectoryBudget[] = [
  {
    label: "storybook preview JS chunk",
    relativeDir: path.join("storybook-static", "assets"),
    limitBytes: STORYBOOK_PREVIEW_CHUNK_LIMIT_BYTES,
  },
];

const STORYBOOK_FILE_BUDGETS: FileBudget[] = [
  {
    label: "storybook framework mocker runtime",
    relativePath: path.join("storybook-static", "vite-inject-mocker-entry.js"),
    limitBytes: STORYBOOK_FRAMEWORK_RUNTIME_LIMIT_BYTES,
  },
];

function usage(): string {
  return ["Usage:", "  bun scripts/check-bundles.ts <all|app|storybook>"].join(
    "\n",
  );
}

function formatKilobytes(sizeBytes: number): string {
  return `${(sizeBytes / 1_000).toFixed(2)} kB`;
}

async function collectJsFiles(absoluteDir: string): Promise<string[]> {
  const dirEntries = await readdir(absoluteDir, { withFileTypes: true });
  const files = await Promise.all(
    dirEntries.map(async (entry) => {
      const resolved = path.join(absoluteDir, entry.name);
      if (entry.isDirectory()) {
        return collectJsFiles(resolved);
      }

      if (entry.isFile() && resolved.endsWith(".js")) {
        return [resolved];
      }

      return [];
    }),
  );

  return files.flat();
}

async function evaluateDirectoryBudget(
  cwd: string,
  budget: DirectoryBudget,
): Promise<BudgetCheckResult> {
  const absoluteDir = path.join(cwd, budget.relativeDir);
  const jsFiles = await collectJsFiles(absoluteDir);

  const files = await Promise.all(
    jsFiles.map(async (absolutePath) => {
      const fileStat = await stat(absolutePath);
      return {
        relativePath: path.relative(cwd, absolutePath),
        sizeBytes: fileStat.size,
        limitBytes: budget.limitBytes,
      };
    }),
  );

  files.sort((left, right) => right.sizeBytes - left.sizeBytes);

  return { label: budget.label, files };
}

async function evaluateFileBudget(
  cwd: string,
  budget: FileBudget,
): Promise<BudgetCheckResult> {
  const absolutePath = path.join(cwd, budget.relativePath);
  const fileStat = await stat(absolutePath);

  return {
    label: budget.label,
    files: [
      {
        relativePath: budget.relativePath,
        sizeBytes: fileStat.size,
        limitBytes: budget.limitBytes,
      },
    ],
  };
}

function printResult(result: BudgetCheckResult): boolean {
  if (result.files.length === 0) {
    throw new Error(
      `[bundle-check] No JavaScript files found for ${result.label}`,
    );
  }

  let hasViolation = false;
  console.log(`[bundle-check] ${result.label}`);

  for (const file of result.files) {
    const status = file.sizeBytes > file.limitBytes ? "FAIL" : "OK";
    if (status === "FAIL") {
      hasViolation = true;
    }

    console.log(
      `  ${status} ${file.relativePath} ${formatKilobytes(file.sizeBytes)} / limit ${formatKilobytes(file.limitBytes)}`,
    );
  }

  return hasViolation;
}

async function runTarget(
  cwd: string,
  target: Exclude<Target, "all">,
): Promise<boolean> {
  const results: BudgetCheckResult[] = [];

  if (target === "app") {
    for (const budget of APP_DIRECTORY_BUDGETS) {
      results.push(await evaluateDirectoryBudget(cwd, budget));
    }
  } else {
    for (const budget of STORYBOOK_DIRECTORY_BUDGETS) {
      results.push(await evaluateDirectoryBudget(cwd, budget));
    }
    for (const budget of STORYBOOK_FILE_BUDGETS) {
      results.push(await evaluateFileBudget(cwd, budget));
    }
  }

  let hasViolation = false;
  for (const result of results) {
    hasViolation = printResult(result) || hasViolation;
  }

  return hasViolation;
}

async function main() {
  const targetArg = process.argv[2];
  if (targetArg !== "all" && targetArg !== "app" && targetArg !== "storybook") {
    console.error(usage());
    process.exit(2);
  }

  const cwd = process.cwd();
  const targets: Array<Exclude<Target, "all">> =
    targetArg === "all" ? ["app", "storybook"] : [targetArg];

  let hasViolation = false;
  for (const target of targets) {
    hasViolation = (await runTarget(cwd, target)) || hasViolation;
  }

  if (hasViolation) {
    process.exit(1);
  }
}

await main();
