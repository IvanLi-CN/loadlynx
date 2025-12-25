import type { TestRunnerConfig } from "@storybook/test-runner";
import { getStoryContext } from "@storybook/test-runner";

const DEFAULT_VIEWPORT_SIZE = { width: 1280, height: 720 };

type ViewportOption = {
  styles?: {
    width?: string;
    height?: string;
  };
};

function parsePx(value: string | undefined): number | null {
  if (!value) return null;
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : null;
}

const config: TestRunnerConfig = {
  async preVisit(page, story) {
    const context = await getStoryContext(page, story);

    const viewportGlobal = context.storyGlobals?.viewport as
      | { value?: string; isRotated?: boolean }
      | string
      | undefined;

    const viewportName =
      typeof viewportGlobal === "string"
        ? viewportGlobal
        : viewportGlobal?.value;
    const isRotated =
      typeof viewportGlobal === "string" ? false : !!viewportGlobal?.isRotated;

    const viewportOptions = (context.parameters as { viewport?: unknown })
      ?.viewport as { options?: Record<string, ViewportOption> } | undefined;

    const option = viewportName
      ? viewportOptions?.options?.[viewportName]
      : undefined;
    const width = parsePx(option?.styles?.width);
    const height = parsePx(option?.styles?.height);

    if (width && height) {
      page.setViewportSize(
        isRotated ? { width: height, height: width } : { width, height },
      );
      return;
    }

    page.setViewportSize(DEFAULT_VIEWPORT_SIZE);
  },
};

export default config;
