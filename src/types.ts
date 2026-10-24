export type AlgorithmType =
  | 'SHA-256'
  | 'SHA-384'
  | 'SHA-512'
  | 'SHA-1'
  | 'MD5'
  | 'BLAKE2b-512'
  | 'BLAKE3';

export interface AlgorithmInfo {
  id: AlgorithmType;
  label: string;
  hexLen: number;
  description: string;
}

export const ALGORITHMS: AlgorithmInfo[] = [
  { id: 'SHA-256', label: 'SHA-256', hexLen: 64, description: 'Standard secure cryptographic hash (256-bit)' },
  { id: 'SHA-384', label: 'SHA-384', hexLen: 96, description: 'NSA Suite B standard (384-bit)' },
  { id: 'SHA-512', label: 'SHA-512', hexLen: 128, description: 'High security cryptographic hash (512-bit)' },
  { id: 'SHA-1', label: 'SHA-1', hexLen: 40, description: 'Legacy 160-bit hash (use with caution)' },
  { id: 'MD5', label: 'MD5', hexLen: 32, description: 'Legacy 128-bit checksum for integrity check only' },
  { id: 'BLAKE2b-512', label: 'BLAKE2b-512', hexLen: 128, description: 'Fast, secure cryptographic hash (512-bit)' },
  { id: 'BLAKE3', label: 'BLAKE3', hexLen: 64, description: 'Extremely fast tree-based cryptographic hash (256-bit)' },
];

export interface HashTelemetry {
  fileSize: number;
  checkedBytes: number;
  elapsedMs: number;
  instantSpeedBps: number;
  averageSpeedBps: number;
  percentage: number;
}

export type VerdictStatus = 'idle' | 'matched' | 'mismatched' | 'calculated' | 'cancelled' | 'error';

export interface VerificationResult {
  status: VerdictStatus;
  algorithm: AlgorithmType;
  computedHash: string;
  expectedHash?: string;
  sourceGuess?: string;
  sourceUrl?: string;
  fileSize: number;
  timeTakenMs: number;
  averageSpeed: number;
  digitalSignature: string;
  errorMessage?: string;
}

export interface SidecarChecksum {
  hash: string;
  algorithmGuess?: AlgorithmType;
  referencedFilename?: string;
}
