import { afterEach, expect, test, vi } from "vitest";

import { downloadJsonFile } from "./download.ts";

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

test("downloadJsonFile creates a JSON blob download and revokes the URL", () => {
  const click = vi.fn();
  const createElement = vi.fn(() => ({
    href: "",
    download: "",
    click,
  }));
  const setTimeoutSpy = vi.fn((fn: TimerHandler) => {
    if (typeof fn === "function") {
      fn();
    }
    return 0;
  });

  vi.stubGlobal("document", { createElement });
  vi.stubGlobal("window", { setTimeout: setTimeoutSpy });
  const createObjectURL = vi
    .spyOn(URL, "createObjectURL")
    .mockReturnValue("blob:mock");
  const revokeObjectURL = vi
    .spyOn(URL, "revokeObjectURL")
    .mockImplementation(() => {});

  downloadJsonFile("demo.json", { ok: true });

  expect(createElement).toHaveBeenCalledWith("a");
  expect(createObjectURL).toHaveBeenCalledTimes(1);
  const blob = createObjectURL.mock.calls[0]?.[0];
  expect(blob).toBeInstanceOf(Blob);
  expect(click).toHaveBeenCalledTimes(1);
  expect(setTimeoutSpy).toHaveBeenCalledTimes(1);
  expect(revokeObjectURL).toHaveBeenCalledWith("blob:mock");
});
