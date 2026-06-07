#!/usr/bin/env node
import assert from "node:assert/strict";
import {
  normalizeScalar,
  parseWorkflowMetadata,
  validateQualityGates,
} from "./check-quality-gates-lib.mjs";

const qualityGates = {
  policy: {
    branch_protection: {
      protected_branches: ["main"],
      require_pull_request: true,
      disallow_direct_pushes: true,
    },
    review_policy: {
      required_approvals: 0,
    },
  },
  required_checks: ["Label Gate"],
  informational_checks: ["check", "digital-check", "web-check"],
  expected_pr_workflows: [
    {
      workflow: "Label Gate",
      jobs: ["Label Gate"],
    },
    {
      workflow: "Code Check",
      jobs: ["check"],
    },
    {
      workflow: "Digital Check",
      jobs: ["digital-check"],
    },
    {
      workflow: "Web Check",
      jobs: ["web-check"],
    },
  ],
};

assert.equal(normalizeScalar(" 'Code Check' "), "Code Check");
assert.equal(normalizeScalar('"Label Gate"'), "Label Gate");

assert.deepEqual(
  parseWorkflowMetadata(
    `name: Code Check

jobs:
  host-rust:
    runs-on: ubuntu-latest
  check:
    name: check
    runs-on: ubuntu-latest
`,
    "check.yml",
  ),
  {
    fileName: "check.yml",
    name: "Code Check",
    hasPermissions: false,
    jobs: [
      { id: "host-rust", name: null, hasTimeoutMinutes: false },
      { id: "check", name: "check", hasTimeoutMinutes: false },
    ],
  },
);

const validWorkflows = [
  {
    fileName: "label-gate.yml",
    name: "Label Gate",
    jobs: [{ id: "label-gate", name: "Label Gate" }],
  },
  {
    fileName: "check.yml",
    name: "Code Check",
    hasPermissions: true,
    jobs: [
      { id: "host-rust", name: null, hasTimeoutMinutes: true },
      { id: "analog-firmware", name: null, hasTimeoutMinutes: true },
      { id: "digital-firmware", name: null, hasTimeoutMinutes: true },
      { id: "check", name: null, hasTimeoutMinutes: true },
    ],
  },
  {
    fileName: "digital-check.yml",
    name: "Digital Check",
    hasPermissions: true,
    jobs: [{ id: "digital-check", name: null, hasTimeoutMinutes: true }],
  },
  {
    fileName: "web-check.yml",
    name: "Web Check",
    hasPermissions: true,
    jobs: [{ id: "web-check", name: null, hasTimeoutMinutes: true }],
  },
];

assert.deepEqual(validateQualityGates({ qualityGates, workflows: validWorkflows }), []);

assert.deepEqual(validateQualityGates({ qualityGates, workflows: [] }), [
  "expected workflow missing locally: Label Gate",
  "expected workflow missing locally: Code Check",
  "expected workflow missing locally: Digital Check",
  "expected workflow missing locally: Web Check",
  'declared checks not backed by expected_pr_workflows: ["Label Gate","check","digital-check","web-check"]',
]);

assert.deepEqual(
  validateQualityGates({
    qualityGates,
    workflows: validWorkflows.map((workflow) =>
      workflow.name === "Code Check"
        ? { ...workflow, jobs: workflow.jobs.filter((job) => (job.name ?? job.id) !== "check") }
        : workflow,
    ),
  }),
  [
    'workflow Code Check missing declared job "check"; actual jobs: ["host-rust","analog-firmware","digital-firmware"]',
  ],
);

assert.deepEqual(
  validateQualityGates({
    qualityGates: {
      ...qualityGates,
      informational_checks: ["check", "digital-check", "web-check", "ghost-check"],
    },
    workflows: validWorkflows,
  }),
  ['declared checks not backed by expected_pr_workflows: ["ghost-check"]'],
);

console.log("quality-gates tests passed");
