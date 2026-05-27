<script setup lang="ts">
import { computed } from 'vue'
import type { PdfError } from '@/types'

const props = defineProps<{
    message: string
    kind?: PdfError['kind']
}>()

const emit = defineEmits<{
    retry: []
    dismiss: []
}>()

const hint = computed<string>(() => {
    switch (props.kind) {
        case 'FileNotFound':
            return 'The file may have been moved, renamed, or deleted. Please select a valid PDF file.'
        case 'InvalidPdf':
            return 'The selected file could not be parsed as a PDF. Make sure the file is not corrupted or password-protected.'
        case 'NoPages':
            return 'The PDF document appears to have no pages. Please select a different file.'
        case 'Io':
            return 'A filesystem error occurred. Check that the output directory is accessible and that you have write permissions.'
        case 'Internal':
            return 'An unexpected internal error occurred. Please try again or report the issue if it persists.'
        default:
            return 'Please try again with a different file or output directory.'
    }
})

const kindLabel = computed<string>(() => {
    switch (props.kind) {
        case 'FileNotFound': return 'File Not Found'
        case 'InvalidPdf':   return 'Invalid PDF'
        case 'NoPages':      return 'No Pages Found'
        case 'Io':           return 'IO Error'
        case 'Internal':     return 'Internal Error'
        default:             return 'Something went wrong'
    }
})

function onRetry(): void   { emit('retry') }
function onDismiss(): void { emit('dismiss') }
</script>

<template>
<div class="error-view" role="alert" aria-live="assertive">

    <!-- ── Header ── -->
    <div class="error-header">

        <!-- Animated error icon -->
        <div class="error-icon" aria-hidden="true">
            <svg viewBox="0 0 32 32" fill="none" xmlns="http://www.w3.org/2000/svg" width="32" height="32">
                <circle cx="16" cy="16" r="15" fill="currentColor" class="error-icon__circle" />
                <!-- X mark -->
                <path
                    fill-rule="evenodd"
                    clip-rule="evenodd"
                    d="M11.293 11.293a1 1 0 0 1 1.414 0L16 14.586l3.293-3.293a1 1 0 1 1 1.414 1.414L17.414 16l3.293 3.293a1 1 0 0 1-1.414 1.414L16 17.414l-3.293 3.293a1 1 0 0 1-1.414-1.414L14.586 16l-3.293-3.293a1 1 0 0 1 0-1.414Z"
                    fill="currentColor"
                    class="error-icon__x"
                />
            </svg>
        </div>

        <!-- Title -->
        <div class="error-header__text">
            <h2 class="error-header__title">{{ kindLabel }}</h2>
            <p class="error-header__subtitle">Something went wrong</p>
        </div>

    </div>

    <!-- ── Error message block ── -->
    <div class="error-block">

        <!-- "Error details" label bar -->
        <div class="error-block__label" aria-hidden="true">
            <span class="error-block__label-text">Error details</span>
        </div>

        <!-- Selectable error message -->
        <p class="error-message" data-selectable>
            <span class="error-message__prefix" aria-hidden="true">error:</span>
            {{ message }}
        </p>

    </div>

    <!-- ── Hint / guidance ── -->
    <div class="error-note">

        <!-- Info icon -->
        <svg
            viewBox="0 0 16 16"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
            width="14"
            height="14"
            aria-hidden="true"
            class="error-note__icon"
        >
            <circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="1.2" fill="none" />
            <path d="M8 7v4" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" />
            <circle cx="8" cy="5" r="0.75" fill="currentColor" />
        </svg>

        <p class="error-note__text">{{ hint }}</p>

    </div>

    <div class="separator" role="separator" />

    <!-- ── Actions ── -->
    <div class="error-actions">

        <!-- Dismiss -->
        <button
            type="button"
            class="btn-ghost error-actions__dismiss"
            @click="onDismiss"
        >
            Dismiss
        </button>

        <!-- Try again — primary CTA -->
        <button
            type="button"
            class="btn-primary btn-glow error-actions__retry"
            @click="onRetry"
        >
            <svg
                viewBox="0 0 16 16"
                fill="none"
                xmlns="http://www.w3.org/2000/svg"
                width="14"
                height="14"
                aria-hidden="true"
            >
                <path
                    fill-rule="evenodd"
                    clip-rule="evenodd"
                    d="M3.5 8a4.5 4.5 0 0 1 7.854-3H9.25a.75.75 0 0 0 0 1.5h3.25a.75.75 0 0 0 .75-.75V2.5a.75.75 0 0 0-1.5 0v1.386A6 6 0 1 0 14 8a.75.75 0 0 0-1.5 0A4.5 4.5 0 1 1 3.5 8Z"
                    fill="currentColor"
                />
            </svg>
            Try again
        </button>

    </div>

</div>
</template>

<style scoped>
/* ─────────────────────────────────────────────
   Container
   ───────────────────────────────────────────── */

.error-view {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
    width: 100%;
    padding: var(--space-2) 0;
}

/* ─────────────────────────────────────────────
   Header
   ───────────────────────────────────────────── */

.error-header {
    display: flex;
    align-items: center;
    gap: var(--space-4);
}

/* Error icon */
.error-icon {
    flex-shrink: 0;
    width: 32px;
    height: 32px;
    color: var(--color-error);
    filter: drop-shadow(0 0 8px rgba(248, 81, 73, 0.35));
    animation: error-icon-in var(--duration-slower) var(--ease-spring) forwards;
}

@keyframes error-icon-in {
    0%   { transform: scale(0.5); opacity: 0; }
    70%  { transform: scale(1.15); opacity: 1; }
    100% { transform: scale(1);   opacity: 1; }
}

.error-icon__circle {
    opacity: 0.12;
}

.error-icon__x {
    color: var(--color-error);
}

/* Title + subtitle */
.error-header__text {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
}

.error-header__title {
    font-family: var(--font-display);
    font-size: var(--text-2xl);
    font-weight: var(--weight-light);
    color: var(--color-error-text);
    line-height: var(--leading-tight);
    letter-spacing: var(--tracking-tight);
}

.error-header__subtitle {
    font-family: var(--font-sans);
    font-size: var(--text-sm);
    color: var(--color-text-tertiary);
    line-height: var(--leading-normal);
}

/* ─────────────────────────────────────────────
   Error block
   ───────────────────────────────────────────── */

.error-block {
    border: 1px solid rgba(var(--color-error-rgb), 0.2);
    border-radius: var(--radius-md);
    background: rgba(var(--color-error-rgb), 0.05);
    overflow: hidden;
}

/* "Error details" label bar */
.error-block__label {
    display: flex;
    align-items: center;
    padding: 4px 14px;
    background: rgba(var(--color-error-rgb), 0.07);
    border-bottom: 1px solid rgba(var(--color-error-rgb), 0.14);
}

.error-block__label-text {
    font-family: var(--font-sans);
    font-size: var(--text-xs);
    font-weight: var(--weight-semibold);
    letter-spacing: var(--tracking-wider);
    text-transform: uppercase;
    color: var(--color-error-text);
    opacity: 0.65;
}

/* Selectable error message */
.error-message {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    color: var(--color-error-text);
    line-height: var(--leading-snug);
    padding: 10px 14px;
    user-select: text;
    cursor: text;
    word-break: break-all;
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    flex-wrap: wrap;
}

.error-message__prefix {
    color: var(--color-error);
    font-weight: var(--weight-bold);
    flex-shrink: 0;
    opacity: 0.75;
}

/* ─────────────────────────────────────────────
   Hint / guidance
   ───────────────────────────────────────────── */

.error-note {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
}

.error-note__icon {
    flex-shrink: 0;
    color: var(--color-text-quaternary);
    margin-top: 1px;
}

.error-note__text {
    font-family: var(--font-sans);
    font-size: var(--text-sm);
    color: var(--color-text-tertiary);
    line-height: var(--leading-relaxed);
    flex: 1;
    min-width: 0;
}

/* ─────────────────────────────────────────────
   Actions
   ───────────────────────────────────────────── */

.error-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--space-3);
}

/* "Dismiss" — ghost, compact */
.error-actions__dismiss {
    font-family: var(--font-sans);
    font-size: var(--text-sm);
    padding: 8px 18px;
    height: 36px;
    color: var(--color-text-tertiary);
}

/* "Try again" — primary pill with subtle glow */
.error-actions__retry {
    min-width: 130px;
    height: 40px;
    font-size: var(--text-md);
    gap: var(--space-2);
}

/* ─────────────────────────────────────────────
   Reduced motion
   ───────────────────────────────────────────── */

@media (prefers-reduced-motion: reduce) {
    .error-icon {
        animation: none;
    }
}
</style>