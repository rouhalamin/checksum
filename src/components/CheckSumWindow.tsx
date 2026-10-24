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
  Volume2,
  VolumeX,
  Languages,
  CheckCircle2,
  Share2,
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
import { playSuccessChime, playErrorChime, playClickSound } from '../utils/audio';
import { CryptoModal } from './CryptoModal';
import { UpdateModal } from './UpdateModal';
import { SecurityGuideModal } from './SecurityGuideModal';

export const CheckSumWindow: React.FC = () => {
  // Core State
  const [algorithm, setAlgorithm] = useState<AlgorithmType>('SHA-256');
  const [isVerifyMode, setIsVerifyMode] = useState<boolean>(true);
  const [expectedHash, setExpectedHash] = useState<string>('');
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [selectedFileName, setSelectedFileName] = useState<string>('');
  const [selectedFileSize, setSelectedFileSize] = useState<number>(0);

  // Sound & Language toggles
  const [soundEnabled, setSoundEnabled] = useState<boolean>(true);
  const [isPersian, setIsPersian] = useState<boolean>(false);

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

  // Verdict & Computed Results
  const [verdict, setVerdict] = useState<VerdictStatus>('idle');
  const [computedHash, setComputedHash] = useState<string>('');
  const [verificationDetails, setVerificationDetails] = useState<string>('');
  const [errorMessage, setErrorMessage] = useState<string>('');
  const [copiedHash, setCopiedHash] = useState<boolean>(false);
  const [copiedReport, setCopiedReport] = useState<boolean>(false);

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

  // Calculate dynamic ETA
  const remainingBytes = Math.max(0, telemetry.fileSize - telemetry.checkedBytes);
  const etaSeconds = telemetry.instantSpeedBps > 0 ? remainingBytes / telemetry.instantSpeedBps : 0;

  // Handle file selection
  const handleFilePicked = (file: File) => {
    if (soundEnabled) playClickSound();

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
              isPersian
                ? `فایل چکسام برای "${parsed.referencedFilename}" شناسایی شد. لطفاً فایل هدف را انتخاب کنید.`
                : `Loaded sidecar checksum for "${parsed.referencedFilename}". Please select the target file if not loaded.`
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
    if (soundEnabled) playClickSound();
    try {
      const res = await fetch('/assets/test.txt');
      const blob = await res.blob();
      const file = new File([blob], 'test.txt', { type: 'text/plain' });
      handleFilePicked(file);
      if (isVerifyMode) {
        setExpectedHash('6c3dc6d38e785b9b657497ec069f143714b2d1c68f18579ad46e5deaece25bb5');
      }
    } catch {
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
      if (soundEnabled) playErrorChime();
    }
  };

  // Start hashing
  const startAction = async () => {
    if (soundEnabled) playClickSound();

    if (isHashing) {
      cancelHashing();
      return;
    }

    if (!selectedFile) {
      setErrorMessage(isPersian ? 'لطفاً ابتدا یک فایل معتبر انتخاب کنید.' : 'Please choose a valid file first.');
      setVerdict('error');
      if (soundEnabled) playErrorChime();
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
        setErrorMessage(validation.error || (isPersian ? 'هش نامعتبر است.' : 'Invalid expected checksum.'));
        setVerdict('error');
        if (soundEnabled) playErrorChime();
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
          isPersian
            ? `اطلاعات اعتبارسنجی: هیچ هش اولیه‌ای برای مقایسه ارائه نشده بود — این چکیده ریاضی فایل با الگوریتم ${algorithm} است.`
            : `Verification Information: no expected checksum was provided — this is the file's ${algorithm} digest only.`
        );
        if (soundEnabled) playSuccessChime();
        return;
      }

      const expectedNormalized = expectedHash.trim().toLowerCase();
      const matched = computed === expectedNormalized;

      const sourceDisplay = sourceHeuristic.domain !== 'Unknown'
        ? `${isPersian ? 'منبع احتمالی:' : 'Likely Source:'} ${sourceHeuristic.domain} (${isPersian ? 'اطمینان: بر اساس نام فایل' : 'Confidence: Filename'})`
        : `${isPersian ? 'منبع احتمالی: نامشخص' : 'Likely Source: Unknown'}`;

      if (matched) {
        setVerdict('matched');
        setVerificationDetails(
          isPersian
            ? `اطلاعات تایید: هش کاملاً تطبیق داشت (${algorithm}) · هش مرجع توسط کاربر وارد شده · ${sourceDisplay} · امضای دیجیتال: در دسترس نیست`
            : `Verification Information: Hash matched (${algorithm}) · Expected hash provided by user · ${sourceDisplay} · Digital Signature: Not available`
        );
        if (soundEnabled) playSuccessChime();
      } else {
        setVerdict('mismatched');
        setVerificationDetails(
          isPersian
            ? `اطلاعات هشدار: هش مطابقت نداشت! (${algorithm}) · ${sourceDisplay} · فایل ممکن است خراب شده یا دستکاری شده باشد.`
            : `Verification Information: Hash did NOT match (${algorithm}) · ${sourceDisplay} · Digital Signature: Not available`
        );
        if (soundEnabled) playErrorChime();
      }
    } catch (err: unknown) {
      setIsHashing(false);
      setIsCancelling(false);

      if (err instanceof DOMException && err.name === 'AbortError') {
        setVerdict('cancelled');
        setErrorMessage(isPersian ? 'عملیات لغو شد.' : 'Cancelled.');
        setProgressPct(0);
      } else {
        setVerdict('error');
        setErrorMessage(err instanceof Error ? err.message : (isPersian ? 'خطا در پردازش فایل' : 'An error occurred while hashing'));
        setProgressPct(0);
        if (soundEnabled) playErrorChime();
      }
    }
  };

  // Copy computed hash
  const copyToClipboard = () => {
    if (!computedHash) return;
    navigator.clipboard.writeText(computedHash);
    if (soundEnabled) playClickSound();
    setCopiedHash(true);
    setTimeout(() => setCopiedHash(false), 2000);
  };

  // Copy report
  const copyReport = () => {
    const report = `CheckSum v1.3.0 Integrity Report\nFile: ${selectedFileName} (${formatBytes(selectedFileSize)})\nAlgorithm: ${algorithm}\nComputed Hash: ${computedHash}\nVerdict: ${verdict.toUpperCase()}\nDetails: ${verificationDetails}\nTimestamp: ${new Date().toISOString()}`;
    navigator.clipboard.writeText(report);
    if (soundEnabled) playClickSound();
    setCopiedReport(true);
    setTimeout(() => setCopiedReport(false), 2000);
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
      className="min-h-screen w-full relative flex flex-col items-center justify-center p-3 sm:p-6 overflow-hidden select-none bg-slate-900"
    >
      {/* Dynamic Ambient Glassmorphism Backdrop Orbs */}
      <div className="absolute inset-0 pointer-events-none overflow-hidden">
        <div className="absolute -top-[15%] -left-[10%] w-[55vw] h-[55vw] max-w-[650px] max-h-[650px] rounded-full bg-gradient-to-tr from-blue-600/25 to-indigo-500/20 blur-[100px] ambient-orb-1" />
        <div className="absolute -bottom-[20%] -right-[10%] w-[60vw] h-[60vw] max-w-[700px] max-h-[700px] rounded-full bg-gradient-to-bl from-teal-500/20 via-sky-600/25 to-purple-600/20 blur-[110px] ambient-orb-2" />
        <div className="absolute top-[40%] left-[25%] w-[35vw] h-[35vw] max-w-[450px] max-h-[450px] rounded-full bg-violet-600/15 blur-[90px]" />
      </div>

      {/* Drag & Drop Overlay Indicator */}
      {isDragging && (
        <div className="fixed inset-0 z-50 bg-blue-600/40 backdrop-blur-md border-4 border-dashed border-white/80 flex items-center justify-center transition-all duration-300">
          <div className="glass-card px-8 py-6 rounded-2xl shadow-2xl text-center max-w-sm mx-4 transform scale-105 transition-transform">
            <Upload className="w-14 h-14 text-blue-600 mx-auto animate-bounce mb-3" />
            <div className="font-bold text-slate-900 text-lg">
              {isPersian ? 'فایل را رها کنید' : 'Drop File to Load'}
            </div>
            <div className="text-xs text-slate-600 mt-1">
              {isPersian ? 'پشتیبانی از هر فایلی تا ده‌ها گیگابایت یا فایل‌های چکسام .sha256' : 'Supports any file up to multi-GBs or .sha256 sidecars'}
            </div>
          </div>
        </div>
      )}

      {/* Main CheckSum Glass Window Frame */}
      <div className="relative z-10 w-full max-w-[640px] glass-window rounded-2xl overflow-hidden flex flex-col transition-all duration-300">
        
        {/* Win32 / Modern Glass Title Bar (Matches image_50b98139.png) */}
        <div className="bg-[#2b3a4e]/90 backdrop-blur-md px-4 py-2 flex items-center justify-between text-white border-b border-white/10 shadow-xs">
          <div className="flex items-center gap-2">
            <div className="w-5 h-5 rounded-full bg-blue-500/20 border border-blue-400/40 flex items-center justify-center shadow-xs">
              <img src="/assets/app-logo.png" alt="CheckSum icon" className="w-3.5 h-3.5 object-contain" />
            </div>
            <span className="font-semibold text-xs tracking-wide text-slate-100">CheckSum</span>
          </div>

          <div className="flex items-center gap-2">
            {/* Audio Toggle */}
            <button
              onClick={() => setSoundEnabled(!soundEnabled)}
              className="text-slate-300 hover:text-white p-1 rounded hover:bg-white/10 transition-colors cursor-pointer"
              title={soundEnabled ? 'Mute Audio Cues' : 'Enable Audio Cues'}
            >
              {soundEnabled ? <Volume2 className="w-3.5 h-3.5 text-blue-300" /> : <VolumeX className="w-3.5 h-3.5 text-slate-400" />}
            </button>

            {/* Language Assistance Toggle */}
            <button
              onClick={() => setIsPersian(!isPersian)}
              className={`px-1.5 py-0.5 rounded text-[11px] font-medium transition-colors cursor-pointer flex items-center gap-1 ${
                isPersian ? 'bg-blue-600 text-white' : 'text-slate-300 hover:text-white hover:bg-white/10'
              }`}
              title="تغییر زبان به فارسی / Switch Language"
            >
              <Languages className="w-3 h-3" />
              <span>{isPersian ? 'FA' : 'EN'}</span>
            </button>

            {/* Info Button */}
            <button
              onClick={() => setIsSecurityOpen(true)}
              className="text-slate-300 hover:text-white px-2 py-0.5 rounded text-[11px] hover:bg-white/10 flex items-center gap-1 transition-colors cursor-pointer"
              title="Verification & Security Info"
            >
              <HelpCircle className="w-3.5 h-3.5 text-blue-300" />
              <span>{isPersian ? 'راهنما' : 'Info'}</span>
            </button>

            {/* Classic 3 window dots */}
            <div className="flex items-center gap-1.5 ml-1">
              <div className="w-2.5 h-2.5 rounded-full bg-slate-400/70"></div>
              <div className="w-2.5 h-2.5 rounded-full bg-slate-400/70"></div>
              <div className="w-2.5 h-2.5 rounded-full bg-red-400/90 shadow-2xs"></div>
            </div>
          </div>
        </div>

        {/* Sub-header with Logo & Load test.txt (Matches image_50b98139.png) */}
        <div className="px-6 pt-4 pb-3 border-b border-slate-200/60 bg-white/40 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="relative group">
              <div className="absolute -inset-1 rounded-full bg-blue-500/20 blur-xs group-hover:bg-blue-500/30 transition-all duration-300" />
              <img
                src="/assets/app-logo.png"
                alt="CheckSum"
                className="relative w-9 h-9 object-contain drop-shadow-xs transform group-hover:scale-105 transition-transform"
              />
            </div>
            <div>
              <div className="font-bold text-sm text-slate-900 tracking-tight flex items-center gap-2">
                CheckSum
                <span className="text-[10px] bg-slate-200/80 text-slate-700 px-2 py-0.5 rounded-full font-mono font-semibold border border-slate-300/60 shadow-2xs">
                  v1.3.0
                </span>
              </div>
              <div className="text-[11px] text-slate-500 font-medium">
                {isPersian ? 'ابزار اعتبارسنجی یکپارچگی فایل با رمزنگاری پیشرفته' : 'Native Cryptographic File Integrity Verifier'}
              </div>
            </div>
          </div>

          <button
            onClick={loadTestFile}
            className="text-[11px] px-3 py-1.5 bg-white/90 hover:bg-white text-slate-700 font-medium rounded-lg border border-slate-300/80 hover:border-slate-400 flex items-center gap-1.5 transition-all shadow-2xs hover:shadow-xs cursor-pointer active:scale-95"
            title="Load sample test.txt from repository assets"
          >
            <Sparkles className="w-3.5 h-3.5 text-amber-500" />
            <span>Load test.txt</span>
          </button>
        </div>

        {/* Form Body with Frosted Glass Panels */}
        <div className="p-6 space-y-4 text-xs">

          {/* Section 1: Hash Algorithm & Mode Switcher (Exact replica of image) */}
          <div className="glass-card p-3.5 rounded-xl flex flex-wrap items-center justify-between gap-3 transition-shadow hover:shadow-sm">
            <div className="flex items-center gap-2.5">
              <label htmlFor="algo-select" className="font-semibold text-slate-700 text-xs">
                {isPersian ? 'الگوریتم هش:' : 'Hash Algorithm:'}
              </label>
              <div className="relative">
                <select
                  id="algo-select"
                  value={algorithm}
                  onChange={(e) => {
                    setAlgorithm(e.target.value as AlgorithmType);
                    if (soundEnabled) playClickSound();
                  }}
                  disabled={isHashing}
                  className="bg-white/95 border border-slate-300/90 rounded-lg px-3 py-1 text-xs text-slate-800 font-semibold focus:outline-hidden focus:ring-2 focus:ring-blue-500/30 focus:border-blue-500 shadow-2xs cursor-pointer disabled:opacity-60 appearance-none pr-7"
                >
                  {ALGORITHMS.map((algo) => (
                    <option key={algo.id} value={algo.id}>
                      {algo.label} ({algo.hexLen} hex)
                    </option>
                  ))}
                </select>
                <div className="absolute right-2.5 top-1/2 -translate-y-1/2 pointer-events-none text-slate-500 text-[10px]">
                  ▼
                </div>
              </div>
            </div>

            {/* Mode selection Radio Buttons */}
            <div className="flex items-center gap-4">
              <label className="flex items-center gap-1.5 cursor-pointer font-medium text-slate-700 hover:text-slate-900 transition-colors">
                <input
                  type="radio"
                  name="operation-mode"
                  checked={isVerifyMode}
                  onChange={() => {
                    setIsVerifyMode(true);
                    if (soundEnabled) playClickSound();
                  }}
                  disabled={isHashing}
                  className="w-3.5 h-3.5 text-blue-600 focus:ring-blue-500 cursor-pointer"
                />
                <span>{isPersian ? 'بررسی (Verify)' : 'Verify'}</span>
              </label>
              <label className="flex items-center gap-1.5 cursor-pointer font-medium text-slate-700 hover:text-slate-900 transition-colors">
                <input
                  type="radio"
                  name="operation-mode"
                  checked={!isVerifyMode}
                  onChange={() => {
                    setIsVerifyMode(false);
                    if (soundEnabled) playClickSound();
                  }}
                  disabled={isHashing}
                  className="w-3.5 h-3.5 text-blue-600 focus:ring-blue-500 cursor-pointer"
                />
                <span>{isPersian ? 'محاسبه (Calculate)' : 'Calculate'}</span>
              </label>
            </div>
          </div>

          {/* Section 2: Expected Hash (Verify mode only) */}
          {isVerifyMode && (
            <div className="space-y-1.5 transition-all duration-200">
              <div className="flex items-center justify-between text-xs">
                <label className="font-semibold text-slate-700">
                  {isPersian ? 'هش مورد انتظار:' : 'Expected Hash:'}
                </label>
                <div className="flex items-center gap-2.5 text-[11px] text-slate-500">
                  <span className={`font-mono px-1.5 py-0.2 rounded ${
                    expectedHash.trim().length === currentAlgoInfo.hexLen
                      ? 'bg-emerald-100 text-emerald-800 font-semibold'
                      : expectedHash.trim().length > 0
                      ? 'bg-amber-100 text-amber-800'
                      : 'text-slate-500'
                  }`}>
                    {expectedHash.trim().length}/{currentAlgoInfo.hexLen} hex
                  </span>
                  <button
                    onClick={() => sidecarInputRef.current?.click()}
                    className="text-blue-600 hover:text-blue-800 font-medium hover:underline cursor-pointer transition-colors"
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
                  placeholder={isPersian ? `هش رسمی ${algorithm} را اینجا قرار دهید` : `Paste the official ${algorithm} checksum here`}
                  className="w-full glass-input rounded-lg px-3.5 py-2 text-xs font-mono text-slate-800 placeholder-slate-400 focus:outline-hidden disabled:bg-slate-100/60"
                />
                {expectedHash && (
                  <button
                    onClick={() => setExpectedHash('')}
                    disabled={isHashing}
                    className="absolute right-2.5 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600 p-0.5 rounded-full hover:bg-slate-200/50 transition-colors"
                    title="Clear"
                  >
                    <XCircle className="w-4 h-4" />
                  </button>
                )}
              </div>
            </div>
          )}

          {/* Section 3: File Selection (Matches image) */}
          <div className="space-y-1.5">
            <div className="flex items-center justify-between text-xs">
              <label className="font-semibold text-slate-700">
                {isPersian ? 'فایل (یا هرجا روی این پنجره درگ کنید):' : 'File (or drag & drop it anywhere on this window):'}
              </label>
              {selectedFileSize > 0 && (
                <span className="text-[11px] text-slate-600 font-mono font-medium bg-slate-100 px-1.5 py-0.2 rounded">
                  {formatBytes(selectedFileSize)}
                </span>
              )}
            </div>
            <div className="flex gap-2">
              <input
                type="text"
                readOnly
                value={selectedFileName || ''}
                placeholder={isPersian ? 'مثلاً C:\\Downloads\\example.iso یا اینجا رها کنید' : 'e.g. C:\\Downloads\\example.iso or drag & drop here'}
                className="flex-1 glass-input rounded-lg px-3.5 py-2 text-xs text-slate-800 placeholder-slate-400 focus:outline-hidden truncate"
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
                className="px-4 py-2 bg-white/90 hover:bg-white active:bg-slate-100 border border-slate-300 rounded-lg text-xs font-semibold text-slate-700 transition-all shadow-2xs hover:shadow-xs cursor-pointer disabled:opacity-60 whitespace-nowrap active:scale-98"
              >
                {isPersian ? 'انتخاب...' : 'Browse...'}
              </button>
            </div>
          </div>

          {/* Section 4: Action Button & Shimmer Progress Bar */}
          <div className="flex items-center gap-3 pt-1">
            <button
              onClick={startAction}
              className={`px-6 py-2.5 rounded-lg text-xs font-bold transition-all shadow-sm cursor-pointer min-w-[95px] active:scale-98 flex items-center justify-center ${
                isHashing
                  ? 'bg-red-600 hover:bg-red-700 text-white shadow-red-500/20'
                  : 'bg-[#2b6cb0] hover:bg-[#20538a] text-white shadow-blue-500/20'
              }`}
            >
              {isHashing
                ? (isCancelling ? (isPersian ? 'در حال لغو...' : 'Cancelling...') : (isPersian ? 'لغو' : 'Cancel'))
                : isVerifyMode
                ? (isPersian ? 'تایید اعتبار' : 'Verify')
                : (isPersian ? 'محاسبه هش' : 'Calculate')}
            </button>

            {/* Glowing Shimmer Progress bar */}
            <div className="flex-1 bg-slate-200/80 h-7 rounded-lg border border-slate-300/90 overflow-hidden relative shadow-inner">
              <div
                className={`h-full transition-all duration-150 ${
                  isCancelling || verdict === 'error'
                    ? 'shimmer-cancelling'
                    : verdict === 'mismatched'
                    ? 'bg-red-500'
                    : isHashing
                    ? 'shimmer-active'
                    : 'bg-emerald-600'
                }`}
                style={{ width: `${progressPct}%` }}
              />
              <div className="absolute inset-0 flex items-center justify-center text-[10px] font-mono font-bold text-slate-800 drop-shadow-xs">
                {isHashing
                  ? `${progressPct}%${etaSeconds > 0 && etaSeconds < 3600 ? ` · ETA: ${Math.ceil(etaSeconds)}s` : ''}`
                  : progressPct === 100
                  ? '100%'
                  : ''}
              </div>
            </div>
          </div>

          {/* Section 5: Live Information Grid (5 metrics from image) */}
          <div className="glass-card rounded-xl p-3.5 grid grid-cols-2 sm:grid-cols-3 gap-y-2.5 gap-x-4 text-xs">
            <div>
              <span className="text-slate-500">{isPersian ? 'حجم: ' : 'Size: '}</span>
              <span className="font-semibold text-slate-800">
                {selectedFileSize > 0 ? formatBytes(selectedFileSize) : '—'}
              </span>
            </div>
            <div>
              <span className="text-slate-500">{isPersian ? 'بررسی‌شده: ' : 'Checked: '}</span>
              <span className="font-semibold text-slate-800">
                {telemetry.checkedBytes > 0 ? formatBytes(telemetry.checkedBytes) : '—'}
              </span>
            </div>
            <div>
              <span className="text-slate-500">{isPersian ? 'سرعت: ' : 'Speed: '}</span>
              <span className="font-semibold text-slate-800 font-mono">
                {telemetry.instantSpeedBps > 0
                  ? formatSpeed(telemetry.instantSpeedBps)
                  : telemetry.averageSpeedBps > 0
                  ? `${formatSpeed(telemetry.averageSpeedBps)} (avg)`
                  : '—'}
              </span>
            </div>
            <div>
              <span className="text-slate-500">{isPersian ? 'زمان: ' : 'Time: '}</span>
              <span className="font-semibold text-slate-800 font-mono">
                {telemetry.elapsedMs > 0 ? formatElapsed(telemetry.elapsedMs / 1000) : '0.0s'}
              </span>
            </div>
            <div className="col-span-2">
              <span className="text-slate-500">{isPersian ? 'منبع احتمالی: ' : 'Likely Source: '}</span>
              {sourceHeuristic.url ? (
                <a
                  href={sourceHeuristic.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="font-semibold text-blue-700 hover:text-blue-900 hover:underline inline-flex items-center gap-1"
                >
                  {sourceHeuristic.domain}
                  <ExternalLink className="w-2.5 h-2.5" />
                  <span className="text-[10px] text-slate-500 font-normal">
                    ({isPersian ? 'اطمینان: نام فایل' : 'Confidence: Filename'})
                  </span>
                </a>
              ) : (
                <span className="font-semibold text-slate-800">
                  {sourceHeuristic.domain}
                </span>
              )}
            </div>
          </div>

          {/* Section 6: Results Banner & Detailed Verification Breakdown */}
          {verdict !== 'idle' && (
            <div className="space-y-2 transform transition-all duration-300">
              {verdict === 'matched' && (
                <div className="p-3.5 bg-emerald-50/90 border border-emerald-300/80 rounded-xl shadow-xs text-emerald-950 flex items-start gap-3">
                  <CheckCircle2 className="w-6 h-6 text-emerald-600 shrink-0 mt-0.5 animate-pulse" />
                  <div className="flex-1">
                    <div className="font-bold text-sm text-emerald-800 flex items-center justify-between">
                      <span>✓ {isPersian ? 'هش تطبیق یافت — اصالت فایل تایید شد' : 'Hash matched — File integrity verified'}</span>
                      <button
                        onClick={copyReport}
                        className="text-[10px] font-normal text-emerald-700 hover:text-emerald-900 flex items-center gap-1 bg-emerald-100/80 px-2 py-0.5 rounded cursor-pointer"
                        title="Copy full verification report"
                      >
                        <Share2 className="w-3 h-3" />
                        <span>{copiedReport ? (isPersian ? 'کپی شد' : 'Copied') : (isPersian ? 'کپی گزارش' : 'Copy Report')}</span>
                      </button>
                    </div>
                    <div className="text-[11px] text-emerald-700 mt-1 leading-relaxed">
                      {verificationDetails}
                    </div>
                  </div>
                </div>
              )}

              {verdict === 'mismatched' && (
                <div className="p-3.5 bg-red-50/90 border border-red-300/80 rounded-xl shadow-xs text-red-950 flex items-start gap-3">
                  <AlertCircle className="w-6 h-6 text-red-600 shrink-0 mt-0.5" />
                  <div className="flex-1">
                    <div className="font-bold text-sm text-red-700">
                      ✕ {isPersian ? 'عدم تطابق هش — فایل با چکسام مورد انتظار یکی نیست' : 'Hash mismatch — The file does not match the expected checksum'}
                    </div>
                    <div className="text-[11px] text-red-700 mt-1 leading-relaxed">
                      {verificationDetails}
                    </div>
                  </div>
                </div>
              )}

              {verdict === 'calculated' && (
                <div className="p-3.5 bg-blue-50/90 border border-blue-300/80 rounded-xl shadow-xs text-blue-950 flex items-start gap-3">
                  <FileCheck className="w-6 h-6 text-blue-600 shrink-0 mt-0.5" />
                  <div className="flex-1">
                    <div className="font-bold text-sm text-blue-800">
                      {algorithm} {isPersian ? 'با موفقیت محاسبه شد.' : 'computed successfully.'}
                    </div>
                    <div className="text-[11px] text-blue-700 mt-1 leading-relaxed">
                      {verificationDetails}
                    </div>
                  </div>
                </div>
              )}

              {verdict === 'cancelled' && (
                <div className="p-3 bg-slate-100 border border-slate-300 rounded-xl text-slate-700 text-xs font-semibold">
                  {errorMessage || (isPersian ? 'عملیات لغو شد.' : 'Cancelled.')}
                </div>
              )}

              {verdict === 'error' && errorMessage && (
                <div className="p-3 bg-red-50 border border-red-300 rounded-xl text-red-800 text-xs font-medium">
                  {errorMessage}
                </div>
              )}
            </div>
          )}

          {/* Section 7: Computed Hash Display with Micro-interaction Copy */}
          <div className="space-y-1.5">
            <label className="font-semibold text-slate-700 text-xs">
              {isPersian ? 'هش محاسبه‌شده:' : 'Computed Hash:'}
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                readOnly
                value={computedHash}
                placeholder={isPersian ? 'چکیده هش پس از پایان محاسبه اینجا ظاهر می‌شود' : 'Checksum digest will appear here upon completion'}
                className="flex-1 glass-input rounded-lg px-3.5 py-2 text-xs font-mono text-slate-900 select-all"
              />
              <button
                onClick={copyToClipboard}
                disabled={!computedHash}
                className={`px-4 py-2 border rounded-lg text-xs font-semibold flex items-center gap-1.5 transition-all cursor-pointer shadow-2xs active:scale-98 ${
                  copiedHash
                    ? 'bg-emerald-50 border-emerald-300 text-emerald-700'
                    : 'bg-white/90 hover:bg-white border-slate-300 text-slate-700 disabled:opacity-40'
                }`}
              >
                {copiedHash ? (
                  <>
                    <Check className="w-3.5 h-3.5 text-emerald-600" />
                    <span>{isPersian ? 'کپی شد' : 'Copied'}</span>
                  </>
                ) : (
                  <>
                    <Copy className="w-3.5 h-3.5 text-slate-600" />
                    <span>{isPersian ? 'کپی' : 'Copy'}</span>
                  </>
                )}
              </button>
            </div>
          </div>
        </div>

        {/* Footer Bar (Clean Spacing, No Overlaps, Exactly as Expected) */}
        <div className="bg-[#e9ebf0]/95 backdrop-blur-md px-6 py-3 border-t border-slate-300/80 flex flex-wrap items-center justify-between gap-y-2 text-xs text-slate-600">
          <div className="flex items-center gap-3">
            <span className="text-slate-500 font-medium">by Rohulamin Erfani</span>
            <span className="text-slate-400">·</span>
            <span className="text-slate-500 font-mono font-semibold">V 1.3.0</span>
          </div>

          <div className="flex items-center gap-4 text-xs font-medium">
            <button
              onClick={() => {
                if (soundEnabled) playClickSound();
                setIsUpdateOpen(true);
              }}
              className="text-blue-700 hover:text-blue-900 hover:underline cursor-pointer flex items-center gap-1 transition-colors"
            >
              <RefreshCw className="w-3 h-3 text-blue-600" />
              <span>Check Update</span>
            </button>
            <a
              href="https://github.com/rouhalamin/checksum"
              target="_blank"
              rel="noopener noreferrer"
              className="text-blue-700 hover:text-blue-900 hover:underline transition-colors"
            >
              Github
            </a>
            <a
              href="https://checksumapp.netlify.app/"
              target="_blank"
              rel="noopener noreferrer"
              className="text-blue-700 hover:text-blue-900 hover:underline transition-colors"
            >
              CheckSum-site
            </a>
            <button
              onClick={() => {
                if (soundEnabled) playClickSound();
                setIsCryptoOpen(true);
              }}
              className="text-blue-700 hover:text-blue-900 hover:underline cursor-pointer transition-colors"
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
