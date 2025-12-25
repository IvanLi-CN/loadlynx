import type { Decorator } from "@storybook/react";
import type { Preview } from "@storybook/react-vite";
import { INITIAL_VIEWPORTS } from "storybook/viewport";

import "../src/index.css";
import { BreakpointRulerOverlay } from "../src/stories/devtools/BreakpointRulerOverlay";

globalThis.__LOADLYNX_STORYBOOK__ = true;

const LOADLYNX_VIEWPORTS = {
  loadlynxSmall: {
    name: "LoadLynx / Small (375)",
    styles: { width: "375px", height: "800px" },
    type: "mobile",
  },
  loadlynxMedium: {
    name: "LoadLynx / Medium (900)",
    styles: { width: "900px", height: "800px" },
    type: "tablet",
  },
  loadlynxLarge: {
    name: "LoadLynx / Large (1200)",
    styles: { width: "1200px", height: "800px" },
    type: "desktop",
  },
  loadlynxBp768: {
    name: "LoadLynx / Breakpoint (768)",
    styles: { width: "768px", height: "800px" },
    type: "tablet",
  },
  loadlynxBp1024: {
    name: "LoadLynx / Breakpoint (1024)",
    styles: { width: "1024px", height: "800px" },
    type: "desktop",
  },
} as const;

const withDarkTheme: Decorator = (Story) => {
  document.documentElement.setAttribute("data-theme", "dark");
  document.body.classList.add(
    "bg-base-100",
    "text-base-content",
    "antialiased",
  );

  return (
    <div className="min-h-screen bg-base-100 p-0 md:p-6 text-base-content antialiased">
      <BreakpointRulerOverlay />
      <Story />
    </div>
  );
};

const preview: Preview = {
  decorators: [withDarkTheme],
  parameters: {
    viewport: {
      options: {
        ...LOADLYNX_VIEWPORTS,
        ...INITIAL_VIEWPORTS,
      },
    },
  },
};

export default preview;
