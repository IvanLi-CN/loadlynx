function packageChunkName(id: string): string | null {
  if (!id.includes("node_modules")) {
    return null;
  }

  if (id.includes("/node_modules/recharts/")) {
    if (
      id.includes("/node_modules/recharts/es6/chart/") ||
      id.includes("/node_modules/recharts/es6/cartesian/") ||
      id.includes("/node_modules/recharts/es6/component/") ||
      id.includes("/node_modules/recharts/es6/container/") ||
      id.includes("/node_modules/recharts/es6/context/")
    ) {
      return "recharts-core-vendor";
    }
    return "recharts-runtime-vendor";
  }

  if (
    id.includes("/node_modules/victory-vendor/") ||
    id.includes("/node_modules/d3-")
  ) {
    return "chart-runtime-vendor";
  }

  if (id.includes("/node_modules/react-smooth/")) {
    return "chart-motion-vendor";
  }

  if (
    id.includes("/node_modules/@tanstack/react-router/") ||
    id.includes("/node_modules/@tanstack/router-core/") ||
    id.includes("/node_modules/@tanstack/react-query/") ||
    id.includes("/node_modules/@tanstack/query-core/") ||
    id.includes("/node_modules/@tanstack/store/") ||
    id.includes("/node_modules/@tanstack/react-store/") ||
    id.includes("/node_modules/@tanstack/history/")
  ) {
    return "tanstack-vendor";
  }

  if (
    id.includes("/node_modules/@reduxjs/") ||
    id.includes("/node_modules/react-redux/") ||
    id.includes("/node_modules/redux/") ||
    id.includes("/node_modules/redux-thunk/") ||
    id.includes("/node_modules/reselect/") ||
    id.includes("/node_modules/immer/")
  ) {
    return "state-vendor";
  }

  if (id.includes("/node_modules/es-toolkit/")) {
    return "utility-vendor";
  }

  if (
    id.includes("/node_modules/pako/") ||
    id.includes("/node_modules/atob-lite/")
  ) {
    return "compression-vendor";
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

  if (
    id.includes("/node_modules/@tanstack/react-query-devtools/") ||
    id.includes("/node_modules/@tanstack/react-router-devtools/")
  ) {
    return "tanstack-devtools-vendor";
  }

  if (id.includes("/node_modules/esptool-js/")) {
    return "esptool-vendor";
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
