import React, { useState, useEffect } from 'react';
import { Loader2, CheckCircle2, AlertCircle, ArrowDownToLine, X } from 'lucide-react';

interface UpdateModalProps {
  isOpen: boolean;
  onClose: () => void;
}

type UpdateStep = 'checking' | 'uptodate' | 'available' | 'downloading' | 'verifying' | 'ready' | 'failed';

export const UpdateModal: React.FC<UpdateModalProps> = ({ isOpen, onClose }) => {
  const [step, setStep] = useState<UpdateStep>('checking');
  const [progress, setProgress] = useState(0);
  const [statusText, setStatusText] = useState('');

  const currentVersion = '1.2.0';
  const remoteVersion = '1.2.0';
  const remoteSize = '988 KB';
  const remoteSha256 = 'A5D74D5A777740934337469D44B8BF4AD43C63F5233D13F2B6C53A8BF211EF6F';
  const changes = [
    'Multi-Algorithm Support: Added SHA-256, SHA-384, SHA-512, SHA-1, MD5, BLAKE2b, and BLAKE3',
    'Dual-Operation Modes: Integrated Calculate and Verify switch functionality',
    'Performance Optimization: Memory chunk loops rewritten for 70% faster hashing',
    'Rich UI Telemetry: Added File Size, Elapsed Time, and real-time MB/s Speed metrics',
    'Native Windows Integration: Implemented direct drag and drop file handling',
    'Hyperlink Callbacks: Added clickable shortcuts for GitHub, Website, and Crypto Support',
    'Verification Engine: High-throughput background worker execution',
  ];

  useEffect(() => {
    if (isOpen) {
      setStep('checking');
      setProgress(0);
      setStatusText('Checking github.com/rouhalamin/checksum/main/newupdate.txt...');

      const timer = setTimeout(() => {
        // Both are 1.2.0, so user is up to date, but we can let them inspect release details
        setStep('uptodate');
      }, 1200);

      return () => clearTimeout(timer);
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const simulateDownload = () => {
    setStep('downloading');
    setProgress(0);
    setStatusText('Downloading update payload...');

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
    }, 180);
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/60 backdrop-blur-xs flex items-center justify-center p-4">
      <div className="bg-[#f6f6f8] text-slate-800 rounded-lg shadow-2xl border border-slate-300 max-w-lg w-full overflow-hidden flex flex-col">
        {/* Title bar */}
        <div className="bg-[#e4e4e8] px-4 py-2.5 border-b border-slate-300 flex items-center justify-between">
          <span className="font-semibold text-slate-800 text-sm">CheckSum - Software Update</span>
          <button
            onClick={onClose}
            className="text-slate-500 hover:text-slate-800 p-0.5 rounded-sm hover:bg-slate-300 transition-colors"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Body */}
        <div className="p-6 space-y-4">
          {step === 'checking' && (
            <div className="text-center py-8 space-y-4">
              <Loader2 className="w-10 h-10 text-blue-600 animate-spin mx-auto" />
              <div>
                <h3 className="font-bold text-slate-800 text-base">Checking for Update...</h3>
                <p className="text-xs text-slate-500 mt-1">{statusText}</p>
              </div>
            </div>
          )}

          {step === 'uptodate' && (
            <div className="space-y-4">
              <div className="flex items-start gap-3">
                <CheckCircle2 className="w-8 h-8 text-emerald-600 shrink-0 mt-0.5" />
                <div>
                  <h3 className="font-bold text-slate-900 text-base">You are up to date</h3>
                  <p className="text-xs text-slate-600 mt-0.5">
                    CheckSum version <span className="font-semibold text-slate-800">v{currentVersion}</span> is the latest version.
                  </p>
                </div>
              </div>

              <div className="bg-white border border-slate-300 rounded p-3 text-xs space-y-2">
                <div className="font-semibold text-slate-700">What's New in v{remoteVersion}:</div>
                <ul className="list-disc list-inside space-y-1 text-slate-600 pl-1">
                  {changes.slice(0, 5).map((change, idx) => (
                    <li key={idx} className="leading-snug">{change}</li>
                  ))}
                </ul>
              </div>

              <div className="text-[11px] text-slate-500 bg-slate-100 p-2 rounded border border-slate-200">
                Official Release SHA-256:
                <code className="block font-mono text-[10px] break-all text-slate-700 mt-0.5">
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

              <div className="bg-white border border-slate-300 rounded p-3 text-xs space-y-2 max-h-48 overflow-y-auto">
                <div className="font-semibold text-slate-800">What's New:</div>
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
              <div className="w-full bg-slate-200 h-4 rounded-sm overflow-hidden border border-slate-300">
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
        <div className="bg-[#ececf0] px-5 py-3 border-t border-slate-300 flex items-center justify-end gap-2">
          {step === 'uptodate' && (
            <>
              <button
                onClick={() => setStep('available')}
                className="px-3 py-1.5 text-xs text-slate-600 hover:text-slate-800 transition-colors mr-auto"
              >
                Re-check Release
              </button>
              <button
                onClick={onClose}
                className="px-5 py-1.5 bg-blue-600 hover:bg-blue-700 text-white rounded text-xs font-semibold transition-colors cursor-pointer"
              >
                Done
              </button>
            </>
          )}

          {step === 'available' && (
            <>
              <button
                onClick={onClose}
                className="px-4 py-1.5 bg-slate-200 hover:bg-slate-300 border border-slate-400 rounded text-xs font-semibold text-slate-800 transition-colors cursor-pointer"
              >
                Cancel
              </button>
              <button
                onClick={simulateDownload}
                className="px-5 py-1.5 bg-blue-600 hover:bg-blue-700 text-white rounded text-xs font-semibold transition-colors cursor-pointer"
              >
                Update Now
              </button>
            </>
          )}

          {(step === 'ready' || step === 'failed') && (
            <button
              onClick={onClose}
              className="px-5 py-1.5 bg-slate-200 hover:bg-slate-300 border border-slate-400 rounded text-xs font-semibold text-slate-800 transition-colors cursor-pointer"
            >
              Close
            </button>
          )}
        </div>
      </div>
    </div>
  );
};
