import { readdir, readFile } from "node:fs/promises";

export function normalizeScalar(value) {
  return value.trim().replace(/^['"]|['"]$/g, "");
}

export function parseWorkflowMetadata(source, fileName) {
  const lines = source.split(/\r?\n/);
  const workflow = {
    fileName,
    name: null,
    hasPermissions: false,
    jobs: [],
    setupBunUsesVersionFile: [],
    setupBunUsesInlineVersion: [],
  };

  let currentJob = null;
  let inJobsSection = false;
  let inSetupBunBlock = false;
  let setupBunIndent = -1;

  for (const line of lines) {
    const usesSetupBunMatch = line.match(/^(\s*)(?:-\s*)?uses:\s*oven-sh\/setup-bun@.+$/);
    if (usesSetupBunMatch) {
      inSetupBunBlock = true;
      setupBunIndent = usesSetupBunMatch[1].length;
    } else if (inSetupBunBlock) {
      const trimmed = line.trim();
      const indent = line.match(/^(\s*)/)?.[1].length ?? 0;
      if (trimmed.length > 0 && indent <= setupBunIndent) {
        inSetupBunBlock = false;
        setupBunIndent = -1;
      }
    }

    if (inSetupBunBlock) {
      const bunVersionFileMatch = line.match(/^\s*bun-version-file:\s*(.+?)\s*$/);
      if (bunVersionFileMatch) {
        workflow.setupBunUsesVersionFile.push(normalizeScalar(bunVersionFileMatch[1]));
      }
      const bunVersionMatch = line.match(/^\s*bun-version:\s*(.+?)\s*$/);
      if (bunVersionMatch) {
        workflow.setupBunUsesInlineVersion.push(normalizeScalar(bunVersionMatch[1]));
      }
    }

    if (!workflow.name) {
      const workflowNameMatch = line.match(/^name:\s*(.+?)\s*$/);
      if (workflowNameMatch) {
        workflow.name = normalizeScalar(workflowNameMatch[1]);
        continue;
      }
    }

    if (!workflow.hasPermissions && /^permissions:\s*(.*)$/.test(line)) {
      workflow.hasPermissions = true;
    }

    if (!inJobsSection) {
      if (/^jobs:\s*$/.test(line)) {
        inJobsSection = true;
      }
      continue;
    }

    if (/^\S/.test(line) && !/^jobs:\s*$/.test(line)) {
      break;
    }

    const jobIdMatch = line.match(/^ {2}([A-Za-z0-9_-]+):\s*$/);
    if (jobIdMatch) {
      currentJob = {
        id: jobIdMatch[1],
        name: null,
        hasTimeoutMinutes: false,
      };
      workflow.jobs.push(currentJob);
      continue;
    }

    if (!currentJob) {
      continue;
    }

    const jobNameMatch = line.match(/^ {4}name:\s*(.+?)\s*$/);
    if (jobNameMatch) {
      currentJob.name = normalizeScalar(jobNameMatch[1]);
      continue;
    }

    if (/^ {4}timeout-minutes:\s*\d+\s*$/.test(line)) {
      currentJob.hasTimeoutMinutes = true;
    }
  }

  return workflow;
}

export async function loadWorkflowMetadata(workflowsDir) {
  const entries = await readdir(workflowsDir, { withFileTypes: true });
  const workflows = [];
  const failures = [];

  for (const entry of entries) {
    if (!entry.isFile() || !entry.name.match(/\.ya?ml$/)) {
      continue;
    }

    const fileUrl = new URL(entry.name, workflowsDir);
    const workflow = parseWorkflowMetadata(await readFile(fileUrl, "utf8"), entry.name);

    if (!workflow.name) {
      failures.push(`workflow ${entry.name}: missing top-level name`);
      continue;
    }

    if (workflow.jobs.length === 0) {
      failures.push(`workflow ${entry.name}: missing jobs`);
      continue;
    }

    workflows.push(workflow);
  }

  return { failures, workflows };
}

export function validateQualityGates({ qualityGates, workflows }) {
  const failures = [];

  function expectEqual(actual, expected, label) {
    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
      failures.push(`${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
    }
  }

  function findWorkflowByName(workflowName) {
    return workflows.find((workflow) => workflow.name === workflowName);
  }

  expectEqual(qualityGates.policy?.branch_protection?.protected_branches, ["main"], "protected_branches");
  expectEqual(qualityGates.policy?.branch_protection?.require_pull_request, true, "require_pull_request");
  expectEqual(qualityGates.policy?.branch_protection?.disallow_direct_pushes, true, "disallow_direct_pushes");
  expectEqual(qualityGates.policy?.review_policy?.required_approvals, 0, "required_approvals");
  expectEqual(qualityGates.required_checks, ["Label Gate"], "required_checks");

  const declaredCheckNames = new Set([
    ...(qualityGates.required_checks ?? []),
    ...(qualityGates.informational_checks ?? []),
  ]);

  for (const expectedWorkflow of qualityGates.expected_pr_workflows ?? []) {
    const workflow = findWorkflowByName(expectedWorkflow.workflow);
    if (!workflow) {
      failures.push(`expected workflow missing locally: ${expectedWorkflow.workflow}`);
      continue;
    }

    const actualJobNames = workflow.jobs.map((job) => job.name ?? job.id);
    for (const expectedJobName of expectedWorkflow.jobs ?? []) {
      if (!actualJobNames.includes(expectedJobName)) {
        failures.push(
          `workflow ${expectedWorkflow.workflow} missing declared job ${JSON.stringify(expectedJobName)}; actual jobs: ${JSON.stringify(actualJobNames)}`,
        );
      }
    }

    for (const expectedJobName of expectedWorkflow.jobs ?? []) {
      declaredCheckNames.delete(expectedJobName);
    }
  }

  if (declaredCheckNames.size > 0) {
    failures.push(
      `declared checks not backed by expected_pr_workflows: ${JSON.stringify([...declaredCheckNames].sort())}`,
    );
  }

  return failures;
}

export function validateWorkflowHygiene({ workflows }) {
  const failures = [];

  for (const workflow of workflows) {
    if (!workflow.hasPermissions) {
      failures.push(`workflow ${workflow.fileName}: missing top-level permissions`);
    }

    for (const job of workflow.jobs) {
      if (!job.hasTimeoutMinutes) {
        failures.push(`workflow ${workflow.fileName} job ${JSON.stringify(job.name ?? job.id)}: missing timeout-minutes`);
      }
    }

    if (workflow.setupBunUsesInlineVersion.length > 0) {
      failures.push(
        `workflow ${workflow.fileName}: setup-bun must use bun-version-file=.bun-version instead of inline bun-version`,
      );
    }

    if (
      workflow.setupBunUsesVersionFile.length > 0 &&
      workflow.setupBunUsesVersionFile.some((value) => value !== ".bun-version")
    ) {
      failures.push(
        `workflow ${workflow.fileName}: setup-bun bun-version-file must be ".bun-version"`,
      );
    }
  }

  return failures;
}

export async function runQualityGatesCheck({
  policyPath = new URL("../quality-gates.json", import.meta.url),
  workflowsDir = new URL("../workflows/", import.meta.url),
} = {}) {
  const content = await readFile(policyPath, "utf8");
  const qualityGates = JSON.parse(content);
  const { failures: workflowFailures, workflows } = await loadWorkflowMetadata(workflowsDir);
  const validationFailures = validateQualityGates({ qualityGates, workflows });

  return {
    failures: [...workflowFailures, ...validationFailures],
    qualityGates,
    workflows,
  };
}

export async function validateWebToolingContracts({
  webPackageJsonPath = new URL("../../web/package.json", import.meta.url),
  webPackageLockPath = new URL("../../web/package-lock.json", import.meta.url),
  webCheckWorkflowPath = new URL("../workflows/web-check.yml", import.meta.url),
  webPagesWorkflowPath = new URL("../workflows/web-pages.yml", import.meta.url),
  releaseWorkflowPath = new URL("../workflows/release.yml", import.meta.url),
} = {}) {
  const failures = [];
  const webPackage = JSON.parse(await readFile(webPackageJsonPath, "utf8"));
  const scripts = webPackage.scripts ?? {};
  const webCheckWorkflow = await readFile(webCheckWorkflowPath, "utf8");
  const webPagesWorkflow = await readFile(webPagesWorkflowPath, "utf8");
  const releaseWorkflow = await readFile(releaseWorkflowPath, "utf8");
  const webInstallGuard =
    'if [ -f package-lock.json ]; then\n            echo "ERROR: web/package-lock.json is not supported. Use Bun and web/bun.lock only." >&2\n            exit 1\n          fi\n          bun ci';

  if (scripts["test:e2e"] !== "node scripts/run-playwright.mjs test") {
    failures.push(
      'web/package.json: scripts["test:e2e"] must be "node scripts/run-playwright.mjs test"',
    );
  }

  if (scripts["test:e2e:ui"] !== "node scripts/run-playwright.mjs test --ui") {
    failures.push(
      'web/package.json: scripts["test:e2e:ui"] must be "node scripts/run-playwright.mjs test --ui"',
    );
  }

  if (!webCheckWorkflow.includes("run: node scripts/run-playwright.mjs install --with-deps")) {
    failures.push(
      ".github/workflows/web-check.yml: Install Playwright browsers step must run node scripts/run-playwright.mjs install --with-deps",
    );
  }

  if (/\bbunx\s+playwright\s+install\b/.test(webCheckWorkflow)) {
    failures.push(
      ".github/workflows/web-check.yml: bunx playwright install is not allowed; use node scripts/run-playwright.mjs install --with-deps",
    );
  }

  try {
    await readFile(webPackageLockPath, "utf8");
    failures.push("web/package-lock.json must not exist; use web/bun.lock as the only lockfile");
  } catch (error) {
    if (error?.code !== "ENOENT") {
      throw error;
    }
  }

  const bunOnlyWorkflows = [
    [".github/workflows/web-check.yml", webCheckWorkflow],
    [".github/workflows/web-pages.yml", webPagesWorkflow],
    [".github/workflows/release.yml", releaseWorkflow],
  ];

  for (const [label, source] of bunOnlyWorkflows) {
    if (!source.includes(webInstallGuard)) {
      failures.push(
        `${label}: web install step must reject web/package-lock.json and run bun ci`,
      );
    }
  }

  return failures;
}
