import { useRegisterSW } from "virtual:pwa-register/react";
import { useState } from "react";
import { PwaUpdatePromptView } from "./pwa-update-prompt-view.tsx";

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
  });

  const close = () => {
    setRegistrationError(null);
    setOfflineReady(false);
    setNeedRefresh(false);
  };

  return (
    <PwaUpdatePromptView
      state={
        needRefresh
          ? "update-ready"
          : offlineReady
            ? "offline-ready"
            : registrationError
              ? "registration-error"
              : "hidden"
      }
      errorMessage={registrationError}
      onClose={close}
      onUpdate={() => {
        void updateServiceWorker(true);
      }}
    />
  );
}
