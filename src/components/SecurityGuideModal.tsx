import React from 'react';
import { ShieldAlert, ShieldCheck, X, Terminal } from 'lucide-react';

interface SecurityGuideModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const SecurityGuideModal: React.FC<SecurityGuideModalProps> = ({ isOpen, onClose }) => {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 bg-black/60 backdrop-blur-md flex items-center justify-center p-4">
      <div className="glass-window max-w-2xl w-full rounded-2xl overflow-hidden flex flex-col max-h-[85vh] shadow-2xl border border-white/60">
        {/* Title bar */}
        <div className="bg-[#2b3a4e]/90 px-6 py-3.5 border-b border-white/10 flex items-center justify-between text-white">
          <div className="flex items-center gap-2">
            <ShieldCheck className="w-5 h-5 text-blue-400" />
            <span className="font-semibold text-sm">Security & Verification Information</span>
          </div>
          <button
            onClick={onClose}
            className="text-slate-300 hover:text-white p-1 rounded hover:bg-white/10 transition-colors cursor-pointer"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 overflow-y-auto space-y-5 text-sm text-slate-800">
          <div className="bg-blue-500/10 border border-blue-400/40 p-4 rounded-xl backdrop-blur-xs">
            <h4 className="font-bold text-blue-900 text-sm mb-1">What CheckSum Establishes</h4>
            <p className="text-xs text-blue-800 leading-relaxed">
              A matching hash proves that your file is <strong>byte-for-byte identical</strong> to the cryptographic reference published by the author.
              This confirms the download wasn't corrupted in transit, truncated, or quietly substituted by an unauthenticated mirror.
            </p>
          </div>

          <div className="bg-amber-500/10 border border-amber-400/40 p-4 rounded-xl flex items-start gap-3 backdrop-blur-xs">
            <ShieldAlert className="w-5 h-5 text-amber-600 shrink-0 mt-0.5" />
            <div className="text-xs text-amber-950 space-y-1.5 leading-relaxed">
              <h4 className="font-bold text-amber-950 text-sm">What It Does Not Establish</h4>
              <p>
                A matching hash does <strong>not</strong> prove that the software is free of vulnerabilities or malicious code.
                It proves your copy matches a <em>reference</em>, not that the reference itself is benevolent.
              </p>
              <p>
                Always obtain reference checksums directly from an authenticated, trusted HTTPS source (such as the software's official domain or signed release notes) rather than third-party forums.
              </p>
            </div>
          </div>

          <div className="glass-card rounded-xl p-4 space-y-2">
            <h4 className="font-bold text-slate-800 text-xs uppercase tracking-wider flex items-center gap-1.5">
              <Terminal className="w-4 h-4 text-slate-600" />
              CLI Verification Alternatives
            </h4>
            <div className="space-y-2 text-xs">
              <div className="bg-slate-900 text-slate-100 p-2.5 rounded-lg font-mono">
                <span className="text-slate-400"># Windows PowerShell / CMD</span>
                <br />
                certutil -hashfile CheckSum.exe SHA256
              </div>
              <div className="bg-slate-900 text-slate-100 p-2.5 rounded-lg font-mono">
                <span className="text-slate-400"># Linux / macOS</span>
                <br />
                sha256sum CheckSum.exe
              </div>
            </div>
          </div>

          <div className="border-t border-slate-200 pt-3 text-xs text-slate-500">
            For security vulnerability reports: contact{' '}
            <a href="mailto:rouhalaminerfani@gmail.com" className="text-blue-600 hover:underline font-medium">
              rouhalaminerfani@gmail.com
            </a>
          </div>
        </div>

        {/* Footer */}
        <div className="bg-[#e9ebf0]/90 px-6 py-3 border-t border-slate-300/80 flex justify-end">
          <button
            onClick={onClose}
            className="px-5 py-1.5 bg-white hover:bg-slate-100 border border-slate-300 rounded-lg text-xs font-semibold text-slate-800 transition-colors cursor-pointer shadow-2xs"
          >
            Understood
          </button>
        </div>
      </div>
    </div>
  );
};
