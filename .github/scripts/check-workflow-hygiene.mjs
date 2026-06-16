#!/usr/bin/env node
import {
  validateCurrentTruthDocs,
  validateHttpSurfaceContracts,
  validateReleasedCliDocs,
  loadWorkflowMetadata,
  validateWebToolingContracts,
  validateWorkflowHygiene,
} from "./check-quality-gates-lib.mjs";

const workflowsDir = new URL("../workflows/", import.meta.url);
const { failures: workflowFailures, workflows } = await loadWorkflowMetadata(workflowsDir);
const hygieneFailures = validateWorkflowHygiene({ workflows });
const toolingFailures = await validateWebToolingContracts();
const currentTruthDocFailures = await validateCurrentTruthDocs();
const httpSurfaceFailures = await validateHttpSurfaceContracts();
const releasedCliDocFailures = await validateReleasedCliDocs();
const failures = [
  ...workflowFailures,
  ...hygieneFailures,
  ...toolingFailures,
  ...currentTruthDocFailures,
  ...httpSurfaceFailures,
  ...releasedCliDocFailures,
];

if (failures.length > 0) {
  console.error("workflow hygiene drift detected:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log("workflow hygiene ok");
