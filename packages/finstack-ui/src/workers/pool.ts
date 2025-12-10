import { wrap, type Remote } from "comlink";
import type { FinstackEngineWorkerApi } from "./finstackEngine";

let workerPromise: Promise<Remote<FinstackEngineWorkerApi>> | null = null;
let workerRef: Worker | null = null;

async function createWorker(): Promise<Remote<FinstackEngineWorkerApi>> {
  workerRef = new Worker(new URL("./finstackEngine.ts", import.meta.url), {
    type: "module",
  });
  return wrap<FinstackEngineWorkerApi>(workerRef);
}

export function getEngineWorker(): Promise<Remote<FinstackEngineWorkerApi>> {
  if (!workerPromise) {
    workerPromise = createWorker();
  }
  return workerPromise;
}

export function resetEngineWorker() {
  workerRef?.terminate();
  workerRef = null;
  workerPromise = null;
}
