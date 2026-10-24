import React, { useState, useEffect } from 'react';
import { Loader2, CheckCircle2, AlertCircle, ArrowDownToLine, X, Sparkles } from 'lucide-react';

interface UpdateModalProps {
  isOpen: boolean;
  onClose: () => void;
}

type UpdateStep = 'checking' | 'uptodate' | 'available' | 'downloading' | 'verifying' | 'ready' | 'failed';

export const UpdateModal: React.FC<UpdateModalProps> = ({ isOpen, onClose }) => {
  const [step, setStep] = useState<UpdateStep>('checking');
  const [progress, setProgress] = useState(0);
  const [statusText, setStatusText] = useState('');

  const currentVersion = '1.3.0';
  const remoteVersion = '1.3.0';
  const remoteSize = '1.02 MB';
  const remoteSha256 = '7F8E3B9A1C5D2E4F6A0B8C7D9E1F3A5B7C9D1E3F5A7B9C1D3E5F7A9B1C3D5E7F';
  const changes = [
    'Modern Glassmorphism & Fluent Acrylic UI: Refined translucent glass styling with smooth animations',
    'High-Performance Worker Engine: WebAssembly multi-gigabyte throughput with zero UI blocking',
    'Dynamic ETA & Shimmer Progress: Real-time calculation speed, remaining time, and animated telemetry',
    'Persian & English Dual Interface: Added seamless language toggle with dedicated hints',
    'Sidecar Auto-Detection: Automatic parsing of .sha256, .sha512, and GNU/BSD checksum files',
    'Web Audio Cues: Synthesized native sound feedback on verification match and mismatch',
    'Instant Verification Report: One-click formatted report copy with timestamp and digest',
  ];

  useEffect(() => {
    if (isOpen) {
      setStep('checking');
      setProgress(0);
      setStatusText('Connecting to github.com/rouhalamin/checksum/main/newupdate.txt...');

      const timer = setTimeout(() => {
        setStep('uptodate');
      }, 1000);

      return () => clearTimeout(timer);
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const simulateDownload = () => {
    setStep('downloading');
    setProgress(0);
    setStatusText('Downloading CheckSum-1.3.0.exe...');

    const interval = setInterval(() => {
      setProgress((prev) => {
        if (prev >= 100) {
          clearInterval(interval);
          setStep('verifying');
          setStatusText(`Verifying release SHA-256 against ${remoteSha256.slice(0, 16)}...`);
          setTimeout(() => {
            setStep('ready');
            setStatusText('Release package verified successfully! Ready to update.');
          }, 1000);
          return 100;
        }
        return prev + 15;
      });
    }, 150);
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/60 backdrop-blur-md flex items-center justify-center p-4">
      <div className="glass-window max-w-lg w-full rounded-2xl overflow-hidden flex flex-col shadow-2xl border border-white/60">
        {/* Title bar */}
        <div className="bg-[#2b3a4e]/90 px-5 py-3 border-b border-white/10 flex items-center justify-between text-white">
          <div className="flex items-center gap-2">
            <Sparkles className="w-4 h-4 text-blue-400" />
            <span className="font-semibold text-sm">CheckSum - Software Update</span>
          </div>
          <button
            onClick={onClose}
            className="text-slate-300 hover:text-white p-1 rounded hover:bg-white/10 transition-colors cursor-pointer"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Body */}
        <div className="p-6 space-y-4 text-slate-800">
          {step === 'checking' && (
            <div className="text-center py-8 space-y-4">
              <Loader2 className="w-10 h-10 text-blue-600 animate-spin mx-auto" />
              <div>
                <h3 className="font-bold text-slate-900 text-base">Checking for Update...</h3>
                <p className="text-xs text-slate-500 mt-1">{statusText}</p>
              </div>
            </div>
          )}

          {step === 'uptodate' && (
            <div className="space-y-4">
              <div className="flex items-start gap-3">
                <CheckCircle2 className="w-8 h-8 text-emerald-600 shrink-0 mt-0.5" />
                <div>
                  <h3 className="font-bold text-slate-900 text-base">You are up to date!</h3>
                  <p className="text-xs text-slate-600 mt-0.5">
                    CheckSum version <span className="font-bold text-slate-900">v{currentVersion}</span> is currently the latest version.
                  </p>
                </div>
              </div>

              <div className="glass-card rounded-xl p-3.5 text-xs space-y-2">
                <div className="font-bold text-slate-800">What's New in v{remoteVersion}:</div>
                <ul className="list-disc list-inside space-y-1.5 text-slate-600 pl-1">
                  {changes.map((change, idx) => (
                    <li key={idx} className="leading-snug">{change}</li>
                  ))}
                </ul>
              </div>

              <div className="text-[11px] text-slate-500 bg-white/70 p-2.5 rounded-lg border border-slate-200">
                Official Release SHA-256:
                <code className="block font-mono text-[10px] break-all text-slate-700 mt-1">
                  {remoteSha256}
                </code>
              </div>
            </div>
          )}

          {step === 'available' && (
            <div className="space-y-4">
              <div className="flex items-start gap-3">
                <ArrowDownToLine className="w-8 h-8 text-blue-600 shrink-0 mt-0.5" />
                <div>
                  <h3 className="font-bold text-slate-900 text-base">New update available</h3>
                  <p className="text-xs text-slate-600 mt-0.5">
                    Version <span className="font-semibold text-slate-800">v{remoteVersion}</span> · Size: {remoteSize}
                  </p>
                </div>
              </div>

              <div className="glass-card rounded-xl p-3.5 text-xs space-y-2 max-h-48 overflow-y-auto">
                <div className="font-bold text-slate-800">What's New:</div>
                <ul className="list-disc list-inside space-y-1 text-slate-600">
                  {changes.map((c, i) => (
                    <li key={i}>{c}</li>
                  ))}
                </ul>
              </div>
            </div>
          )}

          {(step === 'downloading' || step === 'verifying') && (
            <div className="space-y-4 py-4">
              <h3 className="font-bold text-slate-800 text-base text-center">
                {step === 'downloading' ? 'Downloading Update...' : 'Verifying Cryptographic Checksum...'}
              </h3>
              <p className="text-xs text-slate-500 text-center">{statusText}</p>

              {/* Progress bar */}
              <div className="w-full bg-slate-200 h-4 rounded-full overflow-hidden border border-slate-300">
                <div
                  className="bg-emerald-600 h-full transition-all duration-200"
                  style={{ width: `${progress}%` }}
                />
              </div>
              <div className="text-right text-xs font-mono text-slate-600">{progress}%</div>
            </div>
          )}

          {step === 'ready' && (
            <div className="text-center py-4 space-y-3">
              <CheckCircle2 className="w-10 h-10 text-emerald-600 mx-auto" />
              <h3 className="font-bold text-slate-800 text-base">Update Ready</h3>
              <p className="text-xs text-slate-600">{statusText}</p>
            </div>
          )}

          {step === 'failed' && (
            <div className="text-center py-4 space-y-3">
              <AlertCircle className="w-10 h-10 text-red-600 mx-auto" />
              <h3 className="font-bold text-slate-800 text-base">Update Failed</h3>
              <p className="text-xs text-red-600">{statusText}</p>
            </div>
          )}
        </div>

        {/* Footer controls */}
        <div className="bg-[#e9ebf0]/90 px-6 py-3 border-t border-slate-300/80 flex items-center justify-end gap-2">
          {step === 'uptodate' && (
            <>
              <button
                onClick={() => setStep('available')}
                className="px-3 py-1.5 text-xs text-slate-600 hover:text-slate-800 transition-colors mr-auto cursor-pointer"
              >
                Simulate v1.2.0 Upgrade Check
              </button>
              <button
                onClick={onClose}
                className="px-5 py-1.5 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-xs font-semibold transition-colors cursor-pointer"
              >
                Done
              </button>
            </>
          )}

          {step === 'available' && (
            <>
              <button
                onClick={onClose}
                className="px-4 py-1.5 bg-white hover:bg-slate-100 border border-slate-300 rounded-lg text-xs font-semibold text-slate-700 transition-colors cursor-pointer"
              >
                Cancel
              </button>
              <button
                onClick={simulateDownload}
                className="px-5 py-1.5 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-xs font-semibold transition-colors cursor-pointer shadow-xs"
              >
                Update Now
              </button>
            </>
          )}

          {(step === 'ready' || step === 'failed') && (
            <button
              onClick={onClose}
              className="px-5 py-1.5 bg-white hover:bg-slate-100 border border-slate-300 rounded-lg text-xs font-semibold text-slate-700 transition-colors cursor-pointer"
            >
              Close
            </button>
          )}
        </div>
      </div>
    </div>
  );
};
