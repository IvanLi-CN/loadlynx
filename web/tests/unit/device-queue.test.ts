import { afterEach, expect, test, vi } from "vitest";
import {
  __testClearDeviceQueues,
  __testEnqueueForDevice,
  HttpApiError,
} from "../../src/api/client.ts";

const nextTick = () => Promise.resolve();
const flushQueue = async () => {
  await nextTick();
  await nextTick();
  await nextTick();
  await nextTick();
};

afterEach(() => {
  vi.useRealTimers();
  __testClearDeviceQueues();
});

test("per-baseUrl operations are serialized while different baseUrls run independently", async () => {
  __testClearDeviceQueues();

  const starts: string[] = [];
  const finishers: Array<() => void> = [];

  const op = (label: string) => () => {
    starts.push(label);
    return new Promise<string>((resolve) => {
      finishers.push(() => resolve(label));
    });
  };

  const p1 = __testEnqueueForDevice("http://dev-a", op("a1"));
  const p2 = __testEnqueueForDevice("http://dev-a", op("a2"));
  const p3 = __testEnqueueForDevice("http://dev-b", op("b1"));

  await flushQueue();

  expect(starts).toEqual(["a1", "b1"]);

  finishers.shift()?.();
  await p1;
  await flushQueue();
  expect(starts).toEqual(["a1", "b1", "a2"]);

  finishers.shift()?.();
  finishers.shift()?.();
  await Promise.all([p1, p2, p3]);
});

test("queue advances even when a task fails", async () => {
  __testClearDeviceQueues();

  const starts: string[] = [];
  const p1 = __testEnqueueForDevice("http://dev-a", () => {
    starts.push("a1");
    return Promise.reject(new Error("fail"));
  });
  const p2 = __testEnqueueForDevice("http://dev-a", () => {
    starts.push("a2");
    return Promise.resolve("ok");
  });

  await expect(p1).rejects.toBeInstanceOf(Error);
  await expect(p2).resolves.toBe("ok");
  expect(starts).toEqual(["a1", "a2"]);
});

test("queue wait times out when a previous task never settles", async () => {
  __testClearDeviceQueues();
  vi.useFakeTimers();

  const starts: string[] = [];
  const p1 = __testEnqueueForDevice("http://dev-a", () => {
    starts.push("a1");
    return new Promise<string>(() => undefined);
  });
  const p2 = __testEnqueueForDevice("http://dev-a", () => {
    starts.push("a2");
    return Promise.resolve("ok");
  }).catch((error: unknown) => error);

  await flushQueue();
  expect(starts).toEqual(["a1"]);

  await vi.advanceTimersByTimeAsync(15_000);

  const error = await p2;
  expect(error).toBeInstanceOf(HttpApiError);
  expect(error).toMatchObject({
    code: "QUEUE_WAIT_TIMEOUT",
    retryable: true,
  });
  expect(starts).toEqual(["a1"]);
  void p1;
});
