import { readdir, readFile } from "node:fs/promises";

export function normalizeScalar(value) {
  return value.trim().replace(/^['"]|['"]$/g, "");
}

function actionStepIndent(line, usesIndent) {
  return line.slice(usesIndent).startsWith("- ")
    ? usesIndent
    : Math.max(0, usesIndent - 2);
}

export function parseWorkflowMetadata(source, fileName) {
  const lines = source.split(/\r?\n/);
  const workflow = {
    fileName,
    name: null,
    hasPermissions: false,
    jobs: [],
    setupNodeUsesVersionFile: [],
    setupNodeUsesInlineVersion: [],
    setupBunUsesVersionFile: [],
    setupBunUsesInlineVersion: [],
  };

  let currentJob = null;
  let inJobsSection = false;
  let inSetupNodeBlock = false;
  let setupNodeStepIndent = -1;
  let setupNodeWithIndent = -1;
  let inSetupBunBlock = false;
  let setupBunStepIndent = -1;
  let setupBunWithIndent = -1;

  for (const line of lines) {
    const trimmed = line.trim();
    const indent = line.match(/^(\s*)/)?.[1].length ?? 0;

    const usesSetupNodeMatch = line.match(/^(\s*)(?:-\s*)?uses:\s*actions\/setup-node@.+$/);
    if (usesSetupNodeMatch) {
      inSetupNodeBlock = true;
      setupNodeStepIndent = actionStepIndent(line, usesSetupNodeMatch[1].length);
      setupNodeWithIndent = -1;
    } else if (inSetupNodeBlock) {
      if (trimmed.length > 0 && indent <= setupNodeStepIndent) {
        inSetupNodeBlock = false;
        setupNodeStepIndent = -1;
        setupNodeWithIndent = -1;
      }
    }

    if (inSetupNodeBlock) {
      if (/^with:\s*$/.test(trimmed) && indent > setupNodeStepIndent) {
        setupNodeWithIndent = indent;
      } else if (
        setupNodeWithIndent >= 0 &&
        trimmed.length > 0 &&
        indent <= setupNodeWithIndent
      ) {
        setupNodeWithIndent = -1;
      }

      if (setupNodeWithIndent >= 0 && indent > setupNodeWithIndent) {
        const nodeVersionFileMatch = line.match(/^\s*node-version-file:\s*(.+?)\s*$/);
        if (nodeVersionFileMatch) {
          workflow.setupNodeUsesVersionFile.push(normalizeScalar(nodeVersionFileMatch[1]));
        }
        const nodeVersionMatch = line.match(/^\s*node-version:\s*(.+?)\s*$/);
        if (nodeVersionMatch) {
          workflow.setupNodeUsesInlineVersion.push(normalizeScalar(nodeVersionMatch[1]));
        }
      }
    }

    const usesSetupBunMatch = line.match(/^(\s*)(?:-\s*)?uses:\s*oven-sh\/setup-bun@.+$/);
    if (usesSetupBunMatch) {
      inSetupBunBlock = true;
      setupBunStepIndent = actionStepIndent(line, usesSetupBunMatch[1].length);
      setupBunWithIndent = -1;
    } else if (inSetupBunBlock) {
      if (trimmed.length > 0 && indent <= setupBunStepIndent) {
        inSetupBunBlock = false;
        setupBunStepIndent = -1;
        setupBunWithIndent = -1;
      }
    }

    if (inSetupBunBlock) {
      if (/^with:\s*$/.test(trimmed) && indent > setupBunStepIndent) {
        setupBunWithIndent = indent;
      } else if (
        setupBunWithIndent >= 0 &&
        trimmed.length > 0 &&
        indent <= setupBunWithIndent
      ) {
        setupBunWithIndent = -1;
      }

      if (setupBunWithIndent >= 0 && indent > setupBunWithIndent) {
        const bunVersionFileMatch = line.match(/^\s*bun-version-file:\s*(.+?)\s*$/);
        if (bunVersionFileMatch) {
          workflow.setupBunUsesVersionFile.push(normalizeScalar(bunVersionFileMatch[1]));
        }
        const bunVersionMatch = line.match(/^\s*bun-version:\s*(.+?)\s*$/);
        if (bunVersionMatch) {
          workflow.setupBunUsesInlineVersion.push(normalizeScalar(bunVersionMatch[1]));
        }
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

    if (/^ {4}uses:\s*\S/.test(line)) {
      currentJob.isReusableWorkflow = true;
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
      if (job.isReusableWorkflow && job.hasTimeoutMinutes) {
        failures.push(
          `workflow ${workflow.fileName} reusable job ${JSON.stringify(job.name ?? job.id)}: timeout-minutes is not supported`,
        );
      } else if (!job.isReusableWorkflow && !job.hasTimeoutMinutes) {
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

    if (workflow.setupNodeUsesInlineVersion.length > 0) {
      failures.push(
        `workflow ${workflow.fileName}: setup-node must use node-version-file=.node-version instead of inline node-version`,
      );
    }

    if (
      workflow.setupNodeUsesVersionFile.length > 0 &&
      workflow.setupNodeUsesVersionFile.some((value) => value !== ".node-version")
    ) {
      failures.push(
        `workflow ${workflow.fileName}: setup-node node-version-file must be ".node-version"`,
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
  releaseWorkflowPath = new URL("../workflows/release.yml", import.meta.url),
} = {}) {
  const failures = [];
  const webPackage = JSON.parse(await readFile(webPackageJsonPath, "utf8"));
  const scripts = webPackage.scripts ?? {};
  const webCheckWorkflow = await readFile(webCheckWorkflowPath, "utf8");
  const releaseWorkflow = await readFile(releaseWorkflowPath, "utf8");
  const webInstallGuard =
    'if [ -f package-lock.json ]; then\n            echo "ERROR: web/package-lock.json is not supported. Use Bun and web/bun.lock only." >&2\n            exit 1\n          fi\n          bun ci';

  if (scripts["test:e2e"] !== "node scripts/run-playwright.mjs test") {
    failures.push(
      'web/package.json: scripts["test:e2e"] must be "node scripts/run-playwright.mjs test"',
    );
  }

  if (
    scripts["test:preview-smoke"] !==
    "node scripts/run-playwright.mjs test --config playwright.preview.config.ts"
  ) {
    failures.push(
      'web/package.json: scripts["test:preview-smoke"] must be "node scripts/run-playwright.mjs test --config playwright.preview.config.ts"',
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

  if (!webCheckWorkflow.includes("run: bun run test:preview-smoke")) {
    failures.push(
      ".github/workflows/web-check.yml: Production preview smoke step must run bun run test:preview-smoke",
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

export async function validateReleasePagesContracts({
  webPagesWorkflowPath = new URL("../workflows/web-pages.yml", import.meta.url),
  releaseWorkflowPath = new URL("../workflows/release.yml", import.meta.url),
} = {}) {
  const failures = [];
  const webPagesWorkflow = await readFile(webPagesWorkflowPath, "utf8");
  const releaseWorkflow = await readFile(releaseWorkflowPath, "utf8");

  const pagesRequirements = [
    ["workflow_dispatch:", "must only expose workflow_dispatch deployment"],
    ["release_tag:", "must require a release_tag input"],
    ["required: true", "release_tag input must be required"],
    ["gh release download", "must download the published release Web asset"],
    ["prepare-pages-release-bundle.mjs", "must validate the downloaded release bundle"],
    ["path: dist/pages", "must upload the validated Pages directory"],
  ];
  for (const [snippet, message] of pagesRequirements) {
    if (!webPagesWorkflow.includes(snippet)) {
      failures.push(`.github/workflows/web-pages.yml: ${message}`);
    }
  }
  if (/^\s*push:\s*$/m.test(webPagesWorkflow)) {
    failures.push(
      ".github/workflows/web-pages.yml: push-triggered source builds are not allowed",
    );
  }

  const releaseRequirements = [
    ["run: node scripts/run-playwright.mjs install --with-deps", "must install Playwright before release preview smoke"],
    ["run: bun run check:bundle:app", "must enforce the app bundle budget before publishing"],
    ["run: bun run test:preview-smoke", "must run production preview smoke before publishing"],
    ["run: cp dist/index.html dist/404.html", "must package the SPA fallback in the release bundle"],
    ["prepare-pages-release-bundle.mjs", "must validate the exact release Web bundle before Pages upload"],
    ["needs.web.result == 'success'", "must prepare Pages only after the release Web bundle succeeds"],
    ["pages-deploy:", "must contain a Pages deployment job"],
    ["needs.host-tools.result == 'success'", "must deploy Pages only after host tools succeed"],
    ["needs.firmware.result == 'success'", "must deploy Pages only after firmware succeeds"],
    ["needs.pages.result == 'success'", "must deploy Pages only after bundle preparation succeeds"],
    ["- pages-deploy", "must block Create GitHub release on Pages deployment"],
    ["needs.pages-deploy.result == 'success'", "must require successful Pages deployment before release creation"],
  ];
  for (const [snippet, message] of releaseRequirements) {
    if (!releaseWorkflow.includes(snippet)) {
      failures.push(`.github/workflows/release.yml: ${message}`);
    }
  }

  return failures;
}

export async function validateCurrentTruthDocs({
  docs = [
    {
      label: "docs/interfaces/network-control.md",
      path: new URL("../../docs/interfaces/network-control.md", import.meta.url),
      requiredSnippets: [
        "- `POST /api/v1/control`",
        "- `GET /api/v1/presets`、`POST /api/v1/presets`（`PUT /api/v1/presets` 为兼容别名）、`POST /api/v1/presets/apply`",
        "- `GET /api/v1/pd`、`POST /api/v1/pd`（`PUT /api/v1/pd` 为兼容别名）",
        "- `GET /api/v1/diagnostics/export`（`GET /api/v1/diagnostics` 仍为兼容别名）",
      ],
      forbiddenSnippets: [
        "SetPoint/LimitProfile/SoftReset/SetEnable 控制闭环",
        "映射 `SetEnable.enable`，协调 `SetPoint` 发送与安全 gating。",
        "- `PUT /api/v1/control`\n    - 更新统一控制真相源，例如输出开关、活动 preset 或 preset 内容。",
        "- `GET/PUT /api/v1/presets`、`POST /api/v1/presets/apply`",
        "- `GET/PUT /api/v1/pd`",
      ],
    },
    {
      label: "docs/interfaces/network-http-api.md",
      path: new URL("../../docs/interfaces/network-http-api.md", import.meta.url),
      requiredSnippets: [
        "GET /api/v1/diagnostics/export",
        "### 3.6 `POST /api/v1/pd`（`PUT /api/v1/pd` 兼容）",
        "### 3.9 `POST /api/v1/presets`（冻结；`PUT` 兼容）",
        "### 3.12 `POST /api/v1/control`（冻结；`PUT` 兼容）",
      ],
      forbiddenSnippets: [
        "实际下发 SetPoint.target_i_ma",
        "### 3.9 `PUT /api/v1/presets`（冻结）",
        "### 3.12 `PUT /api/v1/control`（冻结）",
      ],
    },
    {
      label: "docs/dev-notes/user-calibration.md",
      path: new URL("../../docs/dev-notes/user-calibration.md", import.meta.url),
      forbiddenSnippets: [
        "不会用校准曲线去反算 SetPoint",
        "Web 调用控制 API 下发对应 CC SetPoint",
        "SetPoint（物理量目标）在 G431 侧反向插值得到 Raw 目标",
      ],
    },
    {
      label: "docs/dev-notes/current-sense-opa2365-v4-2.md",
      path: new URL("../../docs/dev-notes/current-sense-opa2365-v4-2.md", import.meta.url),
      forbiddenSnippets: [
        "数字板通过单一 `SetPoint.target_i_ma` 下发**总目标电流** `I_total`",
        "调增 SetPoint 上限与保护阈值",
      ],
    },
    {
      label: "docs/interfaces/uart-link.md",
      path: new URL("../../docs/interfaces/uart-link.md", import.meta.url),
      forbiddenSnippets: [
        "空闲 10 Hz、工作 50–100 Hz 遥测",
        "SET_POINT` (0x22) | `seq`、`target_i_ma`（mA，两通道合计 CC 设定值） | ≈18 B | 50–100 Hz",
        "当前项目在空闲期发送 10 Hz `PING` 作为显式心跳",
      ],
    },
    {
      label: "docs/dev-notes/software.md",
      path: new URL("../../docs/dev-notes/software.md", import.meta.url),
      forbiddenSnippets: [
        "目标：在 ESP32‑S3 与 STM32G431 之间建立稳定的 UART 链路，并基于共享协议 crate（`loadlynx-protocol`）完成 FastStatus 遥测 + SetPoint 控制闭环。",
        "`MSG_SET_POINT`：当前数字侧接收控制请求并回 ACK；",
        "当前链路已实现 `HELLO`、`FAST_STATUS` 与 `SET_POINT + ACK` 的控制闭环。",
      ],
    },
  ],
} = {}) {
  const failures = [];

  for (const doc of docs) {
    const source = await readFile(doc.path, "utf8");
    for (const snippet of doc.requiredSnippets ?? []) {
      if (!source.includes(snippet)) {
        failures.push(`${doc.label}: required current-truth phrase missing: ${JSON.stringify(snippet)}`);
      }
    }
    for (const snippet of doc.forbiddenSnippets ?? []) {
      if (source.includes(snippet)) {
        failures.push(`${doc.label}: forbidden stale control-path phrase present: ${JSON.stringify(snippet)}`);
      }
    }
  }

  return failures;
}

export async function validateHttpSurfaceContracts({
  firmwareNetPath = new URL("../../firmware/digital/src/net.rs", import.meta.url),
  webClientDevicePath = new URL("../../web/src/api/client-device.ts", import.meta.url),
  webClientBackupPath = new URL("../../web/src/api/client-backup.ts", import.meta.url),
} = {}) {
  const failures = [];
  const firmwareNet = await readFile(firmwareNetPath, "utf8");
  const webClientDevice = await readFile(webClientDevicePath, "utf8");
  const webClientBackup = await readFile(webClientBackupPath, "utf8");

  const requiredFirmwareSnippets = [
    '("PUT", "/api/v1/presets") | ("POST", "/api/v1/presets")',
    '("PUT", "/api/v1/control") | ("POST", "/api/v1/control")',
    '("PUT", "/api/v1/pd") | ("POST", "/api/v1/pd")',
    '("GET", "/api/v1/diagnostics") | ("GET", "/api/v1/diagnostics/export")',
  ];

  for (const snippet of requiredFirmwareSnippets) {
    if (!firmwareNet.includes(snippet)) {
      failures.push(
        `firmware/digital/src/net.rs: required HTTP compatibility route missing: ${JSON.stringify(snippet)}`,
      );
    }
  }

  const requiredWebClientPatterns = [
    {
      label: "postPd must keep POST /api/v1/pd as the primary client path",
      pattern:
        /export async function postPd[\s\S]*?httpJsonQueued<PdView>\(baseUrl, "\/api\/v1\/pd", \{\s*method: "POST"/,
    },
    {
      label: "updatePreset must keep POST /api/v1/presets as the primary client path",
      pattern:
        /export async function updatePreset[\s\S]*?httpJsonQueued<Preset>\(baseUrl, "\/api\/v1\/presets", \{\s*method: "POST"/,
    },
    {
      label: "updateControl must keep POST /api/v1/control as the primary client path",
      pattern:
        /export async function updateControl[\s\S]*?httpJsonQueued<ControlView>\(baseUrl, "\/api\/v1\/control", \{\s*method: "POST"/,
    },
  ];

  for (const requirement of requiredWebClientPatterns) {
    if (!requirement.pattern.test(webClientDevice)) {
      failures.push(`web/src/api/client-device.ts: ${requirement.label}`);
    }
  }

  if (!webClientBackup.includes('"/api/v1/diagnostics/export"')) {
    failures.push(
      'web/src/api/client-backup.ts: exportDiagnostics must keep "/api/v1/diagnostics/export" as the primary client path',
    );
  }

  return failures;
}

export async function validateReleasedCliDocs({
  docs = [
    {
      label: "README.md",
      path: new URL("../../README.md", import.meta.url),
    },
    {
      label: "skills/loadlynx-user-operations/SKILL.md",
      path: new URL("../../skills/loadlynx-user-operations/SKILL.md", import.meta.url),
    },
    {
      label: "docs/specs/fhpfk-loadlynx-operational-skills/SPEC.md",
      path: new URL("../../docs/specs/fhpfk-loadlynx-operational-skills/SPEC.md", import.meta.url),
    },
    {
      label: "docs/specs/fhpfk-loadlynx-operational-skills/IMPLEMENTATION.md",
      path: new URL("../../docs/specs/fhpfk-loadlynx-operational-skills/IMPLEMENTATION.md", import.meta.url),
    },
    {
      label: "docs/specs/fhpfk-loadlynx-operational-skills/HISTORY.md",
      path: new URL("../../docs/specs/fhpfk-loadlynx-operational-skills/HISTORY.md", import.meta.url),
    },
  ],
} = {}) {
  const failures = [];
  const joined = [];

  for (const doc of docs) {
    const source = await readFile(doc.path, "utf8");
    joined.push([doc.label, source]);
  }

  const allText = joined.map(([, source]) => source).join("\n");
  const requiredSnippets = [
    "loadlynx wifi show|set|clear",
    "loadlynx pd set",
    "loadlynx cv <target_v_mv>",
    "loadlynx-devd serve",
    "bridge-http",
    "External USB-C Source Validation",
  ];

  for (const snippet of requiredSnippets) {
    if (!allText.includes(snippet)) {
      failures.push(`released CLI docs: required phrase missing: ${JSON.stringify(snippet)}`);
    }
  }

  const forbiddenPatterns = [
    {
      label: "project-specific external DUT name",
      pattern: /\b[Ii]sola[Pp]urr\b/,
    },
    {
      label: "ordinary user CLI daemon URL path",
      pattern: /--devd\s+http/i,
    },
    {
      label: "stale current CLI WiFi absence claim",
      pattern: /current CLI[\s\S]{0,80}(does not implement|lacks|没有|無).*WiFi/i,
    },
    {
      label: "stale no WiFi command claim",
      pattern: /\bno (released )?`?loadlynx wifi/i,
    },
    {
      label: "stale WiFi implementation milestone",
      pattern: /WiFi 配置仍是实现门槛|CLI WiFi 配置仍是实现门槛|没有 WiFi 配置命令/,
    },
  ];

  for (const [label, source] of joined) {
    for (const { label: ruleLabel, pattern } of forbiddenPatterns) {
      if (pattern.test(source)) {
        failures.push(`${label}: forbidden released CLI drift phrase present (${ruleLabel})`);
      }
    }
  }

  return failures;
}

export async function validateReleaseDecisionDocs({
  docs = [
    {
      label: "README.md",
      path: new URL("../../README.md", import.meta.url),
    },
    {
      label: "AGENTS.md",
      path: new URL("../../AGENTS.md", import.meta.url),
    },
    {
      label: "skills/loadlynx-release-decision/SKILL.md",
      path: new URL("../../skills/loadlynx-release-decision/SKILL.md", import.meta.url),
    },
    {
      label: "skills/loadlynx-release-decision/agents/openai.yaml",
      path: new URL("../../skills/loadlynx-release-decision/agents/openai.yaml", import.meta.url),
    },
    {
      label: "skills/loadlynx-developer-operations/SKILL.md",
      path: new URL("../../skills/loadlynx-developer-operations/SKILL.md", import.meta.url),
    },
    {
      label: "docs/specs/dvfnn-pr-label-release-flow/SPEC.md",
      path: new URL("../../docs/specs/dvfnn-pr-label-release-flow/SPEC.md", import.meta.url),
    },
    {
      label: "docs/specs/dvfnn-pr-label-release-flow/IMPLEMENTATION.md",
      path: new URL("../../docs/specs/dvfnn-pr-label-release-flow/IMPLEMENTATION.md", import.meta.url),
    },
    {
      label: "docs/specs/dvfnn-pr-label-release-flow/HISTORY.md",
      path: new URL("../../docs/specs/dvfnn-pr-label-release-flow/HISTORY.md", import.meta.url),
    },
  ],
} = {}) {
  const failures = [];
  const joined = [];

  for (const doc of docs) {
    const source = await readFile(doc.path, "utf8");
    joined.push([doc.label, source]);
  }

  const byLabel = new Map(joined);
  const allText = joined.map(([, source]) => source).join("\n");
  const requiredSnippets = [
    "skills/loadlynx-release-decision/SKILL.md",
    "owner-facing/user-facing operation contract",
    "type:patch` or higher",
    "type:none` is an explicit no-release decision",
    "workflow_dispatch",
    "pr_number=<PR>",
    "v0.5.2",
  ];

  for (const snippet of requiredSnippets) {
    if (!allText.includes(snippet)) {
      failures.push(`release decision docs: required phrase missing: ${JSON.stringify(snippet)}`);
    }
  }

  const requiredDocumentSnippets = [
    {
      label: "README.md",
      snippets: ["skills/loadlynx-release-decision/SKILL.md", "type:patch` or higher"],
    },
    {
      label: "AGENTS.md",
      snippets: ["skills/loadlynx-release-decision/SKILL.md", "type:patch` or higher"],
    },
    {
      label: "skills/loadlynx-developer-operations/SKILL.md",
      snippets: ["skills/loadlynx-release-decision/SKILL.md"],
    },
    {
      label: "skills/loadlynx-release-decision/SKILL.md",
      snippets: [
        "type:none` is an explicit no-release decision",
        "type:patch` or higher",
        "workflow_dispatch",
        "pr_number=<PR>",
      ],
    },
    {
      label: "docs/specs/dvfnn-pr-label-release-flow/SPEC.md",
      snippets: [
        "Release Decision Matrix",
        "skills/loadlynx-release-decision/SKILL.md",
        "type:patch` or higher",
      ],
    },
    {
      label: "docs/specs/dvfnn-pr-label-release-flow/IMPLEMENTATION.md",
      snippets: ["v0.5.2"],
    },
    {
      label: "docs/specs/dvfnn-pr-label-release-flow/HISTORY.md",
      snippets: ["v0.5.2", "pr_number=<PR>"],
    },
  ];

  for (const { label, snippets } of requiredDocumentSnippets) {
    const source = byLabel.get(label) ?? "";
    for (const snippet of snippets) {
      if (!source.includes(snippet)) {
        failures.push(
          `${label}: required release-decision phrase missing: ${JSON.stringify(snippet)}`,
        );
      }
    }
  }

  const skill = byLabel.get("skills/loadlynx-release-decision/SKILL.md") ?? "";
  if (!/^name: loadlynx-release-decision$/m.test(skill)) {
    failures.push("skills/loadlynx-release-decision/SKILL.md: missing skill name frontmatter");
  }
  if (!/description: ".*release labels.*backfill releases.*"/s.test(skill)) {
    failures.push("skills/loadlynx-release-decision/SKILL.md: description must cover labels and backfills");
  }

  const openaiYaml = byLabel.get("skills/loadlynx-release-decision/agents/openai.yaml") ?? "";
  if (!openaiYaml.includes("$loadlynx-release-decision")) {
    failures.push(
      "skills/loadlynx-release-decision/agents/openai.yaml: default_prompt must mention $loadlynx-release-decision",
    );
  }

  const forbiddenPatterns = [
    {
      label: "operation-contract docs-only no-release default",
      pattern: /(?:skill|docs)-only operation contract can stay type:none/i,
    },
  ];

  for (const [label, source] of joined) {
    for (const { label: ruleLabel, pattern } of forbiddenPatterns) {
      if (pattern.test(source) && !label.includes("loadlynx-release-decision")) {
        failures.push(`${label}: forbidden release decision drift phrase present (${ruleLabel})`);
      }
    }
  }

  return failures;
}
