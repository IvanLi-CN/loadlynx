import { PageContainer } from "../components/layout/page-container.tsx";
import { useDeviceContext } from "../layouts/device-layout.tsx";
import {
  DevdFirmwarePanel,
  UsbSessionPanel,
  WebSerialFlashPanel,
} from "./device-firmware-panels.tsx";

export function DeviceFirmwareRoute() {
  const { device } = useDeviceContext();
  const canUseDevd = Boolean(device.devd?.deviceId && device.devd?.leaseId);

  return (
    <PageContainer className="space-y-6">
      <header className="flex flex-col gap-2">
        <h2 className="text-2xl font-bold">Firmware</h2>
        <p className="text-sm text-base-content/70">
          Flash through the local devd bridge or a Web Serial browser session.
          Real digital flashes require artifact hash evidence, explicit
          confirmation, and post-flash identity capture.
        </p>
      </header>

      {!canUseDevd ? (
        <div role="alert" className="ll-alert ll-alert-warning">
          <span>
            This device is not bound to an active devd USB lease. Connect it
            from the Devices page before using devd firmware operations.
          </span>
        </div>
      ) : null}

      <DevdFirmwarePanel device={device} />
      <WebSerialFlashPanel device={device} />
      <UsbSessionPanel device={device} />
    </PageContainer>
  );
}

export default DeviceFirmwareRoute;
