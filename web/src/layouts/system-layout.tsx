import { Link, Outlet } from "@tanstack/react-router";
import { useTranslation } from "react-i18next";
import { useDeviceContext } from "./device-layout.tsx";

const SYSTEM_TABS = [
  { key: "settings", to: "/$deviceId/settings" as const },
  { key: "calibration", to: "/$deviceId/calibration" as const },
  { key: "status", to: "/$deviceId/status" as const },
  { key: "firmware", to: "/$deviceId/firmware" as const },
  { key: "about", to: "/$deviceId/about" as const },
] as const;

export function SystemLayout() {
  const { t } = useTranslation();
  const { deviceId, device } = useDeviceContext();

  return (
    <div className="grid gap-6 xl:grid-cols-[13rem_minmax(0,1fr)] xl:items-start">
      <div className="space-y-5 xl:sticky xl:top-24">
        <header className="space-y-2 px-1">
          <h2 className="text-2xl font-bold tracking-[0.01em]">
            {t("nav.system")}
          </h2>
          <p className="text-sm text-base-content/65">
            {device.name} · {device.id}
          </p>
        </header>

        <nav
          aria-label={t("nav.systemSections")}
          className="overflow-x-auto pb-1 xl:overflow-visible xl:pb-0"
        >
          <div className="flex min-w-max gap-2 rounded-2xl border border-base-300/70 bg-base-200/20 p-1 xl:min-w-0 xl:flex-col xl:items-stretch xl:gap-1.5 xl:bg-transparent xl:p-0">
            {SYSTEM_TABS.map((tab) => (
              <div key={tab.key} className="space-y-1">
                {tab.key === "calibration" ? (
                  <>
                    <div className="px-3 py-2 text-sm font-semibold text-base-content/72">
                      {t("nav.calibration")}
                    </div>
                    <div className="space-y-1">
                      <Link
                        to="/$deviceId/calibration"
                        params={{ deviceId }}
                        search={{ section: "voltage" }}
                        activeOptions={{
                          includeSearch: true,
                          exact: true,
                        }}
                        activeProps={{
                          className:
                            "ll-button ll-button-sm ll-button-active border-transparent xl:justify-start",
                        }}
                        className="ll-button ll-button-sm ll-button-ghost border border-transparent xl:w-full xl:justify-start"
                      >
                        {t("nav.calibrationVoltage")}
                      </Link>
                      <Link
                        to="/$deviceId/calibration"
                        params={{ deviceId }}
                        search={{ section: "current_ch1" }}
                        activeOptions={{
                          includeSearch: true,
                          exact: true,
                        }}
                        activeProps={{
                          className:
                            "ll-button ll-button-sm ll-button-active border-transparent xl:justify-start",
                        }}
                        className="ll-button ll-button-sm ll-button-ghost border border-transparent xl:w-full xl:justify-start"
                      >
                        {t("nav.calibrationCurrentCh1")}
                      </Link>
                      <Link
                        to="/$deviceId/calibration"
                        params={{ deviceId }}
                        search={{ section: "current_ch2" }}
                        activeOptions={{
                          includeSearch: true,
                          exact: true,
                        }}
                        activeProps={{
                          className:
                            "ll-button ll-button-sm ll-button-active border-transparent xl:justify-start",
                        }}
                        className="ll-button ll-button-sm ll-button-ghost border border-transparent xl:w-full xl:justify-start"
                      >
                        {t("nav.calibrationCurrentCh2")}
                      </Link>
                    </div>
                  </>
                ) : null}

                {tab.key !== "calibration" ? (
                  <Link
                    to={tab.to}
                    params={{ deviceId }}
                    activeProps={{
                      className:
                        "ll-button ll-button-sm ll-button-active border-transparent xl:justify-start",
                    }}
                    className="ll-button ll-button-sm ll-button-ghost border border-transparent xl:w-full xl:justify-start"
                  >
                    {t(`nav.${tab.key}`)}
                  </Link>
                ) : null}
              </div>
            ))}
          </div>
        </nav>
      </div>

      <div className="min-w-0">
        <Outlet />
      </div>
    </div>
  );
}

export default SystemLayout;
