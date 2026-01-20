import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { Outlet } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/router-devtools";

function isStorybookRuntime(): boolean {
  return globalThis.__LOADLYNX_STORYBOOK__ === true;
}

export function RootLayout() {
  const storybookRuntime = isStorybookRuntime();
  return (
    <div className="flex flex-col min-h-screen bg-base-100 text-base-content antialiased">
      <Outlet />

      {import.meta.env.DEV && !storybookRuntime ? (
        <>
          <ReactQueryDevtools initialIsOpen={false} />
          <TanStackRouterDevtools initialIsOpen={false} />
        </>
      ) : null}
    </div>
  );
}

export default RootLayout;
