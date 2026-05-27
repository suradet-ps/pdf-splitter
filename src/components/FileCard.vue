<script setup lang="ts">
defineProps<{
    fileName: string
    pageCount: number
    fileSizeFormatted: string
    outputDirShort: string
    busy: boolean
}>()

const emit = defineEmits<{
    split: []
    'change-file': []
    'change-output': []
}>()

function onSplit(): void {
    emit('split')
}

function onChangeFile(): void {
    emit('change-file')
}

function onChangeOutput(): void {
    emit('change-output')
}
</script>

<template>
<div class="file-card">

    <!-- ── Section 1: File info ── -->
    <div class="file-info">

        <!-- PDF icon -->
        <div class="file-icon" aria-hidden="true">
            <svg viewBox="0 0 36 44" fill="none" xmlns="http://www.w3.org/2000/svg" width="36" height="44">
                <rect x="0" y="0" width="28" height="40" rx="3" fill="currentColor" class="doc-body" />
                <path d="M20 0 L28 8 L20 8 Z" fill="currentColor" class="doc-fold" />
                <line x1="20" y1="0" x2="28" y2="8" stroke="currentColor" stroke-width="0.5" class="doc-fold-line" />
                <rect x="0" y="28" width="28" height="12" rx="2" fill="currentColor" class="doc-band" />
                <rect x="3" y="32" width="5" height="1.6" rx="0.8" fill="currentColor" class="doc-label" />
                <rect x="10" y="32" width="4" height="1.6" rx="0.8" fill="currentColor" class="doc-label" />
                <rect x="16" y="32" width="4" height="1.6" rx="0.8" fill="currentColor" class="doc-label" />
                <rect x="4" y="9" width="16" height="1.4" rx="0.7" fill="currentColor" class="doc-line" />
                <rect x="4" y="13" width="13" height="1.4" rx="0.7" fill="currentColor" class="doc-line" />
                <rect x="4" y="17" width="15" height="1.4" rx="0.7" fill="currentColor" class="doc-line" />
                <rect x="4" y="21" width="10" height="1.4" rx="0.7" fill="currentColor" class="doc-line" />
            </svg>
        </div>

        <!-- File name + metadata -->
        <div class="file-meta">
            <span class="file-name truncate" :title="fileName">{{ fileName }}</span>
            <div class="file-details">
                <span class="file-detail">
                    <span class="file-detail__val">{{ pageCount }}</span>
                    <span class="file-detail__key">{{ pageCount === 1 ? 'page' : 'pages' }}</span>
                </span>
                <template v-if="fileSizeFormatted">
                    <span class="file-detail__sep" aria-hidden="true">·</span>
                    <span class="file-detail">
                        <span class="file-detail__val">{{ fileSizeFormatted }}</span>
                    </span>
                </template>
            </div>
        </div>

        <!-- Dismiss / change-file button -->
        <button
            type="button"
            class="btn-icon dismiss-btn"
            :disabled="busy"
            :aria-label="`Remove ${fileName}`"
            title="Choose a different file"
            @click="onChangeFile"
        >
            <svg viewBox="0 0 20 20" fill="none" xmlns="http://www.w3.org/2000/svg" class="icon-md" aria-hidden="true">
                <path
                    fill-rule="evenodd"
                    clip-rule="evenodd"
                    d="M4.293 4.293a1 1 0 0 1 1.414 0L10 8.586l4.293-4.293a1 1 0 1 1 1.414 1.414L11.414 10l4.293 4.293a1 1 0 0 1-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 0 1-1.414-1.414L8.586 10 4.293 5.707a1 1 0 0 1 0-1.414Z"
                    fill="currentColor"
                />
            </svg>
        </button>
    </div>

    <div class="separator" role="separator" />

    <!-- ── Section 2: Output directory ── -->
    <div class="output-row">
        <div class="output-row__info">
            <span class="output-row__label">Output folder</span>
            <span class="output-row__path truncate" :title="outputDirShort">
                {{ outputDirShort || 'Same folder as PDF' }}
            </span>
        </div>
        <button
            type="button"
            class="output-change-btn"
            :disabled="busy"
            @click="onChangeOutput"
        >
            Change
        </button>
    </div>

    <div class="separator" role="separator" />

    <!-- ── Section 3: Actions ── -->
    <div class="action-row">

        <!-- Page count badge -->
        <span class="page-badge" aria-label="`${pageCount} pages`">
            <svg viewBox="0 0 12 14" fill="none" xmlns="http://www.w3.org/2000/svg" width="10" height="12" aria-hidden="true">
                <path
                    d="M1.5 1A.5.5 0 0 1 2 .5h6.086a.5.5 0 0 1 .353.146l2.414 2.415A.5.5 0 0 1 11 3.414V13a.5.5 0 0 1-.5.5h-8A.5.5 0 0 1 1.5 13V1Z"
                    fill="currentColor"
                    opacity="0.6"
                />
            </svg>
            {{ pageCount }}&nbsp;{{ pageCount === 1 ? 'page' : 'pages' }}
        </span>

        <!-- Split button — primary CTA -->
        <button
            type="button"
            class="btn-primary btn-glow split-btn"
            :disabled="busy"
            @click="onSplit"
        >
            <template v-if="busy">
                <span class="animate-pulse">Splitting…</span>
            </template>
            <template v-else>
                Split PDF
                <svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" width="14" height="14" aria-hidden="true" class="split-btn__arrow">
                    <path
                        fill-rule="evenodd"
                        clip-rule="evenodd"
                        d="M3.75 8a.75.75 0 0 1 .75-.75h5.19L7.22 4.78a.75.75 0 0 1 1.06-1.06l3.5 3.5a.75.75 0 0 1 0 1.06l-3.5 3.5a.75.75 0 0 1-1.06-1.06l2.47-2.47H4.5A.75.75 0 0 1 3.75 8Z"
                        fill="currentColor"
                    />
                </svg>
            </template>
        </button>

    </div>

</div>
</template>

<style scoped>
/* ─────────────────────────────────────────────
   Card shell
   ───────────────────────────────────────────── */

.file-card {
    display: flex;
    flex-direction: column;
    width: 100%;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-md);
    overflow: hidden;
}

/* ─────────────────────────────────────────────
   Section 1 — File info
   ───────────────────────────────────────────── */

.file-info {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: 18px 20px;
}

/* PDF document icon */
.file-icon {
    flex-shrink: 0;
    color: var(--color-accent);
    filter: drop-shadow(0 0 8px rgba(54, 244, 164, 0.18));
}

.doc-body      { opacity: 0.10; }
.doc-fold      { opacity: 0.06; }
.doc-fold-line { opacity: 0.10; }
.doc-band      { opacity: 0.70; }
.doc-label     { opacity: 0.90; fill: var(--color-text-on-accent); }
.doc-line      { opacity: 0.12; }

/* File name + metadata */
.file-meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
}

.file-name {
    font-family: var(--font-display);
    font-size: var(--text-xl);
    font-weight: var(--weight-medium);
    color: var(--color-text-primary);
    line-height: var(--leading-snug);
    letter-spacing: var(--tracking-tight);
    display: block;
    max-width: 380px;
}

.file-details {
    display: flex;
    align-items: center;
    gap: var(--space-2);
}

.file-detail {
    display: inline-flex;
    align-items: baseline;
    gap: 4px;
    font-size: var(--text-sm);
}

.file-detail__val {
    font-family: var(--font-display);
    font-weight: var(--weight-medium);
    color: var(--color-text-secondary);
}

.file-detail__key {
    font-family: var(--font-sans);
    color: var(--color-text-quaternary);
}

.file-detail__sep {
    font-size: var(--text-xs);
    color: var(--color-text-quaternary);
}

/* Dismiss button */
.dismiss-btn {
    flex-shrink: 0;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    color: var(--color-text-quaternary);
    transition:
        background-color var(--duration-fast) var(--ease-out),
        color var(--duration-fast) var(--ease-out);
}

.dismiss-btn:hover:not(:disabled) {
    background: var(--color-error-subtle);
    color: var(--color-error-text);
}

/* ─────────────────────────────────────────────
   Section 2 — Output directory
   ───────────────────────────────────────────── */

.output-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: 12px 20px;
    background: var(--color-surface-raised);
}

.output-row__info {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
    flex: 1;
}

.output-row__label {
    font-family: var(--font-sans);
    font-size: var(--text-xs);
    font-weight: var(--weight-semibold);
    letter-spacing: var(--tracking-wider);
    text-transform: uppercase;
    color: var(--color-text-quaternary);
    line-height: 1;
}

.output-row__path {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    color: var(--color-text-secondary);
    display: block;
    max-width: 360px;
    line-height: var(--leading-normal);
}

/* Compact ghost "Change" button */
.output-change-btn {
    flex-shrink: 0;
    background: transparent;
    color: var(--color-text-tertiary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-full);
    padding: 4px 14px;
    font-family: var(--font-sans);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
    letter-spacing: 0.01em;
    height: 26px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    transition:
        color var(--duration-fast) var(--ease-out),
        border-color var(--duration-fast) var(--ease-out),
        background var(--duration-fast) var(--ease-out);
    outline: none;
    white-space: nowrap;
}

.output-change-btn:hover:not(:disabled) {
    color: var(--color-text-primary);
    border-color: var(--color-border-strong);
    background: var(--color-surface-hover);
}

.output-change-btn:focus-visible {
    box-shadow: var(--shadow-focus);
}

.output-change-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
}

/* ─────────────────────────────────────────────
   Section 3 — Actions
   ───────────────────────────────────────────── */

.action-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 20px;
    gap: var(--space-4);
}

/* Page count badge */
.page-badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-family: var(--font-sans);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
    color: var(--color-text-secondary);
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 4px 10px;
    letter-spacing: 0.02em;
    white-space: nowrap;
}

/* Split button — white pill, black text, neon glow hint */
.split-btn {
    min-width: 140px;
    height: 42px;
    font-size: var(--text-lg);
    padding: 10px 24px 10px 20px;
    gap: var(--space-2);
}

.split-btn__arrow {
    opacity: 0.7;
    transition: transform var(--duration-fast) var(--ease-out);
}

.split-btn:hover:not(:disabled) .split-btn__arrow {
    transform: translateX(2px);
    opacity: 1;
}
</style>