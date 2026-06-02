/**
 * PDF Splitter – TypeScript type definitions.
 */

/**
 * Serialised form of `pdf::PdfError` emitted by every fallible command.
 */
export interface PdfError {
  /** Machine-readable discriminant — matches the Rust enum variant name. */
  kind: 'FileNotFound' | 'InvalidPdf' | 'Io' | 'NoPages' | 'Internal';
  /** Human-readable description suitable for display in the UI. */
  message: string;
}

/**
 * Progress snapshot emitted as `split://progress` after each page is written.
 *
 * Mirrors `pdf::splitter::PageProgress` in `splitter.rs`.
 */
export interface PageProgress {
  /** 1-based count of pages completed so far. */
  current: number;
  /** Total number of pages in the source document. */
  total: number;
  /** Filename of the most-recently completed output file, e.g. `"page_0042.pdf"`. */
  fileName: string;
}

/**
 * Outcome of a successful split operation.
 *
 * Mirrors `pdf::splitter::SplitResult` in `splitter.rs`.
 */
export interface SplitResult {
  /** Total number of pages found in the source document. */
  totalPages: number;
  /** Absolute paths of every output file, sorted lexicographically. */
  outputFiles: string[];
  /** Wall-clock time taken for the whole operation, in milliseconds. */
  elapsedMs: number;
}

/**
 * Discrete states the application can be in.
 *
 * The UI renders different views based on the current state:
 *
 * | State        | Visible component        |
 * |--------------|--------------------------|
 * | `idle`       | Drop-zone / file picker  |
 * | `ready`      | File info + split button |
 * | `processing` | Progress bar             |
 * | `complete`   | Result list              |
 * | `error`      | Error card               |
 */
export type AppState = 'idle' | 'ready' | 'processing' | 'complete' | 'error';

/**
 * Metadata about the PDF file the user has selected, populated after
 * `get_page_count` returns successfully.
 */
export interface PdfFileInfo {
  /** Absolute path to the selected PDF file. */
  path: string;
  /** Display name (basename), e.g. `"report.pdf"`. */
  name: string;
  /** File size in bytes; used to render a human-readable size string. */
  sizeBytes: number;
  /** Number of pages reported by `get_page_count`. */
  pageCount: number;
}

/**
 * Represents a split operation that is currently running.
 */
export interface SplitOperation {
  /** Snapshot of the last `split://progress` event received. */
  progress: PageProgress | null;
  /** Resolved output directory path (may differ from the user's pick if a
   *  sub-directory was created automatically). */
  outputDir: string;
}

/**
 * Format `bytes` as a human-readable string, e.g. `"2.4 MB"`.
 *
 * This is a pure utility kept here so components can import it alongside the
 * types they already depend on.
 */
export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const exp = Math.min(Math.floor(Math.log2(bytes) / 10), units.length - 1);
  const value = bytes / 1024 ** exp;
  return `${value.toFixed(exp === 0 ? 0 : 1)} ${units[exp]}`;
}

/**
 * Extract the basename from an absolute file path, handling both UNIX `/`
 * and Windows `\` separators.
 */
export function basename(path: string): string {
  return path.replace(/.*[\\/]/, '');
}

/**
 * Format a duration in milliseconds as a human-readable string.
 * e.g. `1234` → `"1.2 s"`, `450` → `"450 ms"`.
 */
export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms} ms`;
  return `${(ms / 1000).toFixed(1)} s`;
}
