import { AlgorithmType, SidecarChecksum } from '../types';

export const CHECKSUM_FILE_EXTS = [
  'sha256',
  'sha384',
  'sha512',
  'sha1',
  'md5',
  'b2',
  'blake2',
  'blake2b',
  'blake3',
  'checksum',
  'sum',
  'sfv',
];

export function isChecksumSidecar(filename: string): boolean {
  const ext = filename.split('.').pop()?.toLowerCase();
  return ext ? CHECKSUM_FILE_EXTS.includes(ext) : false;
}

export function guessAlgorithmFromExt(filename: string): AlgorithmType | undefined {
  const ext = filename.split('.').pop()?.toLowerCase();
  switch (ext) {
    case 'sha256':
      return 'SHA-256';
    case 'sha384':
      return 'SHA-384';
    case 'sha512':
      return 'SHA-512';
    case 'sha1':
      return 'SHA-1';
    case 'md5':
      return 'MD5';
    case 'b2':
    case 'blake2':
    case 'blake2b':
      return 'BLAKE2b-512';
    case 'blake3':
      return 'BLAKE3';
    default:
      return undefined;
  }
}

export function parseChecksumFile(content: string, filename?: string): SidecarChecksum {
  const lines = content.split(/\r?\n/);
  const isHex = (s: string) => /^[0-9a-fA-F]+$/.test(s);

  for (const rawLine of lines) {
    const line = rawLine.trim();
    if (!line || line.startsWith('#')) {
      continue;
    }

    // BSD / OpenSSL style: ALGO(filename) = hash
    const eqPos = line.indexOf('=');
    const openParen = line.indexOf('(');
    const closeParen = line.indexOf(')');
    if (eqPos !== -1 && openParen !== -1 && closeParen !== -1 && openParen < closeParen && closeParen < eqPos) {
      const referenced = line.substring(openParen + 1, closeParen).trim();
      const hash = line.substring(eqPos + 1).trim().toLowerCase();
      if (hash && isHex(hash)) {
        return {
          hash,
          referencedFilename: referenced || undefined,
          algorithmGuess: filename ? guessAlgorithmFromExt(filename) : undefined,
        };
      }
    }

    // GNU coreutils style: <hash>  [*]<filename> OR <hash> <filename>
    const parts = line.split(/\s+/);
    if (parts.length >= 1) {
      const candidate = parts[0].trim().toLowerCase();
      if (candidate && isHex(candidate) && candidate.length >= 32) {
        let referenced: string | undefined = undefined;
        if (parts.length > 1) {
          const rest = line.substring(parts[0].length).trim();
          referenced = rest.replace(/^\*/, '').trim();
        }
        return {
          hash: candidate,
          referencedFilename: referenced || undefined,
          algorithmGuess: filename ? guessAlgorithmFromExt(filename) : undefined,
        };
      }
    }
  }

  throw new Error('No valid checksum entry found in this file.');
}
