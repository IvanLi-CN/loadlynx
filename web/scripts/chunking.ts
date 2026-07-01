function packageChunkName(id: string): string | null {
  if (!id.includes("node_modules")) {
    return null;
  }

  if (
    id.includes("/node_modules/@tanstack/react-router/") ||
    id.includes("/node_modules/@tanstack/router-core/") ||
    id.includes("/node_modules/@tanstack/react-query/") ||
    id.includes("/node_modules/@tanstack/query-core/")
  ) {
    return "tanstack-vendor";
  }

  if (
    id.includes("/node_modules/@radix-ui/") ||
    id.includes("/node_modules/lucide-react/") ||
    id.includes("/node_modules/class-variance-authority/") ||
    id.includes("/node_modules/clsx/") ||
    id.includes("/node_modules/tailwind-merge/")
  ) {
    return "ui-vendor";
  }

  if (
    id.includes("/node_modules/i18next/") ||
    id.includes("/node_modules/react-i18next/") ||
    id.includes("/node_modules/decimal.js/")
  ) {
    return "app-vendor";
  }

  if (id.includes("/node_modules/esptool-js/")) {
    return "esptool-vendor";
  }

  if (id.includes("/node_modules/workbox-window/")) {
    return "pwa-vendor";
  }

  if (
    id.includes("/node_modules/@storybook/") ||
    id.includes("/node_modules/storybook/") ||
    id.includes("/node_modules/@chromatic-com/")
  ) {
    return "storybook-vendor";
  }

  return "vendor";
}

export function manualChunks(id: string): string | null {
  return packageChunkName(id);
}
