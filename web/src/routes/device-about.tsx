import { useTranslation } from "react-i18next";
import { PageContainer } from "../components/layout/page-container.tsx";
import { useDeviceIdentityByBaseUrl } from "../devices/hooks.ts";
import { useDeviceContext } from "../layouts/device-layout.tsx";

function readAppVersion(): string {
  return import.meta.env.VITE_APP_VERSION?.trim() || "dev";
}

function readAppGitSha(): string {
  return import.meta.env.VITE_APP_GIT_SHA?.trim() || "unknown";
}

function readAppGitTag(): string {
  return import.meta.env.VITE_APP_GIT_TAG?.trim() || "unknown";
}

export function DeviceAboutRoute() {
  const { t } = useTranslation();
  const { device, deviceId, baseUrl } = useDeviceContext();
  const identityQuery = useDeviceIdentityByBaseUrl(deviceId, baseUrl);
  const identity = identityQuery.data;

  const transportSummary = [
    device.devd ? "devd" : null,
    device.webSerial ? "Web Serial" : null,
    device.connectionMarks?.includes("usb") ? "USB" : null,
    device.connectionMarks?.includes("lan") ? "LAN" : null,
    device.baseUrl,
  ]
    .filter(Boolean)
    .join(" · ");

  return (
    <PageContainer className="space-y-6 font-mono tabular-nums">
      <div className="ll-panel bg-base-100 shadow-sm border border-base-200">
        <div className="ll-panel-body p-6">
          <header>
            <h3 className="text-lg font-bold">{t("about.title")}</h3>
            <p className="mt-1 text-sm text-base-content/65">
              {t("about.subtitle")}
            </p>
          </header>

          <div className="grid gap-6 lg:grid-cols-2">
            <section className="overflow-x-auto">
              <table className="ll-table ll-table-sm">
                <tbody>
                  <tr>
                    <td className="text-base-content/60">
                      {t("about.deviceName")}
                    </td>
                    <td>{device.name}</td>
                  </tr>
                  <tr>
                    <td className="text-base-content/60">
                      {t("about.deviceId")}
                    </td>
                    <td>{identity?.device_id ?? device.id}</td>
                  </tr>
                  <tr>
                    <td className="text-base-content/60">
                      {t("about.hostname")}
                    </td>
                    <td>
                      {identity?.hostname ?? identity?.network.hostname ?? "—"}
                    </td>
                  </tr>
                  <tr>
                    <td className="text-base-content/60">
                      {t("about.shortId")}
                    </td>
                    <td>{identity?.short_id ?? "—"}</td>
                  </tr>
                  <tr>
                    <td className="text-base-content/60">{t("about.ip")}</td>
                    <td>{identity?.network.ip ?? "—"}</td>
                  </tr>
                  <tr>
                    <td className="text-base-content/60">{t("about.mac")}</td>
                    <td>{identity?.network.mac ?? "—"}</td>
                  </tr>
                </tbody>
              </table>
            </section>

            <section className="overflow-x-auto">
              <table className="ll-table ll-table-sm">
                <tbody>
                  <tr>
                    <td className="text-base-content/60">
                      {t("about.digitalFw")}
                    </td>
                    <td>{identity?.digital_fw_version ?? "—"}</td>
                  </tr>
                  <tr>
                    <td className="text-base-content/60">
                      {t("about.analogFw")}
                    </td>
                    <td>{identity?.analog_fw_version ?? "—"}</td>
                  </tr>
                  <tr>
                    <td className="text-base-content/60">
                      {t("about.apiVersion")}
                    </td>
                    <td>{identity?.capabilities.api_version ?? "—"}</td>
                  </tr>
                  <tr>
                    <td className="text-base-content/60">
                      {t("about.protocolVersion")}
                    </td>
                    <td>{identity?.protocol_version ?? "—"}</td>
                  </tr>
                  <tr>
                    <td className="text-base-content/60">
                      {t("about.transport")}
                    </td>
                    <td>{transportSummary}</td>
                  </tr>
                  <tr>
                    <td className="text-base-content/60">
                      {t("about.appVersion")}
                    </td>
                    <td>
                      {readAppVersion()} · {readAppGitTag()} · {readAppGitSha()}
                    </td>
                  </tr>
                </tbody>
              </table>
            </section>
          </div>
        </div>
      </div>
    </PageContainer>
  );
}

export default DeviceAboutRoute;
