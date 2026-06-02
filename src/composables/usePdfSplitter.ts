import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { computed, onUnmounted, ref } from 'vue';

import {
  type AppState,
  basename,
  formatBytes,
  formatDuration,
  type PageProgress,
  type PdfError,
  type PdfFileInfo,
  type SplitOperation,
  type SplitResult,
} from '@/types';

/**
 * Response from the `get_file_info` Tauri command.
 * Mirrors `commands::FileInfo` in `src-tauri/src/commands.rs`.
 */
interface FileInfoResponse {
  pageCount: number;
  sizeBytes: number;
}

/** Tauri event name for per-page progress updates. */
const EVENT_PROGRESS = 'split://progress' as const;

export function usePdfSplitter() {
  /** Current step in the application flow. */
  const state = ref<AppState>('idle');

  /** Metadata for the user-selected PDF, populated in the `ready` state. */
  const fileInfo = ref<PdfFileInfo | null>(null);

  /**
   * The directory where split pages will be saved.
   *
   * Defaults to a sub-directory named after the PDF stem inside the same
   * folder as the source file.  The user can override this via `pickOutputDir`.
   */
  const outputDir = ref<string>('');

  /** Current operation snapshot — only meaningful during `processing`. */
  const operation = ref<SplitOperation | null>(null);

  /** Result of the last successful split — only meaningful during `complete`. */
  const result = ref<SplitResult | null>(null);

  /** Last error encountered — only meaningful during `error`. */
  const error = ref<string | null>(null);

  /** Whether an async operation is pending (disables interactive controls). */
  const isBusy = ref(false);

  /** Cleanup function returned by `listen()`; called on component unmount. */
  let unlistenProgress: UnlistenFn | null = null;

  /**
   * Pending progress payload waiting to be committed on the next animation
   * frame.  When events arrive faster than the display refresh rate (~60 Hz),
   * only the most recent payload is kept; earlier ones are silently dropped.
   * This prevents Vue from re-rendering hundreds of times per second during
   * large splits.
   */
  let pendingProgress: PageProgress | null = null;

  /** `requestAnimationFrame` handle so we can cancel on cleanup. */
  let rafHandle: number | null = null;

  /** Formatted file size string, e.g. `"2.4 MB"`. */
  const fileSizeFormatted = computed<string>(() =>
    fileInfo.value ? formatBytes(fileInfo.value.sizeBytes) : '',
  );

  /**
   * Progress as a percentage (0–100), rounded to the nearest integer.
   * Returns `0` when no operation is in progress.
   */
  const progressPercent = computed<number>(() => {
    const p = operation.value?.progress;
    if (!p || p.total === 0) return 0;
    return Math.round((p.current / p.total) * 100);
  });

  /** Human-readable progress label, e.g. `"Processing page 3 of 10…"`. */
  const progressLabel = computed<string>(() => {
    const p = operation.value?.progress;
    if (!p) return 'Starting…';
    return `Processing page ${p.current} of ${p.total}…`;
  });

  /** Human-readable elapsed time string for the result view. */
  const elapsedFormatted = computed<string>(() =>
    result.value ? formatDuration(result.value.elapsedMs) : '',
  );

  /**
   * Display-friendly basename of the output directory.
   *
   * On macOS the full path can be long; showing only the last two path
   * components keeps the UI clean without losing useful context.
   */
  const outputDirShort = computed<string>(() => {
    if (!outputDir.value) return '';
    const parts = outputDir.value.replace(/\\/g, '/').split('/');
    return parts.slice(-2).join('/');
  });

  /**
   * Derive the default output directory from a given input file path.
   *
   * Rule: `<parent-of-input>/<stem-of-input>/`
   * Example: `/Users/alice/Docs/report.pdf` → `/Users/alice/Docs/report`
   */
  function defaultOutputDir(inputPath: string): string {
    const normalised = inputPath.replace(/\\/g, '/');
    const lastSlash = normalised.lastIndexOf('/');
    const dir = lastSlash >= 0 ? normalised.slice(0, lastSlash) : '.';
    const file = lastSlash >= 0 ? normalised.slice(lastSlash + 1) : normalised;
    const stem = file.endsWith('.pdf') ? file.slice(0, file.length - 4) : file;
    return `${dir}/${stem}`;
  }

  /**
   * Safely dispose the progress event listener and cancel any pending animation
   * frame.
   */
  async function disposeProgressListener(): Promise<void> {
    if (unlistenProgress) {
      unlistenProgress();
      unlistenProgress = null;
    }
    if (rafHandle !== null) {
      cancelAnimationFrame(rafHandle);
      rafHandle = null;
    }
    pendingProgress = null;
  }

  /**
   * Register the `split://progress` event listener with built-in
   * **rAF throttling**.
   *
   * ## Why throttle?
   *
   * Even though processing is sequential, events for small pages can arrive
   * faster than the ~60 Hz display refresh rate. Committing every event to
   * Vue state could cause synchronous re-renders, visibly stuttering the UI.
   *
   * Instead, incoming events are buffered into `pendingProgress` (a plain
   * variable, invisible to Vue).  A single `requestAnimationFrame` callback
   * then flushes the latest buffered value into the reactive `operation`
   * ref once per frame.
   */
  async function attachProgressListener(): Promise<void> {
    await disposeProgressListener();

    unlistenProgress = await listen<PageProgress>(EVENT_PROGRESS, (event) => {
      const p = event.payload;

      pendingProgress = p;

      if (rafHandle === null) {
        rafHandle = requestAnimationFrame(() => {
          rafHandle = null;
          if (pendingProgress && operation.value) {
            operation.value = {
              ...operation.value,
              progress: { ...pendingProgress },
            };
            pendingProgress = null;
          }
        });
      }
    });
  }

  /**
   * Translate a Tauri command rejection into a human-readable string.
   *
   * Tauri serialises `PdfError` as `{ kind, message }`.  For unexpected
   * shapes we fall back to `JSON.stringify` so nothing is silently swallowed.
   */
  function describeError(raw: unknown): string {
    if (typeof raw === 'string') return raw;
    if (raw && typeof raw === 'object') {
      const e = raw as Partial<PdfError>;
      if (e.message) return e.message;
    }
    return JSON.stringify(raw);
  }

  /**
   * Open the native file-picker dialog and load metadata for the chosen PDF.
   *
   * Transitions: `idle` / `ready` / `complete` / `error` → `ready`
   *              (cancelled dialog leaves state unchanged)
   */
  async function pickFile(): Promise<void> {
    if (isBusy.value) return;
    isBusy.value = true;

    try {
      const path = await invoke<string | null>('pick_pdf_file');
      if (!path) return; // User cancelled — no state change.

      // `get_file_info` returns both page count and file size in one round-trip,
      // eliminating the need for `@tauri-apps/plugin-fs` in the renderer.
      const info = await invoke<FileInfoResponse>('get_file_info', { path });

      fileInfo.value = {
        path,
        name: basename(path),
        sizeBytes: info.sizeBytes,
        pageCount: info.pageCount,
      };

      outputDir.value = defaultOutputDir(path);
      result.value = null;
      error.value = null;
      operation.value = null;
      state.value = 'ready';
    } catch (raw) {
      error.value = describeError(raw);
      state.value = 'error';
    } finally {
      isBusy.value = false;
    }
  }

  /**
   * Open the native directory-picker dialog and update `outputDir`.
   *
   * Only callable in the `ready` state.  If the user cancels, `outputDir`
   * keeps its current value.
   */
  async function pickOutputDir(): Promise<void> {
    if (isBusy.value || state.value !== 'ready') return;
    isBusy.value = true;

    try {
      const dir = await invoke<string | null>('pick_output_dir');
      if (dir) {
        outputDir.value = dir;
      }
    } catch (raw) {
      // Non-fatal: the user can still split using the default output dir.
      console.warn('[usePdfSplitter] pickOutputDir failed:', describeError(raw));
    } finally {
      isBusy.value = false;
    }
  }

  /**
   * Invoke the `split_pdf` Tauri command and stream live progress updates.
   *
   * Transitions: `ready` → `processing` → `complete` / `error`
   */
  async function startSplit(): Promise<void> {
    if (isBusy.value || state.value !== 'ready' || !fileInfo.value) return;

    isBusy.value = true;
    error.value = null;

    try {
      // Set up progress listener *before* invoking the command so we never
      // miss the first event.
      await attachProgressListener();

      operation.value = {
        progress: null,
        outputDir: outputDir.value,
      };
      state.value = 'processing';

      const splitResult = await invoke<SplitResult>('split_pdf', {
        inputPath: fileInfo.value.path,
        outputDir: outputDir.value,
      });

      result.value = splitResult;
      state.value = 'complete';
    } catch (raw) {
      error.value = describeError(raw);
      state.value = 'error';
    } finally {
      await disposeProgressListener();
      operation.value = null;
      isBusy.value = false;
    }
  }

  /**
   * Reveal the output directory (or a specific file path) in macOS Finder.
   *
   * Silently ignores failures (e.g. if the folder was deleted after splitting).
   */
  async function revealOutput(path?: string): Promise<void> {
    const target = path ?? result.value?.outputFiles[0] ?? outputDir.value;
    if (!target) return;

    try {
      await invoke('reveal_in_finder', { path: target });
    } catch (raw) {
      console.warn('[usePdfSplitter] revealOutput failed:', describeError(raw));
    }
  }

  /**
   * Reset the application to the initial `idle` state so the user can split
   * another file.
   */
  async function reset(): Promise<void> {
    await disposeProgressListener();
    state.value = 'idle';
    fileInfo.value = null;
    outputDir.value = '';
    operation.value = null;
    result.value = null;
    error.value = null;
    isBusy.value = false;
  }

  /** Clean up the event listener when the owning component is destroyed. */
  onUnmounted(() => {
    disposeProgressListener().catch(console.error);
  });

  return {
    /** Current application state. */
    state,
    /** Metadata for the selected PDF file. */
    fileInfo,
    /** Resolved output directory path. */
    outputDir,
    /** Live snapshot of the running split operation. */
    operation,
    /** Result of the last successful split. */
    result,
    /** Human-readable error message from the last failure. */
    error,
    /** True while any async operation is in flight. */
    isBusy,

    /** Formatted file size, e.g. `"2.4 MB"`. */
    fileSizeFormatted,
    /** Progress as 0–100 integer. */
    progressPercent,
    /** Human-readable progress label. */
    progressLabel,
    /** Formatted elapsed time, e.g. `"1.2 s"`. */
    elapsedFormatted,
    /** Short display form of the output directory path. */
    outputDirShort,

    /** Open the PDF file picker dialog. */
    pickFile,
    /** Open the output directory picker dialog. */
    pickOutputDir,
    /** Begin the split operation. */
    startSplit,
    /** Reveal a path in Finder. */
    revealOutput,
    /** Reset to initial state. */
    reset,
  } as const;
}

/** Convenience type alias for the return value of `usePdfSplitter`. */
export type UsePdfSplitter = ReturnType<typeof usePdfSplitter>;
