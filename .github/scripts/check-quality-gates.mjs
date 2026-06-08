import { runQualityGatesCheck } from "./check-quality-gates-lib.mjs";

const { failures } = await runQualityGatesCheck();

if (failures.length > 0) {
  console.error("quality-gates declaration drift detected:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log("quality-gates declaration ok");
