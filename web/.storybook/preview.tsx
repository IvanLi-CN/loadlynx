import type { Decorator } from "@storybook/react";
import type { Preview } from "@storybook/react-vite";
import { INITIAL_VIEWPORTS } from "storybook/viewport";

import "../src/index.css";

const withDarkTheme: Decorator = (Story) => {
  document.documentElement.setAttribute("data-theme", "dark");
  document.body.classList.add(
    "bg-base-100",
    "text-base-content",
    "antialiased",
  );

  return (
    <div className="min-h-screen bg-base-100 p-6 text-base-content antialiased">
      <Story />
    </div>
  );
};

const preview: Preview = {
  decorators: [withDarkTheme],
  parameters: {
    viewport: {
      options: INITIAL_VIEWPORTS,
    },
  },
};

export default preview;
