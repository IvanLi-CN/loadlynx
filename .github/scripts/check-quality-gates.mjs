import { readFile } from "node:fs/promises";

const policyPath = new URL("../quality-gates.json", import.meta.url);
const content = await readFile(policyPath, "utf8");
const qualityGates = JSON.parse(content);

const failures = [];

function expectEqual(actual, expected, label) {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    failures.push(`${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
  }
}

expectEqual(qualityGates.policy?.branch_protection?.protected_branches, ["main"], "protected_branches");
expectEqual(qualityGates.policy?.branch_protection?.require_pull_request, true, "require_pull_request");
expectEqual(qualityGates.policy?.branch_protection?.disallow_direct_pushes, true, "disallow_direct_pushes");
expectEqual(qualityGates.policy?.review_policy?.required_approvals, 0, "required_approvals");
expectEqual(qualityGates.required_checks, ["Label Gate"], "required_checks");

if (failures.length > 0) {
  console.error("quality-gates declaration drift detected:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log("quality-gates declaration ok");
