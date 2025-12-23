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
            className="btn btn-sm btn-primary"
            onClick={() => addDemoDevice.mutate()}
            disabled={addDemoDevice.isPending}
          >
            {addDemoDevice.isPending ? (
              <span className="loading loading-spinner loading-xs" />
            ) : null}
            Add demo device
          </button>
        ) : null}
      </header>

      <div className="card bg-base-100 shadow-sm border border-base-200">
        <div className="card-body p-4">
          {devices.length === 0 ? (
            <p className="text-sm text-base-content/70">
              No devices yet. Add a demo device to populate the list.
            </p>
          ) : (
            <div className="overflow-x-auto">
              <table className="table table-zebra table-sm">
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
