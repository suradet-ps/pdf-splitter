<script setup lang="ts">
import { ref, computed } from 'vue'
import { basename } from '@/types'

const props = defineProps<{
    totalPages: number
    outputFiles: string[]
    elapsedFormatted: string
    outputDir: string
}>()

const emit = defineEmits<{
    reveal: [path: string]
    reset: []
}>()

const hoveredIndex = ref<number | null>(null)

const fileNames = computed<string[]>(() =>
    props.outputFiles.map((p) => basename(p)),
)

const summaryLabel = computed<string>(() => {
    const pages = props.totalPages
    const unit = pages === 1 ? 'file' : 'files'
    return props.elapsedFormatted
        ? `${pages} ${unit} · ${props.elapsedFormatted}`
        : `${pages} ${unit}`
})

function onRevealDir(): void {
    emit('reveal', props.outputDir)
}

function onRevealFile(path: string): void {
    emit('reveal', path)
}

function onReset(): void {
    emit('reset')
}
</script>

<template>
<div class="result-view">

    <!-- ── Header ── -->
    <div class="result-header">

        <!-- Animated ✓ check badge -->
        <div class="success-badge animate-bounce-in" aria-hidden="true">
            <svg viewBox="0 0 32 32" fill="none" xmlns="http://www.w3.org/2000/svg" width="32" height="32">
                <circle cx="16" cy="16" r="15" fill="currentColor" class="badge-circle" />
                <path fill-rule="evenodd" clip-rule="evenodd"
                    d="M22.78 10.72a.75.75 0 0 1 0 1.06l-8.5 8.5a.75.75 0 0 1-1.06 0l-3.5-3.5a.75.75 0 1 1 1.06-1.06l2.97 2.97 7.97-7.97a.75.75 0 0 1 1.06 0Z"
                    fill="currentColor" class="badge-check" />
            </svg>
        </div>

        <!-- Title & summary -->
        <div class="result-header__text">
            <h2 class="result-header__title">Done</h2>
            <p class="result-header__summary">{{ summaryLabel }}</p>
        </div>

        <!-- Stats chips -->
        <div class="result-stats" aria-label="Split statistics">
            <span class="stat-chip stat-chip--files">
                <!-- Document icon -->
                <svg viewBox="0 0 12 14" fill="none" xmlns="http://www.w3.org/2000/svg" width="10" height="12" aria-hidden="true">
                    <path d="M1.5 1A.5.5 0 0 1 2 .5h6.086a.5.5 0 0 1 .353.146l2.414 2.415A.5.5 0 0 1 11 3.414V13a.5.5 0 0 1-.5.5h-8A.5.5 0 0 1 1.5 13V1Z"
                        fill="currentColor" opacity="0.7"/>
                </svg>
                {{ totalPages }}&nbsp;{{ totalPages === 1 ? 'file' : 'files' }}
            </span>
            <span v-if="elapsedFormatted" class="stat-chip stat-chip--time">
                <!-- Clock icon -->
                <svg viewBox="0 0 12 12" fill="none" xmlns="http://www.w3.org/2000/svg" width="10" height="10" aria-hidden="true">
                    <circle cx="6" cy="6" r="5" stroke="currentColor" stroke-width="1.3" fill="none" />
                    <path d="M6 3.5v2.5l1.5 1" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
                </svg>
                {{ elapsedFormatted }}
            </span>
        </div>

    </div>

    <!-- ── File list ── -->
    <div class="file-list-container">

        <!-- Directory header -->
        <div class="file-list__header">
            <!-- Folder icon -->
            <svg viewBox="0 0 14 12" fill="none" xmlns="http://www.w3.org/2000/svg" width="12" height="10" aria-hidden="true">
                <path fill-rule="evenodd" clip-rule="evenodd"
                    d="M1 1.5A.5.5 0 0 1 1.5 1h3.379a.5.5 0 0 1 .353.146l.768.768A.5.5 0 0 0 6.353 2.1H12.5a.5.5 0 0 1 .5.5v8a.5.5 0 0 1-.5.5h-11A.5.5 0 0 1 1 10.6V1.5Z"
                    fill="currentColor" opacity="0.6"/>
            </svg>
            <span class="file-list__header-label">Output folder</span>
            <span class="file-list__header-path truncate">{{ outputDir }}</span>
        </div>

        <!-- Scrollable file rows -->
        <div class="file-list" role="list" :aria-label="`${totalPages} output files`">
            <TransitionGroup name="list-item" tag="div" class="file-list__inner">
                <div
                    v-for="(path, index) in outputFiles"
                    :key="path"
                    class="file-row"
                    role="listitem"
                    :style="{ transitionDelay: `${Math.min(index * 14, 280)}ms` }"
                    @mouseenter="hoveredIndex = index"
                    @mouseleave="hoveredIndex = null"
                >
                    <!-- Line number -->
                    <span class="file-row__lineno" aria-hidden="true">{{ String(index + 1).padStart(3, '\u00a0') }}</span>

                    <!-- Filename -->
                    <span class="file-row__name">{{ fileNames[index] }}</span>

                    <!-- Reveal button (appears on hover) -->
                    <Transition name="fade">
                        <button
                            v-if="hoveredIndex === index"
                            type="button"
                            class="file-row__reveal"
                            :aria-label="`Reveal ${fileNames[index]} in Finder`"
                            @click.stop="onRevealFile(path)"
                        >
                            <svg viewBox="0 0 14 14" fill="none" xmlns="http://www.w3.org/2000/svg" width="11" height="11" aria-hidden="true">
                                <path fill-rule="evenodd" clip-rule="evenodd"
                                    d="M2 2.5A.5.5 0 0 1 2.5 2h9a.5.5 0 0 1 .5.5v9a.5.5 0 0 1-.5.5H2.5a.5.5 0 0 1-.5-.5v-9ZM3 3v8h8V3H3ZM7 4.5a.5.5 0 0 1 .5.5v2h2a.5.5 0 0 1 0 1h-2v2a.5.5 0 0 1-1 0v-2h-2a.5.5 0 0 1 0-1h2V5a.5.5 0 0 1 .5-.5Z"
                                    fill="currentColor"/>
                            </svg>
                            reveal
                        </button>
                    </Transition>
                </div>
            </TransitionGroup>
        </div>

    </div>

    <!-- ── Actions ── -->
    <div class="result-actions">

        <!-- "Split another" — ghost/secondary button -->
        <button type="button" class="btn-ghost result-actions__secondary" @click="onReset">
            <svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" width="14" height="14" aria-hidden="true">
                <path fill-rule="evenodd" clip-rule="evenodd"
                    d="M3.5 8a4.5 4.5 0 0 1 7.854-3H9.25a.75.75 0 0 0 0 1.5h3.25a.75.75 0 0 0 .75-.75V2.5a.75.75 0 0 0-1.5 0v1.386A6 6 0 1 0 14 8a.75.75 0 0 0-1.5 0A4.5 4.5 0 1 1 3.5 8Z"
                    fill="currentColor"/>
            </svg>
            Split another
        </button>

        <!-- "Open folder" — primary CTA -->
        <button type="button" class="btn-primary btn-glow result-actions__primary" @click="onRevealDir">
            <svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" width="14" height="14" aria-hidden="true">
                <path fill-rule="evenodd" clip-rule="evenodd"
                    d="M1.5 3.5A1.5 1.5 0 0 1 3 2h2.629A1.5 1.5 0 0 1 6.69 2.44L7.81 3.56A.5.5 0 0 0 8.164 3.7H13a1.5 1.5 0 0 1 1.5 1.5v7A1.5 1.5 0 0 1 13 13.7H3A1.5 1.5 0 0 1 1.5 12.2v-8.7Z"
                    fill="currentColor" opacity="0.85"/>
            </svg>
            Open folder
        </button>

    </div>

</div>
</template>

<style scoped>
/* ─────────────────────────────────────────────────────────────
   Result view container
───────────────────────────────────────────────────────────── */

.result-view {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    width: 100%;
}

/* ─────────────────────────────────────────────────────────────
   Header
───────────────────────────────────────────────────────────── */

.result-header {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding-bottom: var(--space-4);
    border-bottom: 1px solid var(--color-separator);
}

/* Animated ✓ check badge */
.success-badge {
    flex-shrink: 0;
    width: 32px;
    height: 32px;
    color: var(--color-accent);
    filter: drop-shadow(0 0 10px rgba(54, 244, 164, 0.4));
}

.badge-circle {
    opacity: 0.12;
}

.badge-check {
    color: var(--color-accent);
}

/* Header text */
.result-header__text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
}

.result-header__title {
    font-family: var(--font-display);
    font-size: var(--text-2xl);
    font-weight: var(--weight-light);
    color: var(--color-text-primary);
    line-height: var(--leading-tight);
    letter-spacing: var(--tracking-tight);
}

.result-header__summary {
    font-family: var(--font-sans);
    font-size: var(--text-sm);
    color: var(--color-text-secondary);
    line-height: var(--leading-normal);
}

/* Stats chips */
.result-stats {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-shrink: 0;
}

.stat-chip {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    font-family: var(--font-sans);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
    letter-spacing: 0.02em;
    padding: 4px var(--space-2);
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    line-height: 1;
    white-space: nowrap;
}

.stat-chip--files {
    background: var(--color-success-subtle);
    color: var(--color-success-text);
    border-color: rgba(54, 244, 164, 0.2);
}

.stat-chip--time {
    background: var(--color-accent-subtle);
    color: var(--color-accent);
    border-color: rgba(54, 244, 164, 0.2);
}

/* ─────────────────────────────────────────────────────────────
   File list
───────────────────────────────────────────────────────────── */

.file-list-container {
    flex: 1;
    min-height: 0;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    overflow: hidden;
    box-shadow: var(--shadow-sm);
}

/* Directory header bar */
.file-list__header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 7px 14px;
    background: var(--color-surface-raised);
    border-bottom: 1px solid var(--color-border-subtle);
    overflow: hidden;
}

.file-list__header svg {
    flex-shrink: 0;
    color: var(--color-text-quaternary);
}

.file-list__header-label {
    font-family: var(--font-sans);
    font-size: var(--text-xs);
    font-weight: var(--weight-semibold);
    letter-spacing: var(--tracking-wider);
    text-transform: uppercase;
    color: var(--color-text-quaternary);
    flex-shrink: 0;
    white-space: nowrap;
}

.file-list__header-path {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--color-text-tertiary);
    flex: 1;
    min-width: 0;
}

/* Scrollable list */
.file-list {
    height: 100%;
    overflow-y: auto;
    overscroll-behavior: contain;
}

.file-list__inner {
    display: flex;
    flex-direction: column;
    padding: var(--space-1) 0;
}

/* File row */
.file-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 3px 14px;
    min-height: 28px;
    cursor: default;
    position: relative;
    transition: background-color var(--duration-fast) var(--ease-out);
}

.file-row:hover {
    background: var(--color-surface-hover);
}

.file-row__lineno {
    flex-shrink: 0;
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--color-text-quaternary);
    min-width: 26px;
    text-align: right;
    line-height: 1;
    white-space: pre;
    user-select: none;
}

.file-row__name {
    flex: 1;
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    font-weight: var(--weight-regular);
    color: var(--color-text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    transition: color var(--duration-fast) var(--ease-out);
}

.file-row:hover .file-row__name {
    color: var(--color-text-primary);
}

/* Reveal button (hover) */
.file-row__reveal {
    position: absolute;
    right: var(--space-3);
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 2px var(--space-2);
    border: 1px solid rgba(var(--color-accent-rgb), 0.25);
    border-radius: var(--radius-sm);
    font-family: var(--font-sans);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
    color: var(--color-accent);
    background: rgba(var(--color-accent-rgb), 0.06);
    cursor: pointer;
    transition:
        background-color var(--duration-fast) var(--ease-out),
        box-shadow var(--duration-fast) var(--ease-out);
    white-space: nowrap;
    line-height: 1;
    height: 22px;
}

.file-row__reveal:hover {
    background: rgba(var(--color-accent-rgb), 0.12);
    box-shadow: 0 0 8px rgba(var(--color-accent-rgb), 0.15);
}

.file-row__reveal:focus-visible {
    box-shadow: var(--shadow-focus);
}

/* ─────────────────────────────────────────────────────────────
   Actions
───────────────────────────────────────────────────────────── */

.result-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    flex-shrink: 0;
}

/* "Split another" — ghost, pill, compact */
.result-actions__secondary {
    font-family: var(--font-sans);
    font-size: var(--text-sm);
    padding: 8px 18px;
    height: 36px;
    gap: var(--space-2);
    color: var(--color-text-secondary);
}

/* "Open folder" — primary, pill, with neon glow */
.result-actions__primary {
    min-width: 148px;
    height: 40px;
    font-size: var(--text-md);
    gap: var(--space-2);
}
</style>