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
    jobs: [
      { id: "build", name: "Build", hasTimeoutMinutes: true },
    ],
  },
);

const healthyWorkflow = {
  fileName: "healthy.yml",
  name: "Healthy",
  hasPermissions: true,
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

console.log("workflow hygiene tests passed");
