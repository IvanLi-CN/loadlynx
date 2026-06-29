import type { Decorator } from "@storybook/react";
import type { Preview } from "@storybook/react-vite";
import { INITIAL_VIEWPORTS } from "storybook/viewport";

import "../src/index.css";
import { i18next } from "../src/i18n/index.ts";
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
    name: "LoadLynx / Tablet (768x1024)",
    styles: { width: "768px", height: "1024px" },
    type: "tablet",
  },
  loadlynxDesktopWide: {
    name: "LoadLynx / Desktop (1440x900)",
    styles: { width: "1440px", height: "900px" },
    type: "desktop",
  },
  loadlynxDesktopUltra: {
    name: "LoadLynx / Ultra-wide (1728x1117)",
    styles: { width: "1728px", height: "1117px" },
    type: "desktop",
  },
  loadlynxBp1024: {
    name: "LoadLynx / Breakpoint (1024)",
    styles: { width: "1024px", height: "800px" },
    type: "desktop",
  },
} as const;

const withDarkTheme: Decorator = (Story, context) => {
  document.documentElement.setAttribute("data-theme", "dark");
  const locale =
    typeof context.globals.loadlynxLocale === "string"
      ? context.globals.loadlynxLocale
      : "zh-CN";
  if (i18next.language !== locale) {
    void i18next.changeLanguage(locale);
  }
  document.body.classList.add(
    "bg-base-100",
    "text-base-content",
    "antialiased",
  );

  const showBreakpointCard = !!context.globals.loadlynxShowBreakpointCard;

  return (
    <div className="min-h-screen bg-base-100 p-0 text-base-content antialiased">
      {showBreakpointCard ? <BreakpointRulerOverlay /> : null}
      <Story />
    </div>
  );
};

const preview: Preview = {
  globalTypes: {
    loadlynxShowBreakpointCard: {
      description: "Show BreakpointRulerOverlay info card (Storybook only)",
      defaultValue: false,
      toolbar: {
        title: "Breakpoint card",
        items: [
          { value: false, title: "Off" },
          { value: true, title: "On" },
        ],
      },
    },
    loadlynxLocale: {
      description: "LoadLynx UI locale",
      defaultValue: "zh-CN",
      toolbar: {
        title: "Locale",
        items: [
          { value: "zh-CN", title: "中文" },
          { value: "en", title: "English" },
        ],
      },
    },
  },
  initialGlobals: {
    loadlynxShowBreakpointCard: false,
    loadlynxLocale: "zh-CN",
  },
  decorators: [withDarkTheme],
  parameters: {
    layout: "fullscreen",
    viewport: {
      options: {
        ...LOADLYNX_VIEWPORTS,
        ...INITIAL_VIEWPORTS,
      },
    },
  },
};

export default preview;
