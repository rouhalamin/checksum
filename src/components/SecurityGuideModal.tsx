import React from 'react';
import { ShieldAlert, ShieldCheck, X, Terminal } from 'lucide-react';

interface SecurityGuideModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const SecurityGuideModal: React.FC<SecurityGuideModalProps> = ({ isOpen, onClose }) => {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 bg-black/60 backdrop-blur-xs flex items-center justify-center p-4">
      <div className="bg-[#f6f6f8] text-slate-800 rounded-lg shadow-2xl border border-slate-300 max-w-2xl w-full overflow-hidden flex flex-col max-h-[85vh]">
        {/* Title bar */}
        <div className="bg-[#e4e4e8] px-5 py-3 border-b border-slate-300 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <ShieldCheck className="w-5 h-5 text-blue-700" />
            <span className="font-semibold text-slate-800 text-sm">Security & Verification Information</span>
          </div>
          <button
            onClick={onClose}
            className="text-slate-500 hover:text-slate-800 p-0.5 rounded-sm hover:bg-slate-300 transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 overflow-y-auto space-y-5 text-sm">
          <div className="bg-blue-50 border border-blue-200 p-3.5 rounded-sm">
            <h4 className="font-semibold text-blue-900 text-sm mb-1">What CheckSum Establishes</h4>
            <p className="text-xs text-blue-800 leading-relaxed">
              A matching hash proves that your file is <strong>byte-for-byte identical</strong> to the cryptographic reference published by the author.
              This confirms the download wasn't corrupted in transit, truncated, or quietly substituted by an unauthenticated mirror.
            </p>
          </div>

          <div className="bg-amber-50 border border-amber-200 p-3.5 rounded-sm flex items-start gap-3">
            <ShieldAlert className="w-5 h-5 text-amber-700 shrink-0 mt-0.5" />
            <div className="text-xs text-amber-900 space-y-1.5 leading-relaxed">
              <h4 className="font-semibold text-amber-950 text-sm">What It Does Not Establish</h4>
              <p>
                A matching hash does <strong>not</strong> prove that the software is free of vulnerabilities or malicious code.
                It proves your copy matches a <em>reference</em>, not that the reference itself is benevolent.
              </p>
              <p>
                Always obtain reference checksums directly from an authenticated, trusted HTTPS source (such as the software's official domain or signed release notes) rather than third-party forums.
              </p>
            </div>
          </div>

          <div>
            <h4 className="font-semibold text-slate-800 text-xs uppercase tracking-wider mb-2 flex items-center gap-1.5">
              <Terminal className="w-4 h-4 text-slate-600" />
              CLI Verification Alternatives
            </h4>
            <div className="space-y-2 text-xs">
              <div className="bg-slate-900 text-slate-100 p-2.5 rounded font-mono">
                <span className="text-slate-400"># Windows PowerShell / CMD</span>
                <br />
                certutil -hashfile CheckSum.exe SHA256
              </div>
              <div className="bg-slate-900 text-slate-100 p-2.5 rounded font-mono">
                <span className="text-slate-400"># Linux / macOS</span>
                <br />
                sha256sum CheckSum.exe
              </div>
            </div>
          </div>

          <div className="border-t border-slate-200 pt-3 text-xs text-slate-500">
            For security vulnerability reports: contact{' '}
            <a href="mailto:rouhalaminerfani@gmail.com" className="text-blue-600 hover:underline">
              rouhalaminerfani@gmail.com
            </a>
          </div>
        </div>

        {/* Footer */}
        <div className="bg-[#ececf0] px-5 py-3 border-t border-slate-300 flex justify-end">
          <button
            onClick={onClose}
            className="px-5 py-1.5 bg-slate-200 hover:bg-slate-300 border border-slate-400 rounded text-xs font-semibold text-slate-800 transition-colors cursor-pointer"
          >
            Understood
          </button>
        </div>
      </div>
    </div>
  );
};
