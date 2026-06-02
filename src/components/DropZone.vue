<script setup lang="ts">

import { ref } from 'vue'
import { FileUp } from 'lucide-vue-next'

const props = defineProps<{
    busy: boolean
}>()

const emit = defineEmits<{
    pick: []
    drop: [path: string]
}>()

const isDragOver = ref(false)
let dragCounter = 0

function onDragEnter(event: DragEvent): void {
    event.preventDefault()
    dragCounter++
    isDragOver.value = true
}

function onDragLeave(event: DragEvent): void {
    event.preventDefault()
    dragCounter--
    if (dragCounter <= 0) {
        dragCounter = 0
        isDragOver.value = false
    }
}

function onDragOver(event: DragEvent): void {
    event.preventDefault()
    if (event.dataTransfer) {
        event.dataTransfer.dropEffect = 'copy'
    }
}

function onDrop(event: DragEvent): void {
    event.preventDefault()
    dragCounter = 0
    isDragOver.value = false

    if (props.busy) return

    const files = event.dataTransfer?.files
    if (!files || files.length === 0) return

    const file = files[0]
    const isPdf =
        file.type === 'application/pdf' ||
        file.name.toLowerCase().endsWith('.pdf')
    if (!isPdf) return

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const path: string | undefined = (file as any).path
    if (path) emit('drop', path)
}

function onSelectClick(): void {
    if (!props.busy) emit('pick')
}
</script>

<template>
<div class="drop-zone-wrapper">

    <div
        class="drop-zone"
        :class="{
            'drop-zone--active': isDragOver && !busy,
            'drop-zone--busy': busy,
        }"
        role="button"
        tabindex="0"
        aria-label="Drop a PDF file here or press Space to open the file picker"
        @dragenter="onDragEnter"
        @dragleave="onDragLeave"
        @dragover="onDragOver"
        @drop="onDrop"
        @click="onSelectClick"
        @keydown.space.prevent="onSelectClick"
        @keydown.enter.prevent="onSelectClick"
    >
        <!-- Subtle green glow overlay on drag-over -->
        <div class="drop-zone__glow" aria-hidden="true" />

        <div class="drop-zone__content">

            <!-- PDF document icon -->
            <div
                class="drop-zone__icon"
                :class="{ 'animate-float': !isDragOver && !busy }"
                aria-hidden="true"
            >
                <FileUp :size="62" :stroke-width="1.2" />
            </div>

            <!-- Text labels -->
            <div class="drop-zone__labels">
                <template v-if="isDragOver && !busy">
                    <span class="drop-zone__heading drop-zone__heading--active">Release to load</span>
                </template>
                <template v-else-if="busy">
                    <span class="drop-zone__heading animate-pulse">Loading…</span>
                </template>
                <template v-else>
                    <span class="drop-zone__heading">Drop your PDF here</span>
                    <span class="drop-zone__subtext">or select a file from your computer</span>
                </template>
            </div>

            <!-- Select file button (hidden during drag-over) -->
            <Transition name="fade">
                <div v-if="!isDragOver" class="drop-zone__actions">
                    <button
                        type="button"
                        class="btn-primary drop-zone__btn"
                        :disabled="busy"
                        tabindex="-1"
                        aria-hidden="true"
                        @click.stop="onSelectClick"
                    >
                        Select file
                        <svg viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg" width="14" height="14" aria-hidden="true">
                            <path fill-rule="evenodd" clip-rule="evenodd" d="M3.75 8a.75.75 0 0 1 .75-.75h5.19L7.22 4.78a.75.75 0 0 1 1.06-1.06l3.5 3.5a.75.75 0 0 1 0 1.06l-3.5 3.5a.75.75 0 0 1-1.06-1.06l2.47-2.47H4.5A.75.75 0 0 1 3.75 8Z" fill="currentColor" />
                        </svg>
                    </button>
                </div>
            </Transition>

        </div>
    </div>

    <!-- Hint text below the zone -->
    <p class="drop-zone__hint" aria-live="polite">
        <template v-if="isDragOver && !busy">
            PDF files only
        </template>
        <template v-else>
            Accepts all PDF versions · any number of pages
        </template>
    </p>

</div>
</template>

<style scoped>
.drop-zone-wrapper {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    flex: 1;
}

/* Drop zone container */

.drop-zone {
    width: 100%;
    flex: 1;
    min-height: var(--size-dropzone-min);
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    border: 1.5px dashed var(--color-border);
    cursor: pointer;
    outline: none;
    position: relative;
    overflow: hidden;
    transition:
        background-color var(--duration-normal) var(--ease-out),
        border-color var(--duration-normal) var(--ease-out),
        box-shadow var(--duration-normal) var(--ease-out);
}

.drop-zone:focus-visible {
    box-shadow: var(--shadow-focus);
    border-color: var(--color-accent);
}

/* Drag-over state */
.drop-zone--active {
    background: rgba(var(--color-accent-rgb), 0.03);
    border-color: var(--color-accent);
    box-shadow:
        0 0 0 var(--size-glow-ring) rgba(var(--color-accent-rgb), 0.08),
        inset 0 0 60px rgba(var(--color-accent-rgb), 0.03);
}

/* Busy state */
.drop-zone--busy {
    opacity: var(--opacity-busy);
    cursor: default;
}

/* Radial glow — only visible on drag-over */
.drop-zone__glow {
    position: absolute;
    inset: var(--size-glow-inset);
    background: radial-gradient(
        ellipse at center,
        rgba(var(--color-accent-rgb), var(--opacity-icon-soft)) 0%,
        transparent 60%
    );
    opacity: 0;
    pointer-events: none;
    transition: opacity var(--duration-normal) var(--ease-out);
}

.drop-zone--active .drop-zone__glow {
    opacity: 1;
}

/* Content */

.drop-zone__content {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-5);
    padding: var(--space-8) var(--space-7);
    text-align: center;
    pointer-events: none;
}

.drop-zone__actions {
    pointer-events: auto;
}

/* Icon */

.drop-zone__icon {
    color: var(--color-text-quaternary);
    transition: color var(--duration-normal) var(--ease-out);
}

.drop-zone--active .drop-zone__icon {
    color: var(--color-accent);
}

.icon-body        { opacity: var(--opacity-icon-soft); }
.icon-fold        { opacity: var(--opacity-icon-faint); }
.icon-fold-stroke { opacity: var(--opacity-icon-subtle); }
.icon-band        { opacity: var(--opacity-disabled); }
.icon-label       { opacity: var(--opacity-icon-strong); fill: currentColor; }
.icon-line        { opacity: var(--opacity-icon-subtle); }

.drop-zone--active .icon-body  { opacity: var(--opacity-icon-medium); }
.drop-zone--active .icon-band  { opacity: var(--opacity-icon-strong); }

/* Labels */

.drop-zone__labels {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
}

.drop-zone__heading {
    font-family: var(--font-display);
    font-size: var(--text-2xl);
    font-weight: var(--weight-light);
    color: var(--color-text-primary);
    line-height: var(--leading-snug);
    letter-spacing: var(--tracking-tight);
    transition: color var(--duration-normal) var(--ease-out);
}

.drop-zone__heading--active {
    color: var(--color-accent);
}

.drop-zone__subtext {
    font-family: var(--font-sans);
    font-size: var(--text-sm);
    color: var(--color-text-tertiary);
    font-weight: var(--weight-regular);
    line-height: var(--leading-normal);
}

/* Button */

.drop-zone__btn {
    /* Inherits .btn-primary — white pill */
    gap: var(--space-2);
    font-size: var(--text-md);
    padding: var(--space-dropzone-btn-y) var(--space-dropzone-btn-x-r) var(--space-dropzone-btn-y) var(--space-dropzone-btn-x-l);
    min-height: var(--size-btn-height-md);
}

/* Hint text */

.drop-zone__hint {
    font-family: var(--font-sans);
    font-size: var(--text-xs);
    color: var(--color-text-quaternary);
    letter-spacing: var(--tracking-wide);
    text-align: center;
    line-height: var(--leading-normal);
    min-height: var(--size-hint-min-h);
}
</style>
