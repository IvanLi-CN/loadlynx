import { ENABLE_MOCK_DEVTOOLS } from "../api/client.ts";
import { useAddDeviceMutation, useDevicesQuery } from "./hooks.ts";

export function DevicesPanel() {
  const devicesQuery = useDevicesQuery();
  const addDemoDevice = useAddDeviceMutation();

  const devices = devicesQuery.data ?? [];

  return (
    <div className="max-w-3xl mx-auto space-y-4">
      <header className="flex items-start justify-between gap-4">
        <div className="space-y-1">
          <h2 className="text-2xl font-bold">Devices</h2>
          <p className="text-sm text-base-content/70">
            Storybook-safe device persistence via injected{" "}
            <code>DeviceStore</code>.
          </p>
        </div>

        {ENABLE_MOCK_DEVTOOLS ? (
          <button
            type="button"
            className="ll-button ll-button-sm ll-button-primary"
            onClick={() => addDemoDevice.mutate()}
            disabled={addDemoDevice.isPending}
          >
            {addDemoDevice.isPending ? (
              <span className="ll-loading ll-loading-spinner ll-loading-xs" />
            ) : null}
            Add sample device
          </button>
        ) : null}
      </header>

      <div className="ll-panel bg-base-100 shadow-sm border border-base-200">
        <div className="ll-panel-body p-4">
          {devices.length === 0 ? (
            <p className="text-sm text-base-content/70">
              No devices yet. Add a sample device to populate the list.
            </p>
          ) : (
            <div className="overflow-x-auto">
              <table className="ll-table ll-table-sm">
                <thead>
                  <tr>
                    <th>Name</th>
                    <th>ID</th>
                    <th>Base URL</th>
                  </tr>
                </thead>
                <tbody>
                  {devices.map((device) => (
                    <tr key={device.id}>
                      <td>{device.name}</td>
                      <td className="font-mono text-xs">{device.id}</td>
                      <td className="font-mono text-xs">{device.baseUrl}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
