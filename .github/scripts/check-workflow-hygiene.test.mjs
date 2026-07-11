#!/usr/bin/env node
import assert from "node:assert/strict";
import {
  parseWorkflowMetadata,
  validateCurrentTruthDocs,
  validateHttpSurfaceContracts,
  validateReleaseDecisionDocs,
  validateReleasePagesContracts,
  validateReleasedCliDocs,
  validateWebToolingContracts,
  validateWorkflowHygiene,
} from "./check-quality-gates-lib.mjs";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
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
    setupNodeUsesVersionFile: [],
    setupNodeUsesInlineVersion: [],
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
  setupNodeUsesVersionFile: [],
  setupNodeUsesInlineVersion: [],
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
  validateWorkflowHygiene({
    workflows: [
      {
        ...healthyWorkflow,
        jobs: [
          {
            id: "notify",
            name: null,
            hasTimeoutMinutes: true,
            isReusableWorkflow: true,
          },
        ],
      },
    ],
  }),
  ['workflow healthy.yml reusable job "notify": timeout-minutes is not supported'],
);

assert.deepEqual(
  parseWorkflowMetadata(
    `name: Reusable
permissions: {}

jobs:
  notify:
    uses: owner/repo/.github/workflows/notify.yml@main
`,
    "reusable.yml",
  ).jobs,
  [
    {
      id: "notify",
      name: null,
      hasTimeoutMinutes: false,
      isReusableWorkflow: true,
    },
  ],
);

assert.deepEqual(
  parseWorkflowMetadata(
    `name: Node
permissions:
  contents: read

jobs:
  build:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - uses: actions/setup-node@v4
        with:
          node-version-file: ".node-version"
`,
    "node.yml",
  ),
  {
    fileName: "node.yml",
    name: "Node",
    hasPermissions: true,
    setupNodeUsesVersionFile: [".node-version"],
    setupNodeUsesInlineVersion: [],
    setupBunUsesVersionFile: [],
    setupBunUsesInlineVersion: [],
    jobs: [
      { id: "build", name: null, hasTimeoutMinutes: true },
    ],
  },
);

assert.deepEqual(
  parseWorkflowMetadata(
    `name: Named Node
permissions:
  contents: read

jobs:
  build:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version-file: ".node-version"
`,
    "named-node.yml",
  ),
  {
    fileName: "named-node.yml",
    name: "Named Node",
    hasPermissions: true,
    setupNodeUsesVersionFile: [".node-version"],
    setupNodeUsesInlineVersion: [],
    setupBunUsesVersionFile: [],
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
        fileName: "inline-node.yml",
        setupNodeUsesVersionFile: [],
        setupNodeUsesInlineVersion: ["20"],
      },
    ],
  }),
  [
    "workflow inline-node.yml: setup-node must use node-version-file=.node-version instead of inline node-version",
  ],
);

assert.deepEqual(
  validateWorkflowHygiene({
    workflows: [
      {
        ...healthyWorkflow,
        fileName: "wrong-node-file.yml",
        setupNodeUsesVersionFile: [".nvmrc"],
        setupNodeUsesInlineVersion: [],
      },
    ],
  }),
  ['workflow wrong-node-file.yml: setup-node node-version-file must be ".node-version"'],
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
    setupNodeUsesVersionFile: [],
    setupNodeUsesInlineVersion: [],
    setupBunUsesVersionFile: [".bun-version"],
    setupBunUsesInlineVersion: [],
    jobs: [
      { id: "build", name: null, hasTimeoutMinutes: true },
    ],
  },
);

assert.deepEqual(
  parseWorkflowMetadata(
    `name: Named Bun
permissions:
  contents: read

jobs:
  build:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - name: Setup Bun
        uses: oven-sh/setup-bun@v2
        with:
          bun-version-file: ".bun-version"
`,
    "named-bun.yml",
  ),
  {
    fileName: "named-bun.yml",
    name: "Named Bun",
    hasPermissions: true,
    setupNodeUsesVersionFile: [],
    setupNodeUsesInlineVersion: [],
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
  const webPackageLockPath = join(tempDir, "package-lock.json");
  const workflowPath = join(tempDir, "web-check.yml");
  const webPagesWorkflowPath = join(tempDir, "web-pages.yml");
  const releaseWorkflowPath = join(tempDir, "release.yml");
  await writeFile(
    webPackagePath,
    JSON.stringify(
      {
        scripts: {
          "test:e2e": "node scripts/run-playwright.mjs test",
          "test:preview-smoke":
            "node scripts/run-playwright.mjs test --config playwright.preview.config.ts",
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
  await writeFile(
    webPagesWorkflowPath,
    `name: Web Pages
jobs:
  build:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps: []
`,
  );
  await writeFile(
    releaseWorkflowPath,
    `name: Release
jobs:
  web:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps: []
`,
  );

  assert.deepEqual(
    await validateWebToolingContracts({
      webPackageJsonPath: new URL(`file://${webPackagePath}`),
      webPackageLockPath: new URL(`file://${webPackageLockPath}`),
      webCheckWorkflowPath: new URL(`file://${workflowPath}`),
      webPagesWorkflowPath: new URL(`file://${webPagesWorkflowPath}`),
      releaseWorkflowPath: new URL(`file://${releaseWorkflowPath}`),
    }),
    [
      ".github/workflows/web-check.yml: Install Playwright browsers step must run node scripts/run-playwright.mjs install --with-deps",
      ".github/workflows/web-check.yml: Production preview smoke step must run bun run test:preview-smoke",
      ".github/workflows/web-check.yml: web install step must reject web/package-lock.json and run bun ci",
      ".github/workflows/release.yml: web install step must reject web/package-lock.json and run bun ci",
    ],
  );

  await writeFile(webPackageLockPath, "{}\n");

  assert.deepEqual(
    await validateWebToolingContracts({
      webPackageJsonPath: new URL(`file://${webPackagePath}`),
      webPackageLockPath: new URL(`file://${webPackageLockPath}`),
      webCheckWorkflowPath: new URL(`file://${workflowPath}`),
      webPagesWorkflowPath: new URL(`file://${webPagesWorkflowPath}`),
      releaseWorkflowPath: new URL(`file://${releaseWorkflowPath}`),
    }),
    [
      ".github/workflows/web-check.yml: Install Playwright browsers step must run node scripts/run-playwright.mjs install --with-deps",
      ".github/workflows/web-check.yml: Production preview smoke step must run bun run test:preview-smoke",
      "web/package-lock.json must not exist; use web/bun.lock as the only lockfile",
      ".github/workflows/web-check.yml: web install step must reject web/package-lock.json and run bun ci",
      ".github/workflows/release.yml: web install step must reject web/package-lock.json and run bun ci",
    ],
  );

  await rm(webPackageLockPath, { force: true });

  await writeFile(
    webPackagePath,
    JSON.stringify(
      {
        scripts: {
          "test:e2e": "playwright test",
          "test:preview-smoke": "playwright test --config playwright.preview.config.ts",
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
      webPackageLockPath: new URL(`file://${webPackageLockPath}`),
      webCheckWorkflowPath: new URL(`file://${workflowPath}`),
      webPagesWorkflowPath: new URL(`file://${webPagesWorkflowPath}`),
      releaseWorkflowPath: new URL(`file://${releaseWorkflowPath}`),
    }),
    [
      'web/package.json: scripts["test:e2e"] must be "node scripts/run-playwright.mjs test"',
      'web/package.json: scripts["test:preview-smoke"] must be "node scripts/run-playwright.mjs test --config playwright.preview.config.ts"',
      'web/package.json: scripts["test:e2e:ui"] must be "node scripts/run-playwright.mjs test --ui"',
      ".github/workflows/web-check.yml: Install Playwright browsers step must run node scripts/run-playwright.mjs install --with-deps",
      ".github/workflows/web-check.yml: Production preview smoke step must run bun run test:preview-smoke",
      ".github/workflows/web-check.yml: web install step must reject web/package-lock.json and run bun ci",
      ".github/workflows/release.yml: web install step must reject web/package-lock.json and run bun ci",
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
      - name: Production preview smoke
        run: bun run test:preview-smoke
      - name: Install dependencies
        run: |
          if [ -f package-lock.json ]; then
            echo "ERROR: web/package-lock.json is not supported. Use Bun and web/bun.lock only." >&2
            exit 1
          fi
          bun ci
`,
  );
  await writeFile(
    webPagesWorkflowPath,
    `name: Web Pages
jobs:
  build:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - name: Install Playwright browsers
        run: node scripts/run-playwright.mjs install --with-deps
      - name: Production preview smoke
        run: bun run test:preview-smoke
      - name: Install dependencies
        run: |
          if [ -f package-lock.json ]; then
            echo "ERROR: web/package-lock.json is not supported. Use Bun and web/bun.lock only." >&2
            exit 1
          fi
          bun ci
`,
  );
  await writeFile(
    releaseWorkflowPath,
    `name: Release
jobs:
  web:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - name: Install dependencies
        run: |
          if [ -f package-lock.json ]; then
            echo "ERROR: web/package-lock.json is not supported. Use Bun and web/bun.lock only." >&2
            exit 1
          fi
          bun ci
`,
  );

  await writeFile(
    webPackagePath,
    JSON.stringify(
      {
        scripts: {
          "test:e2e": "node scripts/run-playwright.mjs test",
          "test:preview-smoke":
            "node scripts/run-playwright.mjs test --config playwright.preview.config.ts",
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
      webPackageLockPath: new URL(`file://${webPackageLockPath}`),
      webCheckWorkflowPath: new URL(`file://${workflowPath}`),
      webPagesWorkflowPath: new URL(`file://${webPagesWorkflowPath}`),
      releaseWorkflowPath: new URL(`file://${releaseWorkflowPath}`),
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
      - name: Install dependencies
        run: |
          if [ -f package-lock.json ]; then
            echo "ERROR: web/package-lock.json is not supported. Use Bun and web/bun.lock only." >&2
            exit 1
          fi
          bun ci
`,
  );

  assert.deepEqual(
    await validateWebToolingContracts({
      webPackageJsonPath: new URL(`file://${webPackagePath}`),
      webPackageLockPath: new URL(`file://${webPackageLockPath}`),
      webCheckWorkflowPath: new URL(`file://${workflowPath}`),
      webPagesWorkflowPath: new URL(`file://${webPagesWorkflowPath}`),
      releaseWorkflowPath: new URL(`file://${releaseWorkflowPath}`),
    }),
    [
      ".github/workflows/web-check.yml: Install Playwright browsers step must run node scripts/run-playwright.mjs install --with-deps",
      ".github/workflows/web-check.yml: bunx playwright install is not allowed; use node scripts/run-playwright.mjs install --with-deps",
      ".github/workflows/web-check.yml: Production preview smoke step must run bun run test:preview-smoke",
    ],
  );

  await writeFile(
    webPagesWorkflowPath,
    `name: Web Pages
jobs:
  build:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - name: Install dependencies
        run: bun ci
`,
  );

  assert.deepEqual(
    await validateWebToolingContracts({
      webPackageJsonPath: new URL(`file://${webPackagePath}`),
      webPackageLockPath: new URL(`file://${webPackageLockPath}`),
      webCheckWorkflowPath: new URL(`file://${workflowPath}`),
      webPagesWorkflowPath: new URL(`file://${webPagesWorkflowPath}`),
      releaseWorkflowPath: new URL(`file://${releaseWorkflowPath}`),
    }),
    [
      ".github/workflows/web-check.yml: Install Playwright browsers step must run node scripts/run-playwright.mjs install --with-deps",
      ".github/workflows/web-check.yml: bunx playwright install is not allowed; use node scripts/run-playwright.mjs install --with-deps",
      ".github/workflows/web-check.yml: Production preview smoke step must run bun run test:preview-smoke",
    ],
  );
} finally {
  await rm(tempDir, { recursive: true, force: true });
}

{
  const tempDir = await mkdtemp(join(tmpdir(), "loadlynx-release-pages-contract-"));
  try {
    const pagesPath = join(tempDir, "web-pages.yml");
    const releasePath = join(tempDir, "release.yml");
    await writeFile(
      pagesPath,
      `name: Pages
on:
  workflow_dispatch:
    inputs:
      release_tag:
        required: true
jobs:
  prepare:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - run: gh release download "$tag"
      - run: node .github/scripts/prepare-pages-release-bundle.mjs
      - uses: actions/upload-pages-artifact@v3
        with:
          path: dist/pages
`,
    );
    await writeFile(
      releasePath,
      `name: Release
jobs:
  web:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - run: node scripts/run-playwright.mjs install --with-deps
      - run: bun run check:bundle:app
      - run: bun run test:preview-smoke
      - run: cp dist/index.html dist/404.html
      - run: node .github/scripts/prepare-pages-release-bundle.mjs
  pages-deploy:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    if: needs.pages.result == 'success'
  pages:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    if: needs.web.result == 'success'
  release:
    needs:
      - pages-deploy
    if: needs.pages-deploy.result == 'success'
`,
    );

    assert.deepEqual(
      await validateReleasePagesContracts({
        webPagesWorkflowPath: new URL(`file://${pagesPath}`),
        releaseWorkflowPath: new URL(`file://${releasePath}`),
      }),
      [],
    );

    await writeFile(
      pagesPath,
      `${await readFile(pagesPath, "utf8")}  push:\n`,
    );
    assert.deepEqual(
      await validateReleasePagesContracts({
        webPagesWorkflowPath: new URL(`file://${pagesPath}`),
        releaseWorkflowPath: new URL(`file://${releasePath}`),
      }),
      [
        ".github/workflows/web-pages.yml: push-triggered source builds are not allowed",
      ],
    );
  } finally {
    await rm(tempDir, { recursive: true, force: true });
  }
}

{
  const tempDir = await mkdtemp(join(tmpdir(), "check-current-truth-docs-"));

  try {
    const networkControlPath = join(tempDir, "network-control.md");
    const networkHttpApiPath = join(tempDir, "network-http-api.md");
    const userCalibrationPath = join(tempDir, "user-calibration.md");
    const currentSensePath = join(tempDir, "current-sense.md");
    const uartLinkPath = join(tempDir, "uart-link.md");
    const softwareNotesPath = join(tempDir, "software.md");

    await writeFile(
      networkControlPath,
      "current control path is SetMode and output_enabled",
    );
    await writeFile(
      networkHttpApiPath,
      "effective_i_ma is reflected through unified control path",
    );
    await writeFile(
      userCalibrationPath,
      "calibration flow uses physical targets and unified UART TX path",
    );
    await writeFile(
      currentSensePath,
      "SetMode snapshot provides total target current in the active control path",
    );
    await writeFile(
      uartLinkPath,
      "current implementation uses event-driven SetMode and implicit heartbeat",
    );
    await writeFile(
      softwareNotesPath,
      "current digital/analog control path uses FastStatus plus SetMode and compatibility-only SetPoint",
    );

    assert.deepEqual(
      await validateCurrentTruthDocs({
        docs: [
          {
            label: "docs/interfaces/network-control.md",
            path: new URL(`file://${networkControlPath}`),
            requiredSnippets: ["current control path is SetMode and output_enabled"],
            forbiddenSnippets: [
              "SetPoint/LimitProfile/SoftReset/SetEnable 控制闭环",
              "- `PUT /api/v1/control`\n    - 更新统一控制真相源，例如输出开关、活动 preset 或 preset 内容。",
            ],
          },
          {
            label: "docs/interfaces/network-http-api.md",
            path: new URL(`file://${networkHttpApiPath}`),
            requiredSnippets: [
              "effective_i_ma is reflected through unified control path",
            ],
            forbiddenSnippets: [
              "实际下发 SetPoint.target_i_ma",
              "### 3.12 `PUT /api/v1/control`（冻结）",
            ],
          },
          {
            label: "docs/dev-notes/user-calibration.md",
            path: new URL(`file://${userCalibrationPath}`),
            forbiddenSnippets: ["Web 调用控制 API 下发对应 CC SetPoint"],
          },
          {
            label: "docs/dev-notes/current-sense-opa2365-v4-2.md",
            path: new URL(`file://${currentSensePath}`),
            forbiddenSnippets: ["调增 SetPoint 上限与保护阈值"],
          },
          {
            label: "docs/interfaces/uart-link.md",
            path: new URL(`file://${uartLinkPath}`),
            forbiddenSnippets: [
              "空闲 10 Hz、工作 50–100 Hz 遥测",
            ],
          },
          {
            label: "docs/dev-notes/software.md",
            path: new URL(`file://${softwareNotesPath}`),
            forbiddenSnippets: [
              "当前链路已实现 `HELLO`、`FAST_STATUS` 与 `SET_POINT + ACK` 的控制闭环。",
            ],
          },
        ],
      }),
      [],
    );

    await writeFile(
      networkHttpApiPath,
      "### 3.12 `PUT /api/v1/control`（冻结）",
    );

    assert.deepEqual(
      await validateCurrentTruthDocs({
        docs: [
          {
            label: "docs/interfaces/network-http-api.md",
            path: new URL(`file://${networkHttpApiPath}`),
            forbiddenSnippets: ["### 3.12 `PUT /api/v1/control`（冻结）"],
          },
        ],
      }),
      [
        'docs/interfaces/network-http-api.md: forbidden stale control-path phrase present: "### 3.12 `PUT /api/v1/control`（冻结）"',
      ],
    );

    await writeFile(
      uartLinkPath,
      "当前项目在空闲期发送 10 Hz `PING` 作为显式心跳。",
    );

    assert.deepEqual(
      await validateCurrentTruthDocs({
        docs: [
          {
            label: "docs/interfaces/uart-link.md",
            path: new URL(`file://${uartLinkPath}`),
            forbiddenSnippets: [
              "当前项目在空闲期发送 10 Hz `PING` 作为显式心跳",
            ],
          },
        ],
      }),
      [
        'docs/interfaces/uart-link.md: forbidden stale control-path phrase present: "当前项目在空闲期发送 10 Hz `PING` 作为显式心跳"',
      ],
    );

    await writeFile(
      networkControlPath,
      "- `GET/PUT /api/v1/pd`",
    );

    assert.deepEqual(
      await validateCurrentTruthDocs({
        docs: [
          {
            label: "docs/interfaces/network-control.md",
            path: new URL(`file://${networkControlPath}`),
            forbiddenSnippets: ["- `GET/PUT /api/v1/pd`"],
          },
        ],
      }),
      [
        'docs/interfaces/network-control.md: forbidden stale control-path phrase present: "- `GET/PUT /api/v1/pd`"',
      ],
    );

    await writeFile(networkControlPath, "missing required current truth");

    assert.deepEqual(
      await validateCurrentTruthDocs({
        docs: [
          {
            label: "docs/interfaces/network-control.md",
            path: new URL(`file://${networkControlPath}`),
            requiredSnippets: ["- `POST /api/v1/control`"],
          },
        ],
      }),
      [
        'docs/interfaces/network-control.md: required current-truth phrase missing: "- `POST /api/v1/control`"',
      ],
    );
  } finally {
    await rm(tempDir, { recursive: true, force: true });
  }
}

{
  const tempDir = await mkdtemp(join(tmpdir(), "check-http-surface-contracts-"));

  try {
    const firmwareNetPath = join(tempDir, "net.rs");
    const webClientDevicePath = join(tempDir, "client-device.ts");
    const webClientBackupPath = join(tempDir, "client-backup.ts");

    await writeFile(
      firmwareNetPath,
      [
        '("PUT", "/api/v1/presets") | ("POST", "/api/v1/presets")',
        '("PUT", "/api/v1/control") | ("POST", "/api/v1/control")',
        '("PUT", "/api/v1/pd") | ("POST", "/api/v1/pd")',
        '("GET", "/api/v1/diagnostics") | ("GET", "/api/v1/diagnostics/export")',
      ].join("\n"),
    );
    await writeFile(
      webClientDevicePath,
      `export async function postPd(baseUrl: string, payload: PdUpdateRequest): Promise<PdView> {
  return httpJsonQueued<PdView>(baseUrl, "/api/v1/pd", {
    method: "POST",
  });
}
export async function updatePreset(baseUrl: string, payload: Preset): Promise<Preset> {
  return httpJsonQueued<Preset>(baseUrl, "/api/v1/presets", {
    method: "POST",
  });
}
export async function updateControl(baseUrl: string, payload: ControlUpdateRequest): Promise<ControlView> {
  return httpJsonQueued<ControlView>(baseUrl, "/api/v1/control", {
    method: "POST",
  });
}`,
    );
    await writeFile(
      webClientBackupPath,
      `export async function exportDiagnostics(baseUrl: string): Promise<DiagnosticsExport> {
  return httpJsonQueued<DiagnosticsExport>(baseUrl, "/api/v1/diagnostics/export");
}`,
    );

    assert.deepEqual(
      await validateHttpSurfaceContracts({
        firmwareNetPath: new URL(`file://${firmwareNetPath}`),
        webClientDevicePath: new URL(`file://${webClientDevicePath}`),
        webClientBackupPath: new URL(`file://${webClientBackupPath}`),
      }),
      [],
    );

    await writeFile(
      webClientBackupPath,
      `export async function exportDiagnostics(baseUrl: string): Promise<DiagnosticsExport> {
  return httpJsonQueued<DiagnosticsExport>(baseUrl, "/api/v1/diagnostics");
}`,
    );

    assert.deepEqual(
      await validateHttpSurfaceContracts({
        firmwareNetPath: new URL(`file://${firmwareNetPath}`),
        webClientDevicePath: new URL(`file://${webClientDevicePath}`),
        webClientBackupPath: new URL(`file://${webClientBackupPath}`),
      }),
      [
        'web/src/api/client-backup.ts: exportDiagnostics must keep "/api/v1/diagnostics/export" as the primary client path',
      ],
    );
  } finally {
    await rm(tempDir, { recursive: true, force: true });
  }
}

{
  const tempDir = await mkdtemp(join(tmpdir(), "check-released-cli-docs-"));

  try {
    const readmePath = join(tempDir, "README.md");
    const userSkillPath = join(tempDir, "SKILL.md");
    const specPath = join(tempDir, "SPEC.md");
    const implementationPath = join(tempDir, "IMPLEMENTATION.md");
    const historyPath = join(tempDir, "HISTORY.md");

    await writeFile(
      readmePath,
      [
        "Current stable host tools expose loadlynx wifi show|set|clear.",
        "Use loadlynx pd set for PD stimulus.",
        "Use loadlynx cv <target_v_mv> for voltage-clamp stimulus.",
        "The local daemon command is loadlynx-devd serve.",
        "The browser/debug bridge is bridge-http.",
        "## External USB-C Source Validation",
      ].join("\n"),
    );
    await writeFile(userSkillPath, "Released user skill uses saved devices.\n");
    await writeFile(specPath, "External DUT diagnostics are the primary verdict.\n");
    await writeFile(implementationPath, "Release v0.5.1 docs are aligned.\n");
    await writeFile(historyPath, "No project-specific external DUT name is embedded.\n");

    const docs = [
      { label: "README.md", path: new URL(`file://${readmePath}`) },
      { label: "SKILL.md", path: new URL(`file://${userSkillPath}`) },
      { label: "SPEC.md", path: new URL(`file://${specPath}`) },
      { label: "IMPLEMENTATION.md", path: new URL(`file://${implementationPath}`) },
      { label: "HISTORY.md", path: new URL(`file://${historyPath}`) },
    ];

    assert.deepEqual(await validateReleasedCliDocs({ docs }), []);

    await writeFile(
      userSkillPath,
      [
        `Do not use --devd ${"http://"}127.0.0.1:30180.`,
        `The current CLI does not ${"implement"} WiFi.`,
        `${"Isola"}${"Purr"}-specific flow.`,
      ].join("\n"),
    );

    assert.deepEqual(await validateReleasedCliDocs({ docs }), [
      "SKILL.md: forbidden released CLI drift phrase present (project-specific external DUT name)",
      "SKILL.md: forbidden released CLI drift phrase present (ordinary user CLI daemon URL path)",
      "SKILL.md: forbidden released CLI drift phrase present (stale current CLI WiFi absence claim)",
    ]);
  } finally {
    await rm(tempDir, { recursive: true, force: true });
  }
}

{
  const tempDir = await mkdtemp(join(tmpdir(), "check-release-decision-docs-"));

  try {
    const readmePath = join(tempDir, "README.md");
    const agentsPath = join(tempDir, "AGENTS.md");
    const skillPath = join(tempDir, "SKILL.md");
    const skillYamlPath = join(tempDir, "openai.yaml");
    const developerSkillPath = join(tempDir, "DEVELOPER_SKILL.md");
    const specPath = join(tempDir, "SPEC.md");
    const implementationPath = join(tempDir, "IMPLEMENTATION.md");
    const historyPath = join(tempDir, "HISTORY.md");

    await writeFile(
      readmePath,
      "See skills/loadlynx-release-decision/SKILL.md; use type:patch` or higher.\n",
    );
    await writeFile(
      agentsPath,
      "Use skills/loadlynx-release-decision/SKILL.md and type:patch` or higher.\n",
    );
    await writeFile(
      skillPath,
      `---
name: loadlynx-release-decision
description: "Decide LoadLynx release labels and backfill releases from merged PRs."
---
type:none\` is an explicit no-release decision.
owner-facing/user-facing operation contract changes require type:patch\` or higher.
Use workflow_dispatch with pr_number=<PR>.
`,
    );
    await writeFile(
      skillYamlPath,
      `interface:
  display_name: "LoadLynx Release Decision"
  short_description: "Choose release labels and backfill releases."
  default_prompt: "Use $loadlynx-release-decision to decide labels."
`,
    );
    await writeFile(developerSkillPath, "Use skills/loadlynx-release-decision/SKILL.md.\n");
    await writeFile(
      specPath,
      "Release Decision Matrix: skills/loadlynx-release-decision/SKILL.md says owner-facing/user-facing operation contract uses type:patch` or higher.\n",
    );
    await writeFile(implementationPath, "Backfill produced v0.5.2.\n");
    await writeFile(historyPath, "workflow_dispatch pr_number=<PR> is the backfill path for v0.5.2.\n");

    const docs = [
      { label: "README.md", path: new URL(`file://${readmePath}`) },
      { label: "AGENTS.md", path: new URL(`file://${agentsPath}`) },
      {
        label: "skills/loadlynx-release-decision/SKILL.md",
        path: new URL(`file://${skillPath}`),
      },
      {
        label: "skills/loadlynx-release-decision/agents/openai.yaml",
        path: new URL(`file://${skillYamlPath}`),
      },
      {
        label: "skills/loadlynx-developer-operations/SKILL.md",
        path: new URL(`file://${developerSkillPath}`),
      },
      {
        label: "docs/specs/dvfnn-pr-label-release-flow/SPEC.md",
        path: new URL(`file://${specPath}`),
      },
      {
        label: "docs/specs/dvfnn-pr-label-release-flow/IMPLEMENTATION.md",
        path: new URL(`file://${implementationPath}`),
      },
      {
        label: "docs/specs/dvfnn-pr-label-release-flow/HISTORY.md",
        path: new URL(`file://${historyPath}`),
      },
    ];

    assert.deepEqual(await validateReleaseDecisionDocs({ docs }), []);

    await writeFile(agentsPath, "Use type:patch` or higher.\n");

    assert.deepEqual(await validateReleaseDecisionDocs({ docs }), [
      'AGENTS.md: required release-decision phrase missing: "skills/loadlynx-release-decision/SKILL.md"',
    ]);

    await writeFile(
      agentsPath,
      "Use skills/loadlynx-release-decision/SKILL.md; docs-only operation contract can stay type:none.\n",
    );
    await writeFile(skillYamlPath, 'interface:\n  default_prompt: "Use release decision."\n');

    assert.deepEqual(await validateReleaseDecisionDocs({ docs }), [
      'AGENTS.md: required release-decision phrase missing: "type:patch` or higher"',
      "skills/loadlynx-release-decision/agents/openai.yaml: default_prompt must mention $loadlynx-release-decision",
      "AGENTS.md: forbidden release decision drift phrase present (operation-contract docs-only no-release default)",
    ]);
  } finally {
    await rm(tempDir, { recursive: true, force: true });
  }
}

console.log("workflow hygiene tests passed");
