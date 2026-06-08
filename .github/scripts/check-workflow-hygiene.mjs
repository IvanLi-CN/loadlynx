#!/usr/bin/env node
import {
  loadWorkflowMetadata,
  validateWebToolingContracts,
  validateWorkflowHygiene,
} from "./check-quality-gates-lib.mjs";

const workflowsDir = new URL("../workflows/", import.meta.url);
const { failures: workflowFailures, workflows } = await loadWorkflowMetadata(workflowsDir);
const hygieneFailures = validateWorkflowHygiene({ workflows });
const toolingFailures = await validateWebToolingContracts();
const failures = [...workflowFailures, ...hygieneFailures, ...toolingFailures];

if (failures.length > 0) {
  console.error("workflow hygiene drift detected:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log("workflow hygiene ok");
