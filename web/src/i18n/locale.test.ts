import { describe, expect, it, vi } from "vitest";

import { resolveInitialLocale } from "./locale";

describe("resolveInitialLocale", () => {
  it("does not read persisted locale in Storybook", () => {
    const readStoredLocale = vi.fn(() => "en");

    expect(resolveInitialLocale(true, readStoredLocale)).toBe("zh-CN");
    expect(readStoredLocale).not.toHaveBeenCalled();
  });

  it("uses persisted locale in the application", () => {
    expect(resolveInitialLocale(false, () => "en")).toBe("en");
  });
});
