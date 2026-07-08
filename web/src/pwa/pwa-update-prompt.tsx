import { useRegisterSW } from "virtual:pwa-register/react";
import { useEffect, useState } from "react";
import { PwaUpdatePromptView } from "./pwa-update-prompt-view.tsx";
import { waitForServiceWorkerWaiting } from "./service-worker-upgrade.ts";
import { type AppVersionPayload, hasRemoteAppUpdate } from "./version-check.ts";

const VERSION_POLL_INTERVAL_MS = 60_000;

function isStorybookRuntime(): boolean {
  return globalThis.__LOADLYNX_STORYBOOK__ === true;
}

export function PwaUpdatePrompt() {
  if (
    isStorybookRuntime() ||
    typeof window === "undefined" ||
    !("serviceWorker" in navigator)
  ) {
    return null;
  }

  return <PwaUpdatePromptRuntime />;
}

function PwaUpdatePromptRuntime() {
  const [registrationError, setRegistrationError] = useState<string | null>(
    null,
  );
  const [registeredServiceWorker, setRegisteredServiceWorker] =
    useState<ServiceWorkerRegistration | null>(null);
  const [versionUpdateReady, setVersionUpdateReady] = useState(false);
  const currentVersion = import.meta.env.VITE_APP_VERSION?.trim() || null;
  const {
    offlineReady: [offlineReady, setOfflineReady],
    needRefresh: [needRefresh, setNeedRefresh],
    updateServiceWorker,
  } = useRegisterSW({
    immediate: true,
    onRegisterError(error) {
      setRegistrationError(
        error instanceof Error ? error.message : String(error),
      );
      console.error("[pwa] service worker registration failed", error);
    },
    onRegisteredSW(_swUrl, registration) {
      setRegisteredServiceWorker(registration ?? null);
    },
  });

  useEffect(() => {
    if (typeof window === "undefined") {
      return undefined;
    }

    let disposed = false;

    const checkVersion = async () => {
      try {
        const response = await fetch("/version.json", {
          cache: "no-store",
        });
        if (!response.ok) {
          return;
        }

        const payload = (await response.json()) as AppVersionPayload;
        if (disposed) {
          return;
        }

        const hasUpdate = hasRemoteAppUpdate(currentVersion, payload);
        setVersionUpdateReady(hasUpdate);

        if (!hasUpdate || !("serviceWorker" in navigator)) {
          return;
        }

        const registrations = await navigator.serviceWorker.getRegistrations();
        await Promise.all(
          registrations.map((registration) => registration.update()),
        );
      } catch {
        // Best-effort only. Keep the current prompt state if the version probe fails.
      }
    };

    const handleVisibilityChange = () => {
      if (!document.hidden) {
        void checkVersion();
      }
    };

    void checkVersion();
    const intervalId = window.setInterval(() => {
      void checkVersion();
    }, VERSION_POLL_INTERVAL_MS);
    document.addEventListener("visibilitychange", handleVisibilityChange);

    return () => {
      disposed = true;
      window.clearInterval(intervalId);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    };
  }, []);

  const close = () => {
    setRegistrationError(null);
    setOfflineReady(false);
    setNeedRefresh(false);
    setVersionUpdateReady(false);
  };

  return (
    <PwaUpdatePromptView
      state={
        registrationError
          ? "registration-error"
          : needRefresh || versionUpdateReady
            ? "update-ready"
            : offlineReady
              ? "offline-ready"
              : "hidden"
      }
      errorMessage={registrationError}
      onClose={close}
      onUpdate={() => {
        void (async () => {
          if (needRefresh) {
            await updateServiceWorker(true);
            return;
          }

          if ("serviceWorker" in navigator) {
            const registrations =
              await navigator.serviceWorker.getRegistrations();
            await Promise.all(
              registrations.map((registration) => registration.update()),
            );

            const activeRegistration =
              registeredServiceWorker ?? registrations[0] ?? null;
            if (activeRegistration) {
              const waitingReady =
                await waitForServiceWorkerWaiting(activeRegistration);
              if (waitingReady) {
                await updateServiceWorker(true);
                return;
              }
            }
          } else {
            await updateServiceWorker(true);
          }
        })();
      }}
    />
  );
}
