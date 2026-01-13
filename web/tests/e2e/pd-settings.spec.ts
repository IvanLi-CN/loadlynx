import { expect, test } from "@playwright/test";

test("PD settings page can read/apply and surfaces errors", async ({
  page,
}) => {
  await page.addInitScript(() => {
    const devices = [
      {
        id: "dev-pd",
        name: "Mock PD Device",
        baseUrl: "http://fake-pd",
      },
    ];
    localStorage.setItem("loadlynx.devices", JSON.stringify(devices));
  });

  await page.addInitScript(() => {
    const globalWithState = window as unknown as {
      __pdPostContentTypes: string[];
      __pdState: unknown;
    };
    globalWithState.__pdPostContentTypes = [];

    const errorEnvelope = (status: number, code: string, message: string) => {
      return new Response(
        JSON.stringify({
          error: { code, message, retryable: status >= 500, details: null },
        }),
        { status, headers: { "Content-Type": "application/json" } },
      );
    };

    const identity = {
      device_id: "dev-pd",
      digital_fw_version: "digital 0.1.0 (mock pd)",
      analog_fw_version: "analog 0.1.0 (mock pd)",
      protocol_version: 1,
      uptime_ms: 123_000,
      network: {
        ip: "192.168.0.222",
        mac: "00:11:22:33:44:55",
        hostname: "mock-pd",
      },
      capabilities: {
        cc_supported: true,
        cv_supported: true,
        cp_supported: false,
        presets_supported: true,
        preset_count: 5,
        api_version: "2.0.0",
      },
    };

    let pdState = {
      attached: true,
      contract_mv: 9000,
      contract_ma: 2000,
      fixed_pdos: [
        { pos: 1, mv: 5000, max_ma: 3000 },
        { pos: 2, mv: 9000, max_ma: 3000 },
        { pos: 3, mv: 12000, max_ma: 3000 },
        { pos: 4, mv: 15000, max_ma: 3000 },
        { pos: 5, mv: 20000, max_ma: 1500 },
      ],
      pps_pdos: [{ pos: 3, min_mv: 3300, max_mv: 21000, max_ma: 3000 }],
      saved: {
        mode: "fixed",
        fixed_object_pos: 5,
        pps_object_pos: 3,
        target_mv: 9000,
        i_req_ma: 1500,
      },
      apply: { pending: false, last: { code: "ok", at_ms: 123456 } },
    };

    const origFetch = window.fetch.bind(window);
    window.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = typeof input === "string" ? input : input.toString();
      if (!url.includes("fake-pd")) {
        return origFetch(input, init);
      }

      if (url.includes("/api/v1/identity")) {
        return new Response(JSON.stringify(identity), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        });
      }

      if (url.includes("/api/v1/calibration/mode")) {
        return new Response(JSON.stringify({ ok: true }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        });
      }

      if (url.includes("/api/v1/pd") && (!init || init.method === "GET")) {
        return new Response(JSON.stringify(pdState), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        });
      }

      if (url.includes("/api/v1/pd") && init?.method === "POST") {
        const headers = (init.headers ?? {}) as Record<string, string>;
        const contentType =
          headers["Content-Type"] ?? headers["content-type"] ?? "";
        globalWithState.__pdPostContentTypes.push(contentType);

        const rawBody = typeof init.body === "string" ? init.body : "";
        let parsed: unknown = null;
        try {
          parsed = JSON.parse(rawBody) as unknown;
        } catch {
          return errorEnvelope(400, "INVALID_REQUEST", "invalid json");
        }

        if (!parsed || typeof parsed !== "object") {
          return errorEnvelope(400, "INVALID_REQUEST", "invalid request");
        }

        const payload = parsed as Record<string, unknown>;
        const mode = typeof payload.mode === "string" ? payload.mode : null;
        const objectPos =
          typeof payload.object_pos === "number" ? payload.object_pos : null;
        const iReqMa =
          typeof payload.i_req_ma === "number" ? payload.i_req_ma : null;

        if (iReqMa === 1450) {
          return errorEnvelope(
            422,
            "LIMIT_VIOLATION",
            "Ireq exceeds Imax for selected PDO",
          );
        }

        // Apply OK: update saved + contract.
        if (mode === "fixed") {
          if (objectPos == null || iReqMa == null) {
            return errorEnvelope(400, "INVALID_REQUEST", "missing fields");
          }
          const pdo = pdState.fixed_pdos.find(
            (entry) => entry.pos === objectPos,
          );
          if (!pdo) {
            return errorEnvelope(422, "LIMIT_VIOLATION", "pdo missing");
          }
          pdState = {
            ...pdState,
            contract_mv: pdo.mv,
            contract_ma: iReqMa,
            saved: {
              ...pdState.saved,
              mode: "fixed",
              fixed_object_pos: objectPos,
              i_req_ma: iReqMa,
            },
            apply: { pending: false, last: { code: "ok", at_ms: 234567 } },
          };
        }

        return new Response(JSON.stringify(pdState), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        });
      }

      return errorEnvelope(404, "UNSUPPORTED_OPERATION", "not found");
    };
  });

  await page.goto("/dev-pd/pd");

  await expect(page.locator("h2")).toContainText("USB‑PD Settings");
  await expect(page.locator("text=ATTACHED")).toBeVisible();

  await page.getByRole("button", { name: "Fixed" }).click();

  // Select PDO #5, set Ireq and apply.
  await page
    .locator("button")
    .filter({ hasText: "20.0 V" })
    .filter({ hasText: "5" })
    .first()
    .click();
  const fixedIreqInput = page.getByRole("spinbutton").first();
  await fixedIreqInput.fill("1500");
  await page.getByRole("button", { name: "Apply" }).click();
  await expect(page.locator(".alert-success")).toBeVisible();

  // Force an error response and verify UI surfaces it without wiping input.
  await fixedIreqInput.fill("1450");
  await page.getByRole("button", { name: "Apply" }).click();
  await expect(page.locator(".alert-error")).toBeVisible();
  await expect(page.locator(".alert-error")).toContainText("LIMIT_VIOLATION");
  await expect(fixedIreqInput).toHaveValue("1450");

  // Ensure we used POST + Content-Type: text/plain (no private network preflight).
  const contentTypes = await page.evaluate(() => {
    const win = window as unknown as { __pdPostContentTypes: string[] };
    return win.__pdPostContentTypes ?? [];
  });
  expect(contentTypes.length).toBeGreaterThan(0);
  expect(contentTypes.every((ct) => ct.includes("text/plain"))).toBeTruthy();
});
