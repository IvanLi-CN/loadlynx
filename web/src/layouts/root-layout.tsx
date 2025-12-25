import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { Outlet } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/router-devtools";

export function RootLayout() {
  return (
    <div className="flex flex-col min-h-screen bg-base-100 text-base-content antialiased">
      <Outlet />

      {import.meta.env.DEV ? (
        <>
          <ReactQueryDevtools initialIsOpen={false} />
          <TanStackRouterDevtools initialIsOpen={false} />
        </>
      ) : null}
    </div>
  );
}

export default RootLayout;
