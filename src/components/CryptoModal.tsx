import React, { useState } from 'react';
import { Copy, Check, AlertTriangle, X } from 'lucide-react';

interface CryptoModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const CryptoModal: React.FC<CryptoModalProps> = ({ isOpen, onClose }) => {
  const [copiedKey, setCopiedKey] = useState<string | null>(null);

  if (!isOpen) return null;

  const copyToClipboard = (text: string, key: string) => {
    navigator.clipboard.writeText(text);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 2000);
  };

  const wallets = [
    {
      id: 'btc',
      title: 'Bitcoin (BTC)',
      network: 'Bitcoin',
      address: 'bc1qgsxsy5wp6twxhuxhvnujd8pza788ra5j4j67gu',
      qrImage: '/assets/btc-qr.png',
      badgeColor: 'bg-amber-100 text-amber-900 border-amber-300',
    },
    {
      id: 'eth',
      title: 'Ethereum (ETH)',
      network: 'Base · Optimism · ERC-20',
      address: '0x6E6764da4c477a08318224B992Bb54Cedd4197a0',
      qrImage: '/assets/eth-qr.png',
      badgeColor: 'bg-indigo-100 text-indigo-900 border-indigo-300',
    },
    {
      id: 'usdt',
      title: 'Tether (USDT)',
      network: 'TRC-20 (TRON)',
      address: 'TDcg8aKadKRZx97Fw5p7BfCGXBBgWFy6Vh',
      qrImage: '/assets/usdt-qr.png',
      badgeColor: 'bg-emerald-100 text-emerald-900 border-emerald-300',
    },
  ];

  return (
    <div className="fixed inset-0 z-50 bg-black/60 backdrop-blur-md flex items-center justify-center p-4">
      <div className="glass-window max-w-3xl w-full rounded-2xl overflow-hidden flex flex-col max-h-[90vh] shadow-2xl border border-white/60">
        {/* Header */}
        <div className="bg-[#2b3a4e]/90 px-6 py-3.5 border-b border-white/10 flex items-center justify-between text-white">
          <div className="flex items-center gap-2">
            <span className="font-semibold text-sm">Support the Project via Cryptocurrency</span>
          </div>
          <button
            onClick={onClose}
            className="text-slate-300 hover:text-white p-1 rounded hover:bg-white/10 transition-colors cursor-pointer"
            title="Close"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 overflow-y-auto space-y-5 text-sm text-slate-800">
          <div className="text-slate-700 leading-relaxed">
            <p className="font-bold text-slate-900 text-base">Hi, I'm Rohulamin Erfani.</p>
            <p className="mt-1 text-xs text-slate-600">
              I built CheckSum to provide a fast, focused, and trustworthy file integrity verifier — keeping it fully open source.
              Your support goes directly toward ongoing testing, infrastructure, documentation, and security improvements.
            </p>
          </div>

          {/* Network Warning Banner */}
          <div className="bg-amber-500/10 border-l-4 border-amber-500 p-3.5 rounded-r-xl flex items-start gap-3 backdrop-blur-xs">
            <AlertTriangle className="w-5 h-5 text-amber-600 shrink-0 mt-0.5" />
            <div className="text-xs text-amber-900 leading-relaxed">
              <span className="font-bold">Network Caution:</span> Always double-check the <strong>network</strong> before sending funds.
              Ethereum and USDT use <strong>different blockchains</strong> — sending funds on the wrong network may result in permanent loss.
            </div>
          </div>

          {/* Wallets Grid */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            {wallets.map((wallet) => (
              <div
                key={wallet.id}
                className="glass-card rounded-xl p-4 flex flex-col items-center text-center shadow-xs"
              >
                <div className="font-bold text-slate-900 text-sm">{wallet.title}</div>
                <div className="text-[11px] text-slate-500 mt-0.5 mb-3">
                  Network: <span className="font-semibold text-slate-700">{wallet.network}</span>
                </div>

                {/* QR Code */}
                <div className="w-36 h-36 bg-white rounded-lg border border-slate-200/80 p-1.5 flex items-center justify-center mb-3 shadow-inner">
                  <img
                    src={wallet.qrImage}
                    alt={`${wallet.title} QR`}
                    className="w-full h-full object-contain"
                  />
                </div>

                {/* Address Box */}
                <div className="w-full mt-auto">
                  <div className="bg-white/80 border border-slate-200 rounded-lg p-2 text-[11px] font-mono break-all select-all text-slate-700 text-left mb-2 leading-tight">
                    {wallet.address}
                  </div>
                  <button
                    onClick={() => copyToClipboard(wallet.address, wallet.id)}
                    className="w-full py-1.5 px-3 bg-white hover:bg-slate-50 border border-slate-300 rounded-lg text-xs font-semibold text-slate-700 flex items-center justify-center gap-1.5 transition-all shadow-2xs hover:shadow-xs active:scale-98 cursor-pointer"
                  >
                    {copiedKey === wallet.id ? (
                      <>
                        <Check className="w-3.5 h-3.5 text-emerald-600" />
                        <span className="text-emerald-700">Copied!</span>
                      </>
                    ) : (
                      <>
                        <Copy className="w-3.5 h-3.5 text-slate-600" />
                        <span>Copy Address</span>
                      </>
                    )}
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Footer */}
        <div className="bg-[#e9ebf0]/90 px-6 py-3 border-t border-slate-300/80 flex justify-end">
          <button
            onClick={onClose}
            className="px-5 py-1.5 bg-white hover:bg-slate-100 border border-slate-300 rounded-lg text-xs font-semibold text-slate-800 transition-colors cursor-pointer shadow-2xs"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
};
