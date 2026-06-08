#!/usr/bin/env node
import assert from "node:assert/strict";
import {
  parseWorkflowMetadata,
  validateWebToolingContracts,
  validateWorkflowHygiene,
} from "./check-quality-gates-lib.mjs";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";

assert.deepEqual(
  parseWorkflowMetadata(
    `name: Example
permissions:
  contents: read

jobs:
  build:
    name: Build
    runs-on: ubuntu-latest
    timeout-minutes: 15
`,
    "example.yml",
  ),
  {
    fileName: "example.yml",
    name: "Example",
    hasPermissions: true,
    setupBunUsesVersionFile: [],
    setupBunUsesInlineVersion: [],
    jobs: [
      { id: "build", name: "Build", hasTimeoutMinutes: true },
    ],
  },
);

const healthyWorkflow = {
  fileName: "healthy.yml",
  name: "Healthy",
  hasPermissions: true,
  setupBunUsesVersionFile: [],
  setupBunUsesInlineVersion: [],
  jobs: [
    { id: "build", name: null, hasTimeoutMinutes: true },
    { id: "publish", name: "Publish", hasTimeoutMinutes: true },
  ],
};

assert.deepEqual(validateWorkflowHygiene({ workflows: [healthyWorkflow] }), []);

assert.deepEqual(
  validateWorkflowHygiene({
    workflows: [
      {
        ...healthyWorkflow,
        hasPermissions: false,
      },
    ],
  }),
  ["workflow healthy.yml: missing top-level permissions"],
);

assert.deepEqual(
  validateWorkflowHygiene({
    workflows: [
      {
        ...healthyWorkflow,
        jobs: [
          { id: "build", name: null, hasTimeoutMinutes: true },
          { id: "publish", name: "Publish", hasTimeoutMinutes: false },
        ],
      },
    ],
  }),
  ['workflow healthy.yml job "Publish": missing timeout-minutes'],
);

assert.deepEqual(
  parseWorkflowMetadata(
    `name: Bun
permissions:
  contents: read

jobs:
  build:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - uses: oven-sh/setup-bun@v2
        with:
          bun-version-file: ".bun-version"
`,
    "bun.yml",
  ),
  {
    fileName: "bun.yml",
    name: "Bun",
    hasPermissions: true,
    setupBunUsesVersionFile: [".bun-version"],
    setupBunUsesInlineVersion: [],
    jobs: [
      { id: "build", name: null, hasTimeoutMinutes: true },
    ],
  },
);

assert.deepEqual(
  validateWorkflowHygiene({
    workflows: [
      {
        ...healthyWorkflow,
        fileName: "inline-bun.yml",
        setupBunUsesVersionFile: [],
        setupBunUsesInlineVersion: ["1.3.14"],
      },
    ],
  }),
  [
    "workflow inline-bun.yml: setup-bun must use bun-version-file=.bun-version instead of inline bun-version",
  ],
);

assert.deepEqual(
  validateWorkflowHygiene({
    workflows: [
      {
        ...healthyWorkflow,
        fileName: "wrong-bun-file.yml",
        setupBunUsesVersionFile: ["package.json"],
        setupBunUsesInlineVersion: [],
      },
    ],
  }),
  ['workflow wrong-bun-file.yml: setup-bun bun-version-file must be ".bun-version"'],
);

const tempDir = await mkdtemp(join(tmpdir(), "loadlynx-workflow-hygiene-"));
try {
  const webPackagePath = join(tempDir, "package.json");
  const workflowPath = join(tempDir, "web-check.yml");
  await writeFile(
    webPackagePath,
    JSON.stringify(
      {
        scripts: {
          "test:e2e": "node scripts/run-playwright.mjs test",
          "test:e2e:ui": "node scripts/run-playwright.mjs test --ui",
        },
      },
      null,
      2,
    ),
  );
  await writeFile(
    workflowPath,
    `name: Web Check
jobs:
  web-check:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps: []
`,
  );

  assert.deepEqual(
    await validateWebToolingContracts({
      webPackageJsonPath: new URL(`file://${webPackagePath}`),
      webCheckWorkflowPath: new URL(`file://${workflowPath}`),
    }),
    [
      ".github/workflows/web-check.yml: Install Playwright browsers step must run node scripts/run-playwright.mjs install --with-deps",
    ],
  );

  await writeFile(
    webPackagePath,
    JSON.stringify(
      {
        scripts: {
          "test:e2e": "playwright test",
          "test:e2e:ui": "playwright test --ui",
        },
      },
      null,
      2,
    ),
  );

  assert.deepEqual(
    await validateWebToolingContracts({
      webPackageJsonPath: new URL(`file://${webPackagePath}`),
      webCheckWorkflowPath: new URL(`file://${workflowPath}`),
    }),
    [
      'web/package.json: scripts["test:e2e"] must be "node scripts/run-playwright.mjs test"',
      'web/package.json: scripts["test:e2e:ui"] must be "node scripts/run-playwright.mjs test --ui"',
      ".github/workflows/web-check.yml: Install Playwright browsers step must run node scripts/run-playwright.mjs install --with-deps",
    ],
  );

  await writeFile(
    workflowPath,
    `name: Web Check
jobs:
  web-check:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - name: Install Playwright browsers
        run: node scripts/run-playwright.mjs install --with-deps
`,
  );

  await writeFile(
    webPackagePath,
    JSON.stringify(
      {
        scripts: {
          "test:e2e": "node scripts/run-playwright.mjs test",
          "test:e2e:ui": "node scripts/run-playwright.mjs test --ui",
        },
      },
      null,
      2,
    ),
  );

  assert.deepEqual(
    await validateWebToolingContracts({
      webPackageJsonPath: new URL(`file://${webPackagePath}`),
      webCheckWorkflowPath: new URL(`file://${workflowPath}`),
    }),
    [],
  );

  await writeFile(
    workflowPath,
    `name: Web Check
jobs:
  web-check:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - name: Install Playwright browsers
        run: bunx playwright install --with-deps
`,
  );

  assert.deepEqual(
    await validateWebToolingContracts({
      webPackageJsonPath: new URL(`file://${webPackagePath}`),
      webCheckWorkflowPath: new URL(`file://${workflowPath}`),
    }),
    [
      ".github/workflows/web-check.yml: Install Playwright browsers step must run node scripts/run-playwright.mjs install --with-deps",
      ".github/workflows/web-check.yml: bunx playwright install is not allowed; use node scripts/run-playwright.mjs install --with-deps",
    ],
  );
} finally {
  await rm(tempDir, { recursive: true, force: true });
}

console.log("workflow hygiene tests passed");
