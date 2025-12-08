import { useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { useMemo } from "react";
import { isHttpApiError } from "../api/client.ts";
import type { StoredDevice } from "../devices/device-store.ts";
import {
  useAddDeviceMutation,
  useDeviceIdentity,
  useDevicesQuery,
} from "../devices/hooks.ts";

export function DevicesRoute() {
  const queryClient = useQueryClient();
  const devicesQuery = useDevicesQuery();
  const addDeviceMutation = useAddDeviceMutation();

  const devices: StoredDevice[] = useMemo(
    () => devicesQuery.data ?? [],
    [devicesQuery.data],
  );

  const isMutating = addDeviceMutation.isPending;

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "1rem",
        maxWidth: "960px",
      }}
    >
      <header>
        <h2
          style={{
            margin: 0,
            fontSize: "1.25rem",
          }}
        >
          Devices
        </h2>
        <p
          style={{
            margin: "0.25rem 0 0",
            fontSize: "0.9rem",
            color: "#9ca3af",
          }}
        >
          Manage known devices for the LoadLynx network console. When the HTTP
          backend is enabled, each device is probed via{" "}
          <code>/api/v1/identity</code> to show live status.
        </p>
      </header>

      <div
        style={{
          display: "flex",
          gap: "0.75rem",
          alignItems: "center",
        }}
      >
        <button
          type="button"
          onClick={() => {
            addDeviceMutation.mutate(undefined, {
              onSuccess: () => {
                // Keep any stale query instances (e.g. from other tabs) in sync.
                queryClient.invalidateQueries({ queryKey: ["devices"] });
              },
            });
          }}
          disabled={isMutating}
          style={{
            padding: "0.5rem 0.9rem",
            borderRadius: "0.375rem",
            border: "1px solid #4b5563",
            backgroundColor: "#111827",
            color: "#e5e7eb",
            fontSize: "0.9rem",
            cursor: isMutating ? "wait" : "pointer",
          }}
        >
          {isMutating ? "Adding device..." : "Add mock device"}
        </button>
        <span
          style={{
            fontSize: "0.8rem",
            color: "#6b7280",
          }}
        >
          Adds a hard-coded mock device entry backed by in-memory API state.
        </span>
      </div>

      <section
        aria-label="Known devices"
        style={{
          borderRadius: "0.75rem",
          border: "1px solid #1f2937",
          background:
            "radial-gradient(circle at top left, rgba(56,189,248,0.06), transparent 60%), #020617",
          padding: "1rem 1.25rem",
        }}
      >
        {devicesQuery.isLoading ? (
          <p
            style={{
              margin: 0,
              fontSize: "0.9rem",
              color: "#9ca3af",
            }}
          >
            Loading devices...
          </p>
        ) : devices.length === 0 ? (
          <p
            style={{
              margin: 0,
              fontSize: "0.9rem",
              color: "#9ca3af",
            }}
          >
            No devices yet. Use the{" "}
            <strong style={{ fontWeight: 500 }}>Add mock device</strong> action
            above to seed a test entry.
          </p>
        ) : (
          <table
            style={{
              width: "100%",
              borderCollapse: "collapse",
              fontSize: "0.9rem",
            }}
          >
            <thead>
              <tr
                style={{
                  textAlign: "left",
                  color: "#9ca3af",
                  borderBottom: "1px solid #111827",
                }}
              >
                <th style={{ padding: "0.4rem 0.25rem" }}>Name</th>
                <th style={{ padding: "0.4rem 0.25rem" }}>Device ID</th>
                <th style={{ padding: "0.4rem 0.25rem" }}>Base URL</th>
                <th style={{ padding: "0.4rem 0.25rem" }}>Status</th>
                <th style={{ padding: "0.4rem 0.25rem" }}>Actions</th>
              </tr>
            </thead>
            <tbody>
              {devices.map((device) => (
                <DeviceRow key={device.id} device={device} />
              ))}
            </tbody>
          </table>
        )}
      </section>
    </div>
  );
}

export default DevicesRoute;

function DeviceRow(props: { device: StoredDevice }) {
  const { device } = props;
  const identityQuery = useDeviceIdentity(device);

  const identity = identityQuery.data;
  const error: unknown = identityQuery.error;

  let statusLabel = "Checking...";
  let statusColor = "#6b7280";
  let statusDetail: string | null = null;

  if (identityQuery.isLoading || identityQuery.isFetching) {
    statusLabel = "Checking...";
    statusColor = "#6b7280";
  } else if (identityQuery.isSuccess && identity) {
    statusLabel = "Online";
    statusColor = "#22c55e";
    statusDetail = identity.network.ip;
  } else if (identityQuery.isError) {
    statusLabel = "Offline";
    statusColor = "#f97316";
    if (isHttpApiError(error)) {
      const code = error.code ?? "HTTP_ERROR";
      const snippet =
        error.message.length > 80
          ? `${error.message.slice(0, 77)}…`
          : error.message;
      statusDetail = `${code}: ${snippet}`;
    } else if (error instanceof Error) {
      const snippet =
        error.message.length > 80
          ? `${error.message.slice(0, 77)}…`
          : error.message;
      statusDetail = snippet;
    } else {
      statusDetail = "Unknown error";
    }
  }

  return (
    <tr
      style={{
        borderBottom: "1px solid #0f172a",
      }}
    >
      <td style={{ padding: "0.4rem 0.25rem" }}>{device.name}</td>
      <td
        style={{
          padding: "0.4rem 0.25rem",
          fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
        }}
      >
        {device.id}
      </td>
      <td
        style={{
          padding: "0.4rem 0.25rem",
          fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
          fontSize: "0.8rem",
        }}
      >
        {device.baseUrl}
      </td>
      <td
        style={{
          padding: "0.4rem 0.25rem",
          fontSize: "0.8rem",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: "0.4rem",
          }}
        >
          <span
            style={{
              width: "0.5rem",
              height: "0.5rem",
              borderRadius: "999px",
              backgroundColor: statusColor,
              boxShadow:
                statusColor === "#22c55e"
                  ? "0 0 0 4px rgba(34,197,94,0.25)"
                  : "none",
            }}
          />
          <span>{statusLabel}</span>
        </div>
        {statusDetail ? (
          <div
            style={{
              marginTop: "0.2rem",
              fontSize: "0.75rem",
              color: "#9ca3af",
            }}
          >
            {statusDetail}
          </div>
        ) : null}
        <button
          type="button"
          onClick={() => {
            void identityQuery.refetch();
          }}
          disabled={identityQuery.isFetching}
          style={{
            marginTop: "0.35rem",
            padding: "0.25rem 0.6rem",
            borderRadius: "999px",
            border: "1px solid #374151",
            backgroundColor: "#020617",
            color: "#e5e7eb",
            fontSize: "0.75rem",
            cursor: identityQuery.isFetching ? "wait" : "pointer",
          }}
        >
          {identityQuery.isFetching ? "Pinging..." : "Test connectivity"}
        </button>
      </td>
      <td
        style={{
          padding: "0.4rem 0.25rem",
          textAlign: "right",
        }}
      >
        <Link
          to="/$deviceId/cc"
          params={{ deviceId: device.id }}
          style={{
            display: "inline-flex",
            alignItems: "center",
            padding: "0.3rem 0.7rem",
            borderRadius: "999px",
            border: "1px solid #4b5563",
            textDecoration: "none",
            color: "#e5e7eb",
            fontSize: "0.8rem",
          }}
        >
          Open CC Control
        </Link>
      </td>
    </tr>
  );
}
