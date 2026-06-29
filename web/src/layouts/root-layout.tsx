import { useQueryClient } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { Outlet, useRouterState } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";
import { useEffect } from "react";
import { ENABLE_APP_DEVTOOLS } from "../api/client.ts";
import { invalidateDevicesQueryCache } from "../devices/query-cache.ts";
import {
  parseDemoModeParam,
  resolveDemoMode,
  stripDemoModeParam,
} from "../lib/demo-mode.ts";

function isStorybookRuntime(): boolean {
  return globalThis.__LOADLYNX_STORYBOOK__ === true;
}

export function RootLayout() {
  const storybookRuntime = isStorybookRuntime();
  const queryClient = useQueryClient();
  const locationHref = useRouterState({
    select: (state) => state.location.href,
  });

  useEffect(() => {
    if (storybookRuntime || typeof window === "undefined") return;

    const currentUrl = new URL(locationHref, window.location.origin);
    const hadDemoModeParam = parseDemoModeParam(currentUrl.search) !== null;
    resolveDemoMode(currentUrl, window.localStorage);
    const cleanedUrl = stripDemoModeParam(currentUrl);

    if (cleanedUrl) {
      window.history.replaceState(window.history.state, "", cleanedUrl);
    }

    if (hadDemoModeParam) {
      void invalidateDevicesQueryCache(queryClient);
    }
  }, [locationHref, queryClient, storybookRuntime]);

  return (
    <div className="ll-app-shell flex flex-col min-h-screen antialiased">
      <Outlet />

      {import.meta.env.DEV && ENABLE_APP_DEVTOOLS && !storybookRuntime ? (
        <>
          <ReactQueryDevtools initialIsOpen={false} />
          <TanStackRouterDevtools initialIsOpen={false} />
        </>
      ) : null}
    </div>
  );
}

export default RootLayout;
