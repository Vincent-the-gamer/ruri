<script setup lang="ts">
import { ref, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import type { AttachedFile } from "../types";

const props = defineProps<{
    disabled?: boolean;
    sending?: boolean;
}>();

const emit = defineEmits<{
    send: [message: string, images: string[], files: AttachedFile[]];
    stop: [];
}>();

const { t } = useI18n();

const inputText = ref("");
const isComposing = ref(false);
const attachedImages = ref<string[]>([]);
const attachedFiles = ref<AttachedFile[]>([]);
const fileInput = ref<HTMLInputElement | null>(null);

const IMAGE_TYPES =
    "image/png,image/jpeg,image/gif,image/webp,image/bmp,image/svg+xml,image/tiff,image/x-icon,image/avif";
const IMAGE_EXTENSIONS = [
    "png",
    "jpg",
    "jpeg",
    "gif",
    "webp",
    "bmp",
    "svg",
    "tiff",
    "tif",
    "ico",
    "avif",
    "heic",
    "heif",
];
const AUDIO_EXTENSIONS = [
    "mp3",
    "wav",
    "ogg",
    "flac",
    "aac",
    "m4a",
    "wma",
    "webm",
    "opus",
];
const AUDIO_TYPES =
    "audio/mpeg,audio/wav,audio/ogg,audio/flac,audio/aac,audio/mp4,audio/x-ms-wma,audio/webm,audio/opus";
const TEXT_EXTENSIONS = [
    "txt",
    "csv",
    "md",
    "markdown",
    "json",
    "xml",
    "html",
    "htm",
    "yaml",
    "yml",
    "log",
    "ini",
    "toml",
    "cfg",
    "conf",
    "rs",
    "py",
    "js",
    "ts",
    "tsx",
    "jsx",
    "java",
    "c",
    "cpp",
    "h",
    "hpp",
    "go",
    "sh",
    "bash",
    "zsh",
    "bat",
    "ps1",
    "sql",
    "r",
    "rb",
    "php",
    "swift",
    "kt",
    "scala",
    "lua",
    "pl",
    "ex",
    "exs",
    "vim",
    "dockerfile",
    "makefile",
    "gitignore",
    "env",
    "css",
    "scss",
    "less",
    "sass",
];
const DOC_EXTENSIONS = [
    "pdf",
    "docx",
    "xlsx",
    "xls",
    "pptx",
    "doc",
    "ppt",
    "odt",
    "ods",
    "odp",
    "rtf",
    "epub",
];

const ALL_ACCEPTED =
    IMAGE_TYPES +
    "," +
    AUDIO_TYPES +
    "," +
    TEXT_EXTENSIONS.map((e) => `.${e}`).join(",") +
    "," +
    DOC_EXTENSIONS.map((e) => `.${e}`).join(",");

function isTextFile(name: string, mimeType: string): boolean {
    if (mimeType.startsWith("text/")) return true;
    const ext = name.split(".").pop()?.toLowerCase() || "";
    return TEXT_EXTENSIONS.includes(ext);
}

function triggerFileInput() {
    fileInput.value?.click();
}

function handleFileSelect(event: Event) {
    const target = event.target as HTMLInputElement;
    if (!target.files) return;
    const files = Array.from(target.files);

    for (const file of files) {
        const ext = file.name.split(".").pop()?.toLowerCase() || "";

        if (file.type.startsWith("image/") || IMAGE_EXTENSIONS.includes(ext)) {
            // Read as data URL for images
            const reader = new FileReader();
            reader.onload = (e) => {
                const dataUrl = e.target?.result as string;
                if (dataUrl) {
                    attachedImages.value.push(dataUrl);
                }
            };
            reader.readAsDataURL(file);
        } else if (
            file.type.startsWith("audio/") ||
            AUDIO_EXTENSIONS.includes(ext)
        ) {
            // Read audio as base64 data URL
            const reader = new FileReader();
            reader.onload = (e) => {
                const dataUrl = e.target?.result as string;
                if (dataUrl) {
                    attachedFiles.value.push({
                        name: file.name,
                        mime_type: file.type || "audio/mpeg",
                        content: dataUrl,
                    });
                }
            };
            reader.readAsDataURL(file);
        } else if (isTextFile(file.name, file.type)) {
            // Read as text for text files
            const reader = new FileReader();
            reader.onload = (e) => {
                const text = e.target?.result as string;
                if (text !== null) {
                    attachedFiles.value.push({
                        name: file.name,
                        mime_type: file.type || "text/plain",
                        content: text,
                    });
                }
            };
            reader.readAsText(file);
        } else {
            // For binary docs (pdf, docx, xlsx, etc.), read as base64
            const reader = new FileReader();
            reader.onload = (e) => {
                const dataUrl = e.target?.result as string;
                if (dataUrl) {
                    attachedFiles.value.push({
                        name: file.name,
                        mime_type: file.type || "application/octet-stream",
                        content: dataUrl,
                    });
                }
            };
            reader.readAsDataURL(file);
        }
    }
    // Reset the input so the same file can be selected again
    target.value = "";
}

function removeImage(index: number) {
    attachedImages.value.splice(index, 1);
}

function removeFile(index: number) {
    attachedFiles.value.splice(index, 1);
}

function handleSend() {
    const text = inputText.value.trim();
    if (
        (!text &&
            attachedImages.value.length === 0 &&
            attachedFiles.value.length === 0) ||
        props.disabled
    )
        return;
    emit("send", text, attachedImages.value, attachedFiles.value);
    inputText.value = "";
    attachedImages.value = [];
    attachedFiles.value = [];
}

function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey && !isComposing.value) {
        e.preventDefault();
        handleSend();
    }
}

// Voice input using Web Speech API
const isRecording = ref(false);
let recognition: any = null;

function toggleVoiceInput() {
    if (isRecording.value) {
        stopVoiceInput();
        return;
    }
    startVoiceInput();
}

function startVoiceInput() {
    const SpeechRecognition =
        (window as any).SpeechRecognition ||
        (window as any).webkitSpeechRecognition;
    if (!SpeechRecognition) {
        alert(t("chat.speechNotSupported"));
        return;
    }
    recognition = new SpeechRecognition();
    recognition.continuous = true;
    recognition.interimResults = true;
    recognition.lang = "zh-CN";

    recognition.onresult = (event: any) => {
        let finalTranscript = "";
        for (let i = event.resultIndex; i < event.results.length; i++) {
            const transcript = event.results[i][0].transcript;
            if (event.results[i].isFinal) {
                finalTranscript += transcript;
            }
        }
        if (finalTranscript) {
            inputText.value += finalTranscript;
        }
    };

    recognition.onerror = () => {
        isRecording.value = false;
    };

    recognition.onend = () => {
        isRecording.value = false;
    };

    recognition.start();
    isRecording.value = true;
}

function stopVoiceInput() {
    if (recognition) {
        recognition.stop();
        recognition = null;
    }
    isRecording.value = false;
}

function handleImageGen() {
    inputText.value =
        "/image " + (inputText.value || t("chat.imageGenPlaceholder"));
}

onUnmounted(() => {
    stopVoiceInput();
});
</script>

<template>
    <div class="chat-input-wrapper">
        <!-- 装饰元素 -->
        <div class="decoration-stars">
            <span class="star star-1">⭐</span>
            <span class="star star-2">✨</span>
            <span class="star star-3">💫</span>
        </div>

        <div class="chat-input-container">
            <!-- Main input box -->
            <div class="input-box">
                <!-- Attached images preview -->
                <div v-if="attachedImages.length > 0" class="attached-previews">
                    <div
                        v-for="(img, idx) in attachedImages"
                        :key="'img-' + idx"
                        class="preview-item image-preview"
                    >
                        <img :src="img" alt="Attached image" />
                        <button
                            class="remove-btn"
                            @click="removeImage(idx)"
                            type="button"
                        >
                            ✕
                        </button>
                    </div>
                </div>

                <!-- Attached files preview -->
                <div v-if="attachedFiles.length > 0" class="attached-previews">
                    <div
                        v-for="(file, idx) in attachedFiles"
                        :key="'file-' + idx"
                        class="preview-item file-chip"
                    >
                        <svg
                            class="file-chip-icon"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        >
                            <path
                                d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"
                            />
                            <polyline points="14 2 14 8 20 8" />
                            <line x1="16" y1="13" x2="8" y2="13" />
                            <line x1="16" y1="17" x2="8" y2="17" />
                            <polyline points="10 9 9 9 8 9" />
                        </svg>
                        <span class="file-chip-name">{{ file.name }}</span>
                        <button
                            class="remove-btn small"
                            @click="removeFile(idx)"
                            type="button"
                        >
                            ✕
                        </button>
                    </div>
                </div>

                <!-- Textarea area -->
                <div class="textarea-area">
                    <textarea
                        v-model="inputText"
                        @keydown="handleKeydown"
                        @compositionstart="isComposing = true"
                        @compositionend="isComposing = false"
                        :placeholder="t('chat.inputPlaceholder')"
                        rows="1"
                        class="input-textarea"
                        @input="
                            (
                                $event.target as HTMLTextAreaElement
                            ).style.height = 'auto';
                            (
                                $event.target as HTMLTextAreaElement
                            ).style.height =
                                Math.min(
                                    ($event.target as HTMLTextAreaElement)
                                        .scrollHeight,
                                    160,
                                ) + 'px';
                        "
                    ></textarea>
                </div>

                <!-- Action bar -->
                <div class="action-bar">
                    <div class="action-tools">
                        <!-- Attach file button -->
                        <input
                            type="file"
                            ref="fileInput"
                            @change="handleFileSelect"
                            :accept="ALL_ACCEPTED"
                            multiple
                            class="hidden"
                        />
                        <button
                            @click="triggerFileInput"
                            class="tool-btn"
                            :title="t('chat.attachFile')"
                            type="button"
                        >
                            <svg
                                class="tool-icon"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <path
                                    d="M21.44 11.05l-9.19 9.19a6 6 0 01-8.49-8.49l9.19-9.19a4 4 0 015.66 5.66l-9.2 9.19a2 2 0 01-2.83-2.83l8.49-8.48"
                                />
                            </svg>
                            <span class="tool-label">{{
                                t("chat.attachFile")
                            }}</span>
                        </button>

                        <!-- Voice input button -->
                        <button
                            @click="toggleVoiceInput"
                            class="tool-btn"
                            :class="{ active: isRecording }"
                            :title="t('chat.voiceInput')"
                            type="button"
                        >
                            <svg
                                class="tool-icon"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <path
                                    d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"
                                />
                                <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
                                <line x1="12" y1="19" x2="12" y2="23" />
                                <line x1="8" y1="23" x2="16" y2="23" />
                            </svg>
                            <span class="tool-label">{{
                                isRecording
                                    ? t("chat.recording")
                                    : t("chat.voiceInput")
                            }}</span>
                        </button>

                        <!-- Image generation button -->
                        <button
                            @click="handleImageGen"
                            class="tool-btn"
                            :title="t('chat.imageGen')"
                            type="button"
                        >
                            <svg
                                class="tool-icon"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <rect
                                    x="3"
                                    y="3"
                                    width="18"
                                    height="18"
                                    rx="2"
                                    ry="2"
                                />
                                <circle cx="8.5" cy="8.5" r="1.5" />
                                <polyline points="21 15 16 10 5 21" />
                            </svg>
                            <span class="tool-label">{{
                                t("chat.imageGen")
                            }}</span>
                        </button>
                    </div>

                    <!-- Stop / Send button -->
                    <button
                        v-if="sending"
                        @click="emit('stop')"
                        class="send-btn stop-btn"
                        type="button"
                        :title="t('chat.stop')"
                    >
                        <svg
                            viewBox="0 0 24 24"
                            fill="currentColor"
                            class="stop-icon"
                        >
                            <rect x="6" y="6" width="12" height="12" rx="2" />
                        </svg>
                    </button>
                    <button
                        v-else
                        @click="handleSend"
                        :disabled="
                            !inputText.trim() &&
                            attachedImages.length === 0 &&
                            attachedFiles.length === 0
                        "
                        class="send-btn"
                        :class="{
                            disabled:
                                !inputText.trim() &&
                                attachedImages.length === 0 &&
                                attachedFiles.length === 0,
                        }"
                        type="button"
                    >
                        <svg
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            class="send-icon"
                        >
                            <line x1="22" y1="2" x2="11" y2="13" />
                            <polygon points="22 2 15 22 11 13 2 9 22 2" />
                        </svg>
                    </button>
                </div>
            </div>
        </div>

        <!-- 底部提示 -->
        <div class="input-hint">
            <span class="hint-text">{{ t("chat.hint") }}</span>
        </div>
    </div>
</template>

<style scoped>
.chat-input-wrapper {
    border-top: 1px solid hsl(var(--border));
    padding: 0.75rem 1.5rem 1rem;
    position: relative;
    overflow: hidden;
    background: hsl(var(--background));
}

.chat-input-wrapper::before {
    content: "";
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: linear-gradient(
        90deg,
        hsl(var(--primary)),
        hsl(280 70% 60%),
        hsl(var(--primary))
    );
    opacity: 0.6;
}

/* Decoration stars */
.decoration-stars {
    position: absolute;
    width: 100%;
    height: 100%;
    pointer-events: none;
    overflow: hidden;
    opacity: 0.4;
}

.dark .decoration-stars {
    opacity: 0.2;
}

.star {
    position: absolute;
    font-size: 0.875rem;
    animation: float 3s ease-in-out infinite;
}

.star-1 {
    top: 20%;
    left: 5%;
    animation-delay: 0s;
}
.star-2 {
    top: 30%;
    right: 10%;
    animation-delay: 1s;
}
.star-3 {
    bottom: 25%;
    left: 15%;
    animation-delay: 2s;
}

@keyframes float {
    0%,
    100% {
        transform: translateY(0) rotate(0deg);
    }
    50% {
        transform: translateY(-8px) rotate(5deg);
    }
}

.chat-input-container {
    max-width: 52rem;
    margin: 0 auto 0.25rem;
    position: relative;
    z-index: 1;
}

/* Main input box */
.input-box {
    background: hsl(var(--card));
    border: 1.5px solid hsl(var(--border));
    border-radius: 1rem;
    padding: 0.5rem;
    transition: all 0.2s ease;
    box-shadow: 0 1px 3px hsl(var(--primary) / 0.05);
}

.input-box:focus-within {
    border-color: hsl(var(--primary));
    box-shadow:
        0 0 0 3px hsl(var(--primary) / 0.1),
        0 2px 8px hsl(var(--primary) / 0.1);
}

/* Attached previews */
.attached-previews {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    padding: 0.5rem 0.5rem 0;
}

.preview-item {
    animation: fadeIn 0.2s ease-out;
}

@keyframes fadeIn {
    from {
        opacity: 0;
        transform: scale(0.95);
    }
    to {
        opacity: 1;
        transform: scale(1);
    }
}

.image-preview {
    position: relative;
    width: 64px;
    height: 64px;
    border-radius: 0.5rem;
    overflow: hidden;
    border: 1px solid hsl(var(--border));
}

.image-preview img {
    width: 100%;
    height: 100%;
    object-fit: cover;
}

.file-chip {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.375rem 0.625rem;
    background: hsl(var(--secondary));
    border: 1px solid hsl(var(--border));
    border-radius: 0.5rem;
    font-size: 0.75rem;
    color: hsl(var(--foreground));
    max-width: 200px;
}

.file-chip-icon {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    color: hsl(var(--primary));
}

.file-chip-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.remove-btn {
    position: absolute;
    top: 2px;
    right: 2px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.6);
    color: white;
    border: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    line-height: 1;
}

.remove-btn:hover {
    background: rgba(0, 0, 0, 0.8);
}

.remove-btn.small {
    position: static;
    width: 16px;
    height: 16px;
    background: hsl(var(--muted) / 0.8);
    color: hsl(var(--muted-foreground));
    font-size: 10px;
}

.remove-btn.small:hover {
    background: #ef4444;
    color: white;
}

/* Textarea */
.textarea-area {
    padding: 0.25rem 0.5rem;
}

.input-textarea {
    width: 100%;
    background: transparent;
    border: none;
    outline: none;
    font-size: 0.9375rem;
    line-height: 1.5;
    color: hsl(var(--foreground));
    resize: none;
    min-height: 1.5rem;
    max-height: 160px;
    font-family: inherit;
    padding: 0;
}

.input-textarea::placeholder {
    color: hsl(var(--muted-foreground));
}

.input-textarea::-webkit-scrollbar {
    width: 4px;
}

.input-textarea::-webkit-scrollbar-track {
    background: transparent;
}

.input-textarea::-webkit-scrollbar-thumb {
    background: hsl(var(--muted));
    border-radius: 2px;
}

/* Action bar */
.action-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.25rem 0.25rem 0.25rem 0.5rem;
}

.action-tools {
    display: flex;
    align-items: center;
    gap: 0.25rem;
}

/* Tool buttons */
.tool-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.375rem 0.625rem;
    border: none;
    border-radius: 0.5rem;
    background: transparent;
    color: hsl(var(--muted-foreground));
    cursor: pointer;
    transition: all 0.2s ease;
    font-size: 0.75rem;
    white-space: nowrap;
}

.tool-btn:hover {
    background: hsl(var(--secondary));
    color: hsl(var(--primary));
}

.tool-btn.active {
    background: hsl(var(--primary) / 0.1);
    color: hsl(var(--primary));
}

.tool-btn.active .tool-icon {
    animation: pulse 1.5s ease-in-out infinite;
}

@keyframes pulse {
    0%,
    100% {
        transform: scale(1);
    }
    50% {
        transform: scale(1.15);
    }
}

.tool-icon {
    width: 1rem;
    height: 1rem;
    flex-shrink: 0;
}

.tool-label {
    font-size: 0.6875rem;
    font-weight: 500;
}

/* Send button */
.send-btn {
    flex-shrink: 0;
    width: 2rem;
    height: 2rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(
        135deg,
        hsl(var(--primary)) 0%,
        hsl(280 70% 60%) 100%
    );
    border: none;
    border-radius: 0.5rem;
    cursor: pointer;
    transition: all 0.2s ease;
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.3);
}

.send-btn:hover:not(.disabled) {
    transform: translateY(-1px);
    box-shadow: 0 4px 12px hsl(var(--primary) / 0.4);
}

.send-btn:active:not(.disabled) {
    transform: translateY(0) scale(0.95);
}

.send-icon {
    width: 1rem;
    height: 1rem;
    color: white;
}

.send-btn.disabled {
    background: hsl(var(--muted));
    cursor: not-allowed;
    opacity: 0.5;
    box-shadow: none;
}

.send-btn.disabled .send-icon {
    color: hsl(var(--muted-foreground));
}

.stop-btn {
    background: linear-gradient(
        135deg,
        hsl(0 72% 51%) 0%,
        hsl(0 60% 40%) 100%
    ) !important;
    box-shadow: 0 2px 8px hsl(0 72% 51% / 0.3) !important;
}

.stop-btn:hover {
    transform: translateY(-1px) !important;
    box-shadow: 0 4px 12px hsl(0 72% 51% / 0.4) !important;
}

.stop-btn:active {
    transform: translateY(0) scale(0.95) !important;
}

.stop-icon {
    width: 0.875rem;
    height: 0.875rem;
    color: white;
}

/* Hidden file input */
.hidden {
    display: none;
}

/* Bottom hint */
.input-hint {
    text-align: center;
    padding-top: 0.375rem;
}

.hint-text {
    font-size: 0.6875rem;
    color: hsl(var(--muted-foreground) / 0.7);
}

/* Responsive */
@media (max-width: 640px) {
    .chat-input-wrapper {
        padding: 0.75rem 1rem 0.75rem;
    }

    .tool-label {
        display: none;
    }

    .tool-btn {
        padding: 0.375rem;
    }

    .star {
        display: none;
    }
}
</style>
