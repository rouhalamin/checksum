import React, { useState, useRef } from 'react';
import {
  Copy,
  Check,
  FileCheck,
  Upload,
  AlertCircle,
  ExternalLink,
  HelpCircle,
  RefreshCw,
  XCircle,
  Sparkles,
} from 'lucide-react';
import {
  AlgorithmType,
  ALGORITHMS,
  HashTelemetry,
  VerdictStatus,
} from '../types';
import {
  computeFileHash,
  validateExpectedHash,
} from '../utils/hasher';
import {
  guessSource,
  formatBytes,
  formatSpeed,
  formatElapsed,
} from '../utils/sourceGuess';
import {
  isChecksumSidecar,
  parseChecksumFile,
} from '../utils/sidecar';
import { CryptoModal } from './CryptoModal';
import { UpdateModal } from './UpdateModal';
import { SecurityGuideModal } from './SecurityGuideModal';

export const CheckSumWindow: React.FC = () => {
  // State
  const [algorithm, setAlgorithm] = useState<AlgorithmType>('SHA-256');
  const [isVerifyMode, setIsVerifyMode] = useState<boolean>(true);
  const [expectedHash, setExpectedHash] = useState<string>('');
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [selectedFileName, setSelectedFileName] = useState<string>('');
  const [selectedFileSize, setSelectedFileSize] = useState<number>(0);

  // Hashing runtime
  const [isHashing, setIsHashing] = useState<boolean>(false);
  const [isCancelling, setIsCancelling] = useState<boolean>(false);
  const [progressPct, setProgressPct] = useState<number>(0);
  const [telemetry, setTelemetry] = useState<HashTelemetry>({
    fileSize: 0,
    checkedBytes: 0,
    elapsedMs: 0,
    instantSpeedBps: 0,
    averageSpeedBps: 0,
    percentage: 0,
  });

  // Verdict & Computed
  const [verdict, setVerdict] = useState<VerdictStatus>('idle');
  const [computedHash, setComputedHash] = useState<string>('');
  const [verificationDetails, setVerificationDetails] = useState<string>('');
  const [errorMessage, setErrorMessage] = useState<string>('');
  const [copiedHash, setCopiedHash] = useState<boolean>(false);

  // Modals
  const [isCryptoOpen, setIsCryptoOpen] = useState<boolean>(false);
  const [isUpdateOpen, setIsUpdateOpen] = useState<boolean>(false);
  const [isSecurityOpen, setIsSecurityOpen] = useState<boolean>(false);

  // Drag & drop state
  const [isDragging, setIsDragging] = useState<boolean>(false);

  // Abort controller
  const abortControllerRef = useRef<AbortController | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const sidecarInputRef = useRef<HTMLInputElement>(null);

  // Current algorithm metadata
  const currentAlgoInfo = ALGORITHMS.find((a) => a.id === algorithm) || ALGORITHMS[0];

  // Source heuristic
  const sourceHeuristic = selectedFileName
    ? guessSource(selectedFileName)
    : { domain: 'Unknown', url: null };

  // Handle file selection
  const handleFilePicked = (file: File) => {
    // Check if user dropped or picked a checksum sidecar file (.sha256, etc.)
    if (isChecksumSidecar(file.name)) {
      const reader = new FileReader();
      reader.onload = (e) => {
        try {
          const content = e.target?.result as string;
          const parsed = parseChecksumFile(content, file.name);
          setExpectedHash(parsed.hash);
          if (parsed.algorithmGuess) {
            setAlgorithm(parsed.algorithmGuess);
          }
          setIsVerifyMode(true);
          setErrorMessage('');
          setVerdict('idle');
          if (parsed.referencedFilename) {
            setVerificationDetails(
              `Loaded sidecar checksum for "${parsed.referencedFilename}". Please select the target file if not loaded.`
            );
          }
        } catch (err: unknown) {
          setErrorMessage(err instanceof Error ? err.message : 'Failed to parse checksum file');
          setVerdict('error');
        }
      };
      reader.readAsText(file);
      return;
    }

    setSelectedFile(file);
    setSelectedFileName(file.name);
    setSelectedFileSize(file.size);
    setVerdict('idle');
    setComputedHash('');
    setVerificationDetails('');
    setErrorMessage('');
    setProgressPct(0);
    setTelemetry({
      fileSize: file.size,
      checkedBytes: 0,
      elapsedMs: 0,
      instantSpeedBps: 0,
      averageSpeedBps: 0,
      percentage: 0,
    });
  };

  // Load sample test file
  const loadTestFile = async () => {
    try {
      const res = await fetch('/assets/test.txt');
      const blob = await res.blob();
      const file = new File([blob], 'test.txt', { type: 'text/plain' });
      handleFilePicked(file);
      if (isVerifyMode && !expectedHash) {
        // Pre-fill expected SHA-256 for test.txt ("Hello welcome to my Github\n")
        // Echo "Hello welcome to my Github\n" sha256 is computed dynamically
        setExpectedHash('6c3dc6d38e785b9b657497ec069f143714b2d1c68f18579ad46e5deaece25bb5');
      }
    } catch {
      // Fallback in-memory
      const content = 'Hello welcome to my Github\n';
      const file = new File([content], 'test.txt', { type: 'text/plain' });
      handleFilePicked(file);
    }
  };

  // Cancel hashing
  const cancelHashing = () => {
    if (abortControllerRef.current) {
      setIsCancelling(true);
      abortControllerRef.current.abort();
    }
  };

  // Start hashing
  const startAction = async () => {
    if (isHashing) {
      cancelHashing();
      return;
    }

    if (!selectedFile) {
      setErrorMessage('Please choose a valid file first.');
      setVerdict('error');
      return;
    }

    // Validate expected hash in Verify mode
    if (isVerifyMode) {
      const validation = validateExpectedHash(
        expectedHash,
        algorithm,
        currentAlgoInfo.hexLen
      );
      if (!validation.valid) {
        setErrorMessage(validation.error || 'Invalid expected checksum.');
        setVerdict('error');
        return;
      }
    }

    // Reset run state
    setErrorMessage('');
    setVerdict('idle');
    setComputedHash('');
    setVerificationDetails('');
    setProgressPct(0);
    setIsHashing(true);
    setIsCancelling(false);

    abortControllerRef.current = new AbortController();

    try {
      const result = await computeFileHash(selectedFile, algorithm, {
        abortSignal: abortControllerRef.current.signal,
        onProgress: (t) => {
          setTelemetry(t);
          setProgressPct(Math.round(t.percentage));
        },
      });

      const computed = result.hash.toLowerCase();
      setComputedHash(computed);
      setProgressPct(100);
      setIsHashing(false);
      setIsCancelling(false);

      if (!isVerifyMode) {
        setVerdict('calculated');
        setVerificationDetails(
          `Verification Information: no expected checksum was provided — this is the file's ${algorithm} digest only.`
        );
        return;
      }

      const expectedNormalized = expectedHash.trim().toLowerCase();
      const matched = computed === expectedNormalized;

      const sourceDisplay = sourceHeuristic.domain !== 'Unknown'
        ? `Likely Source: ${sourceHeuristic.domain} (Confidence: Filename)`
        : 'Likely Source: Unknown';

      if (matched) {
        setVerdict('matched');
        setVerificationDetails(
          `Verification Information: Hash matched (${algorithm}) · Expected hash provided by user · ${sourceDisplay} · Digital Signature: Not available`
        );
      } else {
        setVerdict('mismatched');
        setVerificationDetails(
          `Verification Information: Hash did NOT match (${algorithm}) · ${sourceDisplay} · Digital Signature: Not available`
        );
      }
    } catch (err: unknown) {
      setIsHashing(false);
      setIsCancelling(false);

      if (err instanceof DOMException && err.name === 'AbortError') {
        setVerdict('cancelled');
        setErrorMessage('Cancelled.');
        setProgressPct(0);
      } else {
        setVerdict('error');
        setErrorMessage(err instanceof Error ? err.message : 'An error occurred while hashing');
        setProgressPct(0);
      }
    }
  };

  // Copy computed hash
  const copyToClipboard = () => {
    if (!computedHash) return;
    navigator.clipboard.writeText(computedHash);
    setCopiedHash(true);
    setTimeout(() => setCopiedHash(false), 2000);
  };

  // Window drag & drop listeners
  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(true);
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);

    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      handleFilePicked(e.dataTransfer.files[0]);
    }
  };

  return (
    <div
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      className="min-h-screen bg-[#dce1e8] py-8 px-4 flex flex-col items-center justify-center relative select-none"
    >
      {/* Drop overlay indicator */}
      {isDragging && (
        <div className="fixed inset-0 z-40 bg-blue-600/30 backdrop-blur-xs border-4 border-dashed border-blue-600 flex items-center justify-center pointer-events-none">
          <div className="bg-white px-6 py-4 rounded-lg shadow-xl text-center">
            <Upload className="w-12 h-12 text-blue-600 mx-auto animate-bounce mb-2" />
            <div className="font-bold text-slate-800 text-lg">Drop file anywhere to load</div>
            <div className="text-xs text-slate-500 mt-1">Accepts any file or .sha256 sidecar checksums</div>
          </div>
        </div>
      )}

      {/* Main CheckSum Dialog Window */}
      <div className="w-full max-w-[620px] bg-[#f6f6f8] text-slate-800 rounded-md shadow-2xl border border-slate-300 overflow-hidden flex flex-col">
        {/* Win32 Window Title Bar */}
        <div className="bg-gradient-to-r from-[#2c405a] to-[#3a587d] px-3 py-1.5 flex items-center justify-between text-white border-b border-[#23354b]">
          <div className="flex items-center gap-2">
            <img src="/assets/app-logo.png" alt="CheckSum icon" className="w-4 h-4 object-contain" />
            <span className="font-semibold text-xs tracking-wide">CheckSum</span>
          </div>
          <div className="flex items-center gap-1.5 text-xs">
            <button
              onClick={() => setIsSecurityOpen(true)}
              className="text-slate-200 hover:text-white px-1.5 py-0.5 rounded text-[11px] hover:bg-white/10 flex items-center gap-1 cursor-pointer"
              title="Verification & Security Info"
            >
              <HelpCircle className="w-3.5 h-3.5" />
              <span>Info</span>
            </button>
            <div className="w-2.5 h-2.5 rounded-full bg-slate-400 opacity-60"></div>
            <div className="w-2.5 h-2.5 rounded-full bg-slate-400 opacity-60"></div>
            <div className="w-2.5 h-2.5 rounded-full bg-red-400 opacity-80"></div>
          </div>
        </div>

        {/* Branding Sub-header */}
        <div className="px-5 pt-3 pb-2 border-b border-slate-200 bg-white/70 flex items-center justify-between">
          <div className="flex items-center gap-2.5">
            <img src="/assets/app-logo.png" alt="CheckSum" className="w-8 h-8 object-contain" />
            <div>
              <div className="font-bold text-sm text-slate-900 tracking-tight flex items-center gap-2">
                CheckSum
                <span className="text-[10px] bg-slate-200 text-slate-700 px-1.5 py-0.2 rounded font-mono font-normal">
                  v1.2.0
                </span>
              </div>
              <div className="text-[11px] text-slate-500">
                Native Cryptographic File Integrity Verifier
              </div>
            </div>
          </div>
          <button
            onClick={loadTestFile}
            className="text-[11px] px-2.5 py-1 bg-slate-100 hover:bg-slate-200 text-slate-700 rounded border border-slate-300 flex items-center gap-1 transition-colors cursor-pointer"
            title="Load sample test.txt from repository assets"
          >
            <Sparkles className="w-3 h-3 text-amber-600" />
            <span>Load test.txt</span>
          </button>
        </div>

        {/* Content Form Body */}
        <div className="p-5 space-y-4 text-xs">
          {/* Section 1: Algorithm & Mode */}
          <div className="flex flex-wrap items-center justify-between gap-3 bg-white p-3 rounded border border-slate-200">
            <div className="flex items-center gap-2.5">
              <label htmlFor="algo-select" className="font-semibold text-slate-700 text-xs">
                Hash Algorithm:
              </label>
              <select
                id="algo-select"
                value={algorithm}
                onChange={(e) => setAlgorithm(e.target.value as AlgorithmType)}
                disabled={isHashing}
                className="bg-white border border-slate-300 rounded px-2.5 py-1 text-xs text-slate-800 font-medium focus:outline-hidden focus:border-blue-500 shadow-2xs cursor-pointer disabled:opacity-60"
              >
                {ALGORITHMS.map((algo) => (
                  <option key={algo.id} value={algo.id}>
                    {algo.label} ({algo.hexLen} hex)
                  </option>
                ))}
              </select>
            </div>

            {/* Mode selection Radio Buttons */}
            <div className="flex items-center gap-4">
              <label className="flex items-center gap-1.5 cursor-pointer font-medium text-slate-700">
                <input
                  type="radio"
                  name="operation-mode"
                  checked={isVerifyMode}
                  onChange={() => setIsVerifyMode(true)}
                  disabled={isHashing}
                  className="text-blue-600 focus:ring-blue-500 cursor-pointer"
                />
                <span>Verify</span>
              </label>
              <label className="flex items-center gap-1.5 cursor-pointer font-medium text-slate-700">
                <input
                  type="radio"
                  name="operation-mode"
                  checked={!isVerifyMode}
                  onChange={() => setIsVerifyMode(false)}
                  disabled={isHashing}
                  className="text-blue-600 focus:ring-blue-500 cursor-pointer"
                />
                <span>Calculate</span>
              </label>
            </div>
          </div>

          {/* Section 2: Expected Hash (Verify mode only) */}
          {isVerifyMode && (
            <div className="space-y-1">
              <div className="flex items-center justify-between text-xs">
                <label className="font-semibold text-slate-700">
                  Expected Hash:
                </label>
                <div className="flex items-center gap-2 text-[11px] text-slate-500">
                  <span className="font-mono">
                    {expectedHash.trim().length}/{currentAlgoInfo.hexLen} hex
                  </span>
                  <button
                    onClick={() => sidecarInputRef.current?.click()}
                    className="text-blue-600 hover:underline cursor-pointer"
                    title="Import checksum from .sha256 or sidecar file"
                  >
                    Import .sha256
                  </button>
                  <input
                    ref={sidecarInputRef}
                    type="file"
                    className="hidden"
                    accept=".sha256,.sha384,.sha512,.sha1,.md5,.b2,.blake2,.blake3,.checksum,.sum,.sfv,.txt"
                    onChange={(e) => {
                      if (e.target.files && e.target.files[0]) {
                        handleFilePicked(e.target.files[0]);
                      }
                    }}
                  />
                </div>
              </div>
              <div className="relative">
                <input
                  type="text"
                  value={expectedHash}
                  onChange={(e) => {
                    setExpectedHash(e.target.value);
                    if (verdict !== 'idle') setVerdict('idle');
                  }}
                  disabled={isHashing}
                  placeholder={`Paste the official ${algorithm} checksum here`}
                  className="w-full bg-white border border-slate-300 rounded px-3 py-1.5 text-xs font-mono text-slate-800 placeholder-slate-400 focus:outline-hidden focus:border-blue-500 shadow-2xs disabled:bg-slate-100"
                />
                {expectedHash && (
                  <button
                    onClick={() => setExpectedHash('')}
                    disabled={isHashing}
                    className="absolute right-2 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600 p-0.5"
                    title="Clear"
                  >
                    <XCircle className="w-3.5 h-3.5" />
                  </button>
                )}
              </div>
            </div>
          )}

          {/* Section 3: File Input */}
          <div className="space-y-1">
            <div className="flex items-center justify-between text-xs">
              <label className="font-semibold text-slate-700">
                File (or drag & drop it anywhere on this window):
              </label>
              {selectedFileSize > 0 && (
                <span className="text-[11px] text-slate-500 font-mono">
                  {formatBytes(selectedFileSize)}
                </span>
              )}
            </div>
            <div className="flex gap-2">
              <input
                type="text"
                readOnly
                value={selectedFileName || ''}
                placeholder="e.g. C:\Downloads\example.iso or drag & drop here"
                className="flex-1 bg-white border border-slate-300 rounded px-3 py-1.5 text-xs text-slate-800 placeholder-slate-400 focus:outline-hidden shadow-2xs truncate"
              />
              <input
                ref={fileInputRef}
                type="file"
                className="hidden"
                onChange={(e) => {
                  if (e.target.files && e.target.files[0]) {
                    handleFilePicked(e.target.files[0]);
                  }
                }}
              />
              <button
                onClick={() => fileInputRef.current?.click()}
                disabled={isHashing}
                className="px-4 py-1.5 bg-slate-100 hover:bg-slate-200 active:bg-slate-300 border border-slate-300 rounded text-xs font-semibold text-slate-700 transition-colors shadow-2xs cursor-pointer disabled:opacity-60 whitespace-nowrap"
              >
                Browse...
              </button>
            </div>
          </div>

          {/* Section 4: Action & Progress */}
          <div className="flex items-center gap-3 pt-1">
            <button
              onClick={startAction}
              className={`px-6 py-2 rounded text-xs font-bold transition-all shadow-xs cursor-pointer ${
                isHashing
                  ? 'bg-red-600 hover:bg-red-700 text-white'
                  : 'bg-[#2b6cb0] hover:bg-[#23588f] text-white active:scale-98'
              }`}
            >
              {isHashing ? (isCancelling ? 'Cancelling...' : 'Cancel') : isVerifyMode ? 'Verify' : 'Calculate'}
            </button>

            {/* Win32 themed progress bar */}
            <div className="flex-1 bg-slate-200 h-6 rounded-sm border border-slate-300 overflow-hidden relative shadow-inner">
              <div
                className={`h-full transition-all duration-150 ${
                  isCancelling || verdict === 'error'
                    ? 'bg-red-600'
                    : verdict === 'mismatched'
                    ? 'bg-red-600'
                    : 'bg-emerald-600'
                }`}
                style={{ width: `${progressPct}%` }}
              />
              <div className="absolute inset-0 flex items-center justify-center text-[10px] font-mono font-semibold text-slate-800 drop-shadow-xs">
                {isHashing ? `${progressPct}%` : progressPct === 100 ? '100%' : ''}
              </div>
            </div>
          </div>

          {/* Section 5: Live Information Grid */}
          <div className="bg-white border border-slate-200 rounded p-3 grid grid-cols-2 sm:grid-cols-3 gap-y-2 gap-x-4 text-xs">
            <div>
              <span className="text-slate-500">Size: </span>
              <span className="font-semibold text-slate-800">
                {selectedFileSize > 0 ? formatBytes(selectedFileSize) : '—'}
              </span>
            </div>
            <div>
              <span className="text-slate-500">Checked: </span>
              <span className="font-semibold text-slate-800">
                {telemetry.checkedBytes > 0 ? formatBytes(telemetry.checkedBytes) : '—'}
              </span>
            </div>
            <div>
              <span className="text-slate-500">Speed: </span>
              <span className="font-semibold text-slate-800">
                {telemetry.instantSpeedBps > 0
                  ? formatSpeed(telemetry.instantSpeedBps)
                  : telemetry.averageSpeedBps > 0
                  ? `${formatSpeed(telemetry.averageSpeedBps)} (avg)`
                  : '—'}
              </span>
            </div>
            <div>
              <span className="text-slate-500">Time: </span>
              <span className="font-semibold text-slate-800">
                {telemetry.elapsedMs > 0 ? formatElapsed(telemetry.elapsedMs / 1000) : '0.0s'}
              </span>
            </div>
            <div className="col-span-2">
              <span className="text-slate-500">Likely Source: </span>
              {sourceHeuristic.url ? (
                <a
                  href={sourceHeuristic.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="font-semibold text-blue-700 hover:underline inline-flex items-center gap-1"
                >
                  {sourceHeuristic.domain}
                  <ExternalLink className="w-2.5 h-2.5" />
                  <span className="text-[10px] text-slate-500 font-normal">
                    (Confidence: Filename)
                  </span>
                </a>
              ) : (
                <span className="font-semibold text-slate-800">
                  {sourceHeuristic.domain}
                </span>
              )}
            </div>
          </div>

          {/* Section 6: Results Banner & Verification Breakdown */}
          {verdict !== 'idle' && (
            <div className="space-y-2 transition-all duration-200">
              {verdict === 'matched' && (
                <div className="p-3 bg-emerald-50 border-l-4 border-emerald-600 rounded-r text-emerald-950 flex items-start gap-2.5">
                  <Check className="w-5 h-5 text-emerald-700 shrink-0 mt-0.5" />
                  <div>
                    <div className="font-bold text-sm text-emerald-800">
                      ✓ Hash matched — File integrity verified
                    </div>
                    <div className="text-[11px] text-emerald-700 mt-1 leading-relaxed">
                      {verificationDetails}
                    </div>
                  </div>
                </div>
              )}

              {verdict === 'mismatched' && (
                <div className="p-3 bg-red-50 border-l-4 border-red-600 rounded-r text-red-950 flex items-start gap-2.5">
                  <AlertCircle className="w-5 h-5 text-red-700 shrink-0 mt-0.5" />
                  <div>
                    <div className="font-bold text-sm text-red-700">
                      ✕ Hash mismatch — The file does not match the expected checksum
                    </div>
                    <div className="text-[11px] text-red-700 mt-1 leading-relaxed">
                      {verificationDetails}
                    </div>
                  </div>
                </div>
              )}

              {verdict === 'calculated' && (
                <div className="p-3 bg-blue-50 border-l-4 border-blue-600 rounded-r text-blue-950 flex items-start gap-2.5">
                  <FileCheck className="w-5 h-5 text-blue-700 shrink-0 mt-0.5" />
                  <div>
                    <div className="font-bold text-sm text-blue-800">
                      {algorithm} computed successfully.
                    </div>
                    <div className="text-[11px] text-blue-700 mt-1 leading-relaxed">
                      {verificationDetails}
                    </div>
                  </div>
                </div>
              )}

              {verdict === 'cancelled' && (
                <div className="p-2.5 bg-slate-100 border-l-4 border-slate-400 rounded-r text-slate-700 text-xs font-semibold">
                  Cancelled.
                </div>
              )}

              {verdict === 'error' && errorMessage && (
                <div className="p-2.5 bg-red-50 border-l-4 border-red-500 rounded-r text-red-800 text-xs font-medium">
                  {errorMessage}
                </div>
              )}
            </div>
          )}

          {/* Section 7: Computed Hash Display */}
          <div className="space-y-1">
            <label className="font-semibold text-slate-700 text-xs">
              Computed Hash:
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                readOnly
                value={computedHash}
                placeholder="Checksum digest will appear here upon completion"
                className="flex-1 bg-white border border-slate-300 rounded px-3 py-1.5 text-xs font-mono text-slate-900 select-all shadow-2xs"
              />
              <button
                onClick={copyToClipboard}
                disabled={!computedHash}
                className="px-4 py-1.5 bg-slate-100 hover:bg-slate-200 border border-slate-300 rounded text-xs font-semibold text-slate-700 flex items-center gap-1.5 transition-colors cursor-pointer disabled:opacity-50"
              >
                {copiedHash ? (
                  <>
                    <Check className="w-3.5 h-3.5 text-emerald-600" />
                    <span className="text-emerald-700">Copied</span>
                  </>
                ) : (
                  <>
                    <Copy className="w-3.5 h-3.5 text-slate-600" />
                    <span>Copy</span>
                  </>
                )}
              </button>
            </div>
          </div>
        </div>

        {/* Footer Band (Authentic Win32 Grey Footer Bar) */}
        <div className="bg-[#e8e8ec] px-5 py-3 border-t border-slate-300 flex flex-wrap items-center justify-between gap-y-2 text-xs text-slate-600">
          <div className="flex items-center gap-3">
            <span className="text-slate-500">by Rohulamin Erfani</span>
            <span className="text-slate-400">·</span>
            <span className="text-slate-500 font-mono">V 1.2.0</span>
          </div>

          <div className="flex items-center gap-4 text-xs font-medium">
            <button
              onClick={() => setIsUpdateOpen(true)}
              className="text-blue-700 hover:text-blue-900 hover:underline cursor-pointer flex items-center gap-1"
            >
              <RefreshCw className="w-3 h-3" />
              <span>Check Update</span>
            </button>
            <a
              href="https://github.com/rouhalamin/checksum"
              target="_blank"
              rel="noopener noreferrer"
              className="text-blue-700 hover:text-blue-900 hover:underline inline-flex items-center gap-0.5"
            >
              Github
            </a>
            <a
              href="https://checksumapp.netlify.app/"
              target="_blank"
              rel="noopener noreferrer"
              className="text-blue-700 hover:text-blue-900 hover:underline inline-flex items-center gap-0.5"
            >
              CheckSum-site
            </a>
            <button
              onClick={() => setIsCryptoOpen(true)}
              className="text-blue-700 hover:text-blue-900 hover:underline cursor-pointer"
            >
              Support via Crypto
            </button>
          </div>
        </div>
      </div>

      {/* Dialog Modals */}
      <CryptoModal isOpen={isCryptoOpen} onClose={() => setIsCryptoOpen(false)} />
      <UpdateModal isOpen={isUpdateOpen} onClose={() => setIsUpdateOpen(false)} />
      <SecurityGuideModal isOpen={isSecurityOpen} onClose={() => setIsSecurityOpen(false)} />
    </div>
  );
};
