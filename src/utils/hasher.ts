import {
  createSHA256,
  createSHA384,
  createSHA512,
  createSHA1,
  createMD5,
  createBLAKE2b,
  createBLAKE3,
  IHasher,
} from 'hash-wasm';
import { AlgorithmType, HashTelemetry } from '../types';

const CHUNK_SIZE = 4 * 1024 * 1024; // 4 MiB chunk loop, exactly matching CheckSum's optimal chunking

export async function createHasherInstance(algo: AlgorithmType): Promise<IHasher> {
  switch (algo) {
    case 'SHA-256':
      return await createSHA256();
    case 'SHA-384':
      return await createSHA384();
    case 'SHA-512':
      return await createSHA512();
    case 'SHA-1':
      return await createSHA1();
    case 'MD5':
      return await createMD5();
    case 'BLAKE2b-512':
      return await createBLAKE2b(512);
    case 'BLAKE3':
      return await createBLAKE3();
    default:
      return await createSHA256();
  }
}

export interface HashOptions {
  onProgress?: (telemetry: HashTelemetry) => void;
  abortSignal?: AbortSignal;
}

export async function computeFileHash(
  file: File | Blob,
  algorithm: AlgorithmType,
  options: HashOptions = {}
): Promise<{ hash: string; elapsedMs: number; averageSpeedBps: number }> {
  const { onProgress, abortSignal } = options;
  const totalBytes = file.size;
  const hasher = await createHasherInstance(algorithm);
  hasher.init();

  let offset = 0;
  const startTime = performance.now();
  let lastPollTime = startTime;
  let lastPollBytes = 0;

  while (offset < totalBytes) {
    if (abortSignal?.aborted) {
      throw new DOMException('Hashing aborted by user', 'AbortError');
    }

    const chunkEnd = Math.min(offset + CHUNK_SIZE, totalBytes);
    const slice = file.slice(offset, chunkEnd);
    const arrayBuffer = await slice.arrayBuffer();
    const uint8View = new Uint8Array(arrayBuffer);

    hasher.update(uint8View);
    offset = chunkEnd;

    const now = performance.now();
    const dt = (now - lastPollTime) / 1000;
    let instantSpeed = 0;

    if (dt >= 0.08 || offset === totalBytes) {
      const bytesInInterval = offset - lastPollBytes;
      instantSpeed = dt > 0 ? bytesInInterval / dt : 0;
      lastPollTime = now;
      lastPollBytes = offset;
    }

    const totalElapsedSecs = Math.max((now - startTime) / 1000, 0.001);
    const averageSpeed = offset / totalElapsedSecs;
    const percentage = totalBytes > 0 ? Math.min((offset / totalBytes) * 100, 100) : 100;

    if (onProgress) {
      onProgress({
        fileSize: totalBytes,
        checkedBytes: offset,
        elapsedMs: Math.round(now - startTime),
        instantSpeedBps: instantSpeed,
        averageSpeedBps: averageSpeed,
        percentage,
      });
    }

    // Yield back to main thread for smooth 60fps UI
    if (offset < totalBytes) {
      await new Promise((resolve) => setTimeout(resolve, 0));
    }
  }

  const hexDigest = hasher.digest('hex');
  const totalElapsedMs = Math.round(performance.now() - startTime);
  const totalElapsedSecs = Math.max(totalElapsedMs / 1000, 0.001);
  const finalAverageSpeed = totalBytes / totalElapsedSecs;

  return {
    hash: hexDigest.toLowerCase(),
    elapsedMs: totalElapsedMs,
    averageSpeedBps: finalAverageSpeed,
  };
}

export function validateExpectedHash(raw: string, algo: AlgorithmType, expectedLen: number): { valid: boolean; error?: string } {
  const trimmed = raw.trim().toLowerCase();
  if (!trimmed) {
    return { valid: false, error: 'Please enter the expected checksum.' };
  }
  if (!/^[0-9a-f]+$/.test(trimmed)) {
    return {
      valid: false,
      error: `Invalid ${algo} checksum — it must contain only hex digits (0-9, a-f).`,
    };
  }
  if (trimmed.length !== expectedLen) {
    return {
      valid: false,
      error: `Invalid ${algo} checksum length — expected ${expectedLen} hex characters, got ${trimmed.length}.`,
    };
  }
  return { valid: true };
}
