import { expect, test } from "bun:test";
import {
  __testClearDeviceQueues,
  __testEnqueueForDevice,
} from "../../src/api/client.ts";

const nextTick = () => Promise.resolve();

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

  await nextTick();
  await nextTick();

  expect(starts).toEqual(["a1", "b1"]);

  finishers.shift()?.();
  await p1;
  await nextTick();
  await nextTick();
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
