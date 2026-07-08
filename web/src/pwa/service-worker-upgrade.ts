const DEFAULT_WAIT_TIMEOUT_MS = 4_000;

type WaitingRegistration = Pick<
  ServiceWorkerRegistration,
  "waiting" | "installing" | "addEventListener" | "removeEventListener"
>;

export function waitForServiceWorkerWaiting(
  registration: WaitingRegistration,
  timeoutMs = DEFAULT_WAIT_TIMEOUT_MS,
): Promise<boolean> {
  if (registration.waiting) {
    return Promise.resolve(true);
  }

  return new Promise((resolve) => {
    let timeoutId: ReturnType<typeof setTimeout> | null = null;
    let installingWorker: ServiceWorker | null = null;

    const cleanup = () => {
      registration.removeEventListener("updatefound", handleUpdateFound);
      if (installingWorker) {
        installingWorker.removeEventListener("statechange", handleStateChange);
      }
      if (timeoutId !== null) {
        globalThis.clearTimeout(timeoutId);
      }
    };

    const finish = (result: boolean) => {
      cleanup();
      resolve(result);
    };

    const handleStateChange = () => {
      if (registration.waiting) {
        finish(true);
      }
    };

    const attachInstallingWorker = (worker: ServiceWorker | null) => {
      if (!worker || worker === installingWorker) {
        return;
      }
      if (installingWorker) {
        installingWorker.removeEventListener("statechange", handleStateChange);
      }
      installingWorker = worker;
      installingWorker.addEventListener("statechange", handleStateChange);
    };

    const handleUpdateFound = () => {
      attachInstallingWorker(registration.installing);
      if (registration.waiting) {
        finish(true);
      }
    };

    registration.addEventListener("updatefound", handleUpdateFound);
    attachInstallingWorker(registration.installing);
    timeoutId = globalThis.setTimeout(() => {
      finish(Boolean(registration.waiting));
    }, timeoutMs);
  });
}
