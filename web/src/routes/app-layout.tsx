import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { Link, Outlet, useParams } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/router-devtools";
import { useDevicesQuery } from "../devices/hooks.ts";

export function AppLayout() {
  const { deviceId } = useParams({ strict: false }) as {
    deviceId?: string;
  };

  const { data: devices } = useDevicesQuery();
  const currentDevice =
    deviceId && devices
      ? devices.find((device) => device.id === deviceId)
      : undefined;

  return (
    <div
      style={{
        minHeight: "100vh",
        display: "flex",
        flexDirection: "column",
        backgroundColor: "#020617",
        color: "#e5e7eb",
        fontFamily: "-apple-system, BlinkMacSystemFont, system-ui, sans-serif",
      }}
    >
      <header
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "0.75rem 1.5rem",
          borderBottom: "1px solid #1f2937",
          background:
            "radial-gradient(circle at top, rgba(56,189,248,0.12), transparent 55%)",
        }}
      >
        <div>
          <h1
            style={{
              margin: 0,
              fontSize: "1.2rem",
              letterSpacing: "0.03em",
            }}
          >
            LoadLynx Web Console
          </h1>
          <p
            style={{
              margin: 0,
              marginTop: "0.1rem",
              fontSize: "0.8rem",
              color: "#9ca3af",
            }}
          >
            Network device manager &amp; CC control
          </p>
        </div>

        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: "0.75rem",
            minWidth: 0,
          }}
        >
          <label
            style={{
              display: "flex",
              flexDirection: "column",
              fontSize: "0.75rem",
            }}
          >
            <span
              style={{
                marginBottom: "0.1rem",
                color: "#9ca3af",
              }}
            >
              Current device
            </span>
            <select
              disabled
              style={{
                minWidth: "220px",
                padding: "0.25rem 0.5rem",
                borderRadius: "0.375rem",
                border: "1px solid #374151",
                backgroundColor: "#020617",
                color: "#e5e7eb",
                fontSize: "0.8rem",
              }}
            >
              <option>
                {currentDevice
                  ? `${currentDevice.name} (${currentDevice.id})`
                  : "No device selected (device selector)"}
              </option>
            </select>
          </label>

          <Link
            to="/devices"
            activeProps={{
              style: { backgroundColor: "#0f172a" },
            }}
            style={{
              padding: "0.4rem 0.8rem",
              borderRadius: "999px",
              border: "1px solid #374151",
              fontSize: "0.8rem",
              textDecoration: "none",
              color: "#e5e7eb",
              backgroundColor: "#020617",
            }}
          >
            Add device
          </Link>
        </div>
      </header>

      <div
        style={{
          display: "flex",
          flex: 1,
          minHeight: 0,
        }}
      >
        <nav
          aria-label="Primary"
          style={{
            width: "220px",
            borderRight: "1px solid #1f2937",
            padding: "1rem 1.25rem",
            boxSizing: "border-box",
          }}
        >
          <div
            style={{
              marginBottom: "0.75rem",
              fontSize: "0.75rem",
              textTransform: "uppercase",
              letterSpacing: "0.08em",
              color: "#6b7280",
            }}
          >
            Navigation
          </div>

          <div
            style={{
              display: "flex",
              flexDirection: "column",
              gap: "0.25rem",
              fontSize: "0.85rem",
            }}
          >
            <Link
              to="/devices"
              activeProps={{
                style: {
                  backgroundColor: "#0f172a",
                  color: "#38bdf8",
                },
              }}
              style={{
                display: "block",
                padding: "0.4rem 0.6rem",
                borderRadius: "0.375rem",
                textDecoration: "none",
                color: "#e5e7eb",
              }}
            >
              Devices
            </Link>

            {deviceId ? (
              <Link
                to="/$deviceId/cc"
                params={{ deviceId }}
                activeProps={{
                  style: {
                    backgroundColor: "#0f172a",
                    color: "#38bdf8",
                  },
                }}
                style={{
                  display: "block",
                  padding: "0.4rem 0.6rem",
                  borderRadius: "0.375rem",
                  textDecoration: "none",
                  color: "#e5e7eb",
                }}
              >
                CC Control
              </Link>
            ) : (
              <button
                type="button"
                disabled
                style={{
                  display: "block",
                  width: "100%",
                  padding: "0.4rem 0.6rem",
                  borderRadius: "0.375rem",
                  border: "1px dashed #1f2937",
                  backgroundColor: "transparent",
                  color: "#6b7280",
                  fontSize: "0.85rem",
                  textAlign: "left",
                  cursor: "not-allowed",
                }}
              >
                CC Control (select device)
              </button>
            )}

            <div
              style={{
                marginTop: "0.75rem",
                fontSize: "0.75rem",
                color: "#6b7280",
              }}
            >
              Other functions
              <div style={{ marginTop: "0.25rem" }}>
                <span>status / settings (placeholders)</span>
              </div>
            </div>
          </div>
        </nav>

        <main
          style={{
            flex: 1,
            padding: "1.25rem 1.5rem",
            boxSizing: "border-box",
          }}
        >
          <Outlet />
        </main>
      </div>

      {import.meta.env.DEV ? (
        <>
          <ReactQueryDevtools initialIsOpen={false} />
          <TanStackRouterDevtools initialIsOpen={false} />
        </>
      ) : null}
    </div>
  );
}

export default AppLayout;
