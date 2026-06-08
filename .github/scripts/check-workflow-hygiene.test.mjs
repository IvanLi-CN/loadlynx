#!/usr/bin/env node
import assert from "node:assert/strict";
import {
  parseWorkflowMetadata,
  validateWorkflowHygiene,
} from "./check-quality-gates-lib.mjs";

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

console.log("workflow hygiene tests passed");
