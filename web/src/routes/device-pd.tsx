import { PageContainer } from "../components/layout/page-container.tsx";
import { useDeviceContext } from "../layouts/device-layout.tsx";
import { PdControlPanel } from "./device-pd-panel.tsx";

export function DevicePdRoute() {
  const { deviceId, device, baseUrl } = useDeviceContext();

  return (
    <PageContainer className="flex flex-col gap-6 font-mono tabular-nums">
      <PdControlPanel
        deviceId={deviceId}
        deviceName={device.name}
        baseUrl={baseUrl}
      />
    </PageContainer>
  );
}

export default DevicePdRoute;
