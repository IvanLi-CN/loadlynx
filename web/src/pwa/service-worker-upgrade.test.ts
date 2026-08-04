import { expect, test } from "vitest";
import {
  activateWaitingWorkerOrReload,
  waitForServiceWorkerWaiting,
} from "./service-worker-upgrade.ts";

class FakeServiceWorker extends EventTarget {}

class FakeRegistration extends EventTarget {
  waiting: ServiceWorker | null = null;
  installing: ServiceWorker | null = null;
}

test("waitForServiceWorkerWaiting resolves once the registration gets a waiting worker", async () => {
  const registration = new FakeRegistration();
  const installing = new FakeServiceWorker() as ServiceWorker;
  registration.installing = installing;

  const waitingPromise = waitForServiceWorkerWaiting(registration, 100);
  registration.waiting = installing;
  installing.dispatchEvent(new Event("statechange"));

  await expect(waitingPromise).resolves.toBe(true);
});

test("waitForServiceWorkerWaiting times out when no waiting worker appears", async () => {
  const registration = new FakeRegistration();

  await expect(waitForServiceWorkerWaiting(registration, 10)).resolves.toBe(
    false,
  );
});

test("activateWaitingWorkerOrReload reloads when no waiting worker appears", async () => {
  const registration = new FakeRegistration();
  const updates: boolean[] = [];
  let reloads = 0;

  await activateWaitingWorkerOrReload(
    registration,
    async (reloadPage) => {
      updates.push(Boolean(reloadPage));
    },
    () => {
      reloads += 1;
    },
    1,
  );

  expect(updates).toEqual([]);
  expect(reloads).toBe(1);
});
