<script setup lang="ts">
import { ref } from "vue";
import type { AttachedFile } from "../types";

const props = defineProps<{
    disabled?: boolean;
}>();

const emit = defineEmits<{
    send: [message: string, images: string[], files: AttachedFile[]];
}>();

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
];
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
            <div class="input-field">
                <!-- Attached images preview -->
                <div v-if="attachedImages.length > 0" class="attached-images">
                    <div
                        v-for="(img, idx) in attachedImages"
                        :key="idx"
                        class="image-preview"
                    >
                        <img :src="img" alt="Attached image" />
                        <button
                            class="remove-image"
                            @click="removeImage(idx)"
                            type="button"
                        >
                            ✕
                        </button>
                    </div>
                </div>

                <!-- Attached files preview -->
                <div v-if="attachedFiles.length > 0" class="attached-files">
                    <div
                        v-for="(file, idx) in attachedFiles"
                        :key="'file-' + idx"
                        class="file-chip"
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
                            class="remove-file"
                            @click="removeFile(idx)"
                            type="button"
                        >
                            ✕
                        </button>
                    </div>
                </div>

                <div class="input-wrapper">
                    <div class="input-icon">
                        <span>💬</span>
                    </div>
                    <textarea
                        v-model="inputText"
                        @keydown="handleKeydown"
                        @compositionstart="isComposing = true"
                        @compositionend="isComposing = false"
                        placeholder="和琉璃说点什么吧... (Enter 发送, Shift+Enter 换行)"
                        rows="3"
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
                    <!-- Attachment button -->
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
                        class="attach-button"
                        title="添加图片或文件"
                        type="button"
                    >
                        <svg
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            class="attach-icon"
                        >
                            <path
                                d="M21.44 11.05l-9.19 9.19a6 6 0 01-8.49-8.49l9.19-9.19a4 4 0 015.66 5.66l-9.2 9.19a2 2 0 01-2.83-2.83l8.49-8.48"
                            />
                        </svg>
                    </button>
                </div>
            </div>
            <button
                @click="handleSend"
                :disabled="
                    !inputText.trim() &&
                    attachedImages.length === 0 &&
                    attachedFiles.length === 0
                "
                class="send-button"
                :class="{
                    disabled:
                        !inputText.trim() &&
                        attachedImages.length === 0 &&
                        attachedFiles.length === 0,
                }"
                :title="
                    inputText.trim() ||
                    attachedImages.length > 0 ||
                    attachedFiles.length > 0
                        ? '发送消息 💕'
                        : '请输入消息后再发送'
                "
            >
                <span class="send-emoji">💌</span>
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

        <!-- 底部提示 -->
        <div class="input-hint">
            <span class="hint-text">💡 琉璃会用粉色的心️回答你哦~</span>
        </div>
    </div>
</template>

<style scoped>
.chat-input-wrapper {
    border-top: 1px solid hsl(var(--border));
    padding: 1.25rem 1.5rem;
    position: relative;
    overflow: hidden;
    background: hsl(var(--background));
    border-radius: 1rem 1rem 0 0;
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

/* 装饰星星 */
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

.chat-input-container {
    max-width: 52rem;
    margin: 0 auto 0.5rem;
    display: flex;
    align-items: flex-end;
    gap: 0.75rem;
    position: relative;
    z-index: 1;
}

.input-field {
    flex: 1;
    position: relative;
}

.input-wrapper {
    position: relative;
    display: flex;
    align-items: flex-end;
    gap: 0.625rem;
}

.input-icon {
    flex-shrink: 0;
    width: 2.25rem;
    height: 2.25rem;
    border-radius: 0.75rem;
    background: linear-gradient(
        135deg,
        hsl(var(--primary)) 0%,
        hsl(280 70% 60%) 100%
    );
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1rem;
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.25);
    animation: pulse-glow 2s ease-in-out infinite;
}

@keyframes pulse-glow {
    0%,
    100% {
        box-shadow: 0 2px 8px hsl(var(--primary) / 0.25);
    }
    50% {
        box-shadow: 0 4px 16px hsl(var(--primary) / 0.35);
    }
}

.input-textarea {
    flex: 1;
    background: hsl(var(--card));
    border: 1.5px solid hsl(var(--border));
    border-radius: 0.875rem;
    padding: 0.75rem 1rem;
    font-size: 0.9375rem;
    line-height: 1.5;
    color: hsl(var(--foreground));
    resize: none;
    min-height: 2.75rem;
    max-height: 160px;
    transition: all 0.2s ease;
    font-family: inherit;
}

.input-textarea::placeholder {
    color: hsl(var(--muted-foreground));
}

.input-textarea:focus {
    outline: none;
    border-color: hsl(var(--primary));
    box-shadow: 0 0 0 3px hsl(var(--primary) / 0.15);
    background: hsl(var(--card));
}

.input-textarea:focus {
    border-color: hsl(var(--primary));
}

.input-textarea::-webkit-scrollbar {
    width: 6px;
}

.input-textarea::-webkit-scrollbar-track {
    background: transparent;
}

.input-textarea::-webkit-scrollbar-thumb {
    background: hsl(var(--muted));
    border-radius: 3px;
}

.input-textarea::-webkit-scrollbar-thumb:hover {
    background: hsl(var(--muted-foreground) / 0.5);
}

/* Attachment button */
.attach-button {
    flex-shrink: 0;
    width: 2.25rem;
    height: 2.25rem;
    border-radius: 0.75rem;
    border: 1.5px solid hsl(var(--border));
    background: hsl(var(--card));
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.2s ease;
    color: hsl(var(--muted-foreground));
}

.attach-button:hover {
    border-color: hsl(var(--primary));
    color: hsl(var(--primary));
    background: hsl(var(--primary) / 0.05);
}

.attach-icon {
    width: 1rem;
    height: 1rem;
}

/* Attached images preview */
.attached-images {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
    margin-left: 2.875rem;
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

.remove-image {
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

.remove-image:hover {
    background: rgba(0, 0, 0, 0.8);
}

/* Attached files preview */
.attached-files {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
    margin-left: 2.875rem;
}

.file-chip {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.375rem 0.625rem;
    background: hsl(var(--card));
    border: 1px solid hsl(var(--border));
    border-radius: 0.5rem;
    font-size: 0.75rem;
    color: hsl(var(--foreground));
    max-width: 200px;
    animation: fadeIn 0.2s ease-out;
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

.remove-file {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: hsl(var(--muted) / 0.8);
    border: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 10px;
    line-height: 1;
    color: hsl(var(--muted-foreground));
    transition: all 0.15s ease;
}

.remove-file:hover {
    background: #ef4444;
    color: white;
}

/* 发送按钮 */
.send-button {
    flex-shrink: 0;
    width: 2.75rem;
    height: 2.75rem;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    background: linear-gradient(
        135deg,
        hsl(var(--primary)) 0%,
        hsl(280 70% 60%) 100%
    );
    border: none;
    border-radius: 0.75rem;
    cursor: pointer;
    transition: all 0.2s ease;
    box-shadow: 0 2px 8px hsl(var(--primary) / 0.3);
    overflow: hidden;
}

.send-button::before {
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(
        135deg,
        rgba(255, 255, 255, 0.2) 0%,
        rgba(255, 255, 255, 0.05) 100%
    );
    opacity: 0;
    transition: opacity 0.2s ease;
}

.send-button:hover:not(.disabled)::before {
    opacity: 1;
}

.send-button:hover:not(.disabled) {
    transform: translateY(-2px);
    box-shadow: 0 4px 16px hsl(var(--primary) / 0.4);
}

.send-button:active:not(.disabled) {
    transform: translateY(0) scale(0.98);
}

.send-icon {
    width: 1.125rem;
    height: 1.125rem;
    color: white;
    transition: transform 0.2s ease;
    position: relative;
    z-index: 1;
}

.send-button:hover:not(.disabled) .send-icon {
    transform: translateX(2px) translateY(-2px);
}

.send-emoji {
    position: absolute;
    font-size: 1rem;
    opacity: 0;
    transform: scale(0.5);
    transition: all 0.2s ease;
}

.send-button:hover:not(.disabled) .send-emoji {
    opacity: 1;
    transform: scale(1);
}

.send-button.disabled {
    background: hsl(var(--muted));
    cursor: not-allowed;
    opacity: 0.5;
    box-shadow: none;
}

.send-button.disabled .send-icon {
    color: hsl(var(--muted-foreground));
}

/* 底部提示 */
.input-hint {
    text-align: center;
    padding-top: 0.25rem;
}

.hint-text {
    font-size: 0.75rem;
    color: hsl(var(--muted-foreground));
}

/* 响应式 */
@media (max-width: 640px) {
    .chat-input-wrapper {
        padding: 1rem;
    }

    .chat-input-container {
        flex-direction: column;
        gap: 0.625rem;
    }

    .input-wrapper {
        gap: 0.5rem;
    }

    .input-icon {
        width: 2rem;
        height: 2rem;
        font-size: 0.875rem;
    }

    .send-button {
        width: 100%;
        height: 2.5rem;
        flex-direction: row;
        gap: 0.5rem;
    }

    .send-button .send-icon {
        width: 1rem;
        height: 1rem;
    }

    .star {
        display: none;
    }

    .attached-images {
        margin-left: 2.5rem;
    }

    .attached-files {
        margin-left: 2.5rem;
    }
}
</style>
