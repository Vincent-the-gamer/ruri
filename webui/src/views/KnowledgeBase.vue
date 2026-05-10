<script setup lang="ts">
import { onMounted, ref, reactive, computed } from "vue";
import { useI18n } from "vue-i18n";
import { Icon } from "@iconify/vue";
import { useKnowledgeBaseStore } from "../stores/knowledgeBase";
import type {
    KnowledgeBase,
    CreateKnowledgeBaseRequest,
    UpdateKnowledgeBaseRequest,
    KbDocument,
    SearchResult,
} from "../types";

const { t } = useI18n();
const kbStore = useKnowledgeBaseStore();

// ─── View State ──────────────────────────────────────────────────
type ViewState = "list" | "documents";
const viewState = ref<ViewState>("list");
const activeKb = ref<KnowledgeBase | null>(null);

// ─── Modal State ─────────────────────────────────────────────────
const showModal = ref(false);
const editingKb = ref<KnowledgeBase | null>(null);

const form = reactive({
    name: "",
    description: "",
    // Embedding config
    embBaseUrl: "",
    embApiKey: "",
    embModel: "",
    embDimension: 1024,
    // Rerank config
    rerankEnabled: false,
    rerankBaseUrl: "",
    rerankApiKey: "",
    rerankModel: "",
    // Chunking
    chunkSize: 512,
    chunkOverlap: 64,
});

// ─── Document State ──────────────────────────────────────────────
const documents = ref<KbDocument[]>([]);
const docsLoading = ref(false);
const uploadLoading = ref(false);
const fileInputRef = ref<HTMLInputElement | null>(null);
const isDragging = ref(false);

// ─── Search State ────────────────────────────────────────────────
const searchQuery = ref("");
const searchResults = ref<SearchResult[]>([]);
const searchLoading = ref(false);

// ─── Lifecycle ───────────────────────────────────────────────────
onMounted(() => {
    kbStore.fetchKnowledgeBases();
});

// ─── Knowledge Base CRUD ─────────────────────────────────────────
function openCreate() {
    editingKb.value = null;
    form.name = "";
    form.description = "";
    form.embBaseUrl = "";
    form.embApiKey = "";
    form.embModel = "";
    form.embDimension = 1024;
    form.rerankEnabled = false;
    form.rerankBaseUrl = "";
    form.rerankApiKey = "";
    form.rerankModel = "";
    form.chunkSize = 512;
    form.chunkOverlap = 64;
    showModal.value = true;
}

function openEdit(kb: KnowledgeBase) {
    editingKb.value = kb;
    form.name = kb.name;
    form.description = kb.description;
    form.embBaseUrl = kb.embedding_provider_config.base_url;
    form.embApiKey = kb.embedding_provider_config.api_key || "";
    form.embModel = kb.embedding_provider_config.model;
    form.embDimension = kb.embedding_provider_config.dimension;
    form.rerankEnabled = !!kb.rerank_provider_config;
    form.rerankBaseUrl = kb.rerank_provider_config?.base_url || "";
    form.rerankApiKey = kb.rerank_provider_config?.api_key || "";
    form.rerankModel = kb.rerank_provider_config?.model || "";
    form.chunkSize = kb.chunk_size;
    form.chunkOverlap = kb.chunk_overlap;
    showModal.value = true;
}

async function handleSave() {
    try {
        if (editingKb.value) {
            const data: UpdateKnowledgeBaseRequest = {
                name: form.name,
                description: form.description,
                rerank_provider_config: form.rerankEnabled
                    ? {
                          base_url: form.rerankBaseUrl,
                          api_key: form.rerankApiKey || null,
                          model: form.rerankModel,
                      }
                    : null,
                chunk_size: form.chunkSize,
                chunk_overlap: form.chunkOverlap,
            };
            await kbStore.updateKnowledgeBase(editingKb.value.id, data);
        } else {
            const data: CreateKnowledgeBaseRequest = {
                name: form.name,
                description: form.description,
                embedding_provider_config: {
                    base_url: form.embBaseUrl,
                    api_key: form.embApiKey || null,
                    model: form.embModel,
                    dimension: form.embDimension,
                },
                rerank_provider_config: form.rerankEnabled
                    ? {
                          base_url: form.rerankBaseUrl,
                          api_key: form.rerankApiKey || null,
                          model: form.rerankModel,
                      }
                    : undefined,
                chunk_size: form.chunkSize,
                chunk_overlap: form.chunkOverlap,
            };
            await kbStore.createKnowledgeBase(data);
        }
        showModal.value = false;
        editingKb.value = null;
    } catch {
        // error is in store
    }
}

function handleCancelModal() {
    showModal.value = false;
    editingKb.value = null;
}

async function handleDeleteKb(kb: KnowledgeBase) {
    if (
        !confirm(
            t(
                "knowledgeBase.deleteConfirm",
                "Are you sure you want to delete this knowledge base?",
            ),
        )
    )
        return;
    try {
        await kbStore.deleteKnowledgeBase(kb.id);
        if (activeKb.value?.id === kb.id) {
            viewState.value = "list";
            activeKb.value = null;
        }
    } catch {
        // error is in store
    }
}

// ─── Document Management ─────────────────────────────────────────
function openDocuments(kb: KnowledgeBase) {
    activeKb.value = kb;
    viewState.value = "documents";
    loadDocuments();
}

function backToList() {
    viewState.value = "list";
    activeKb.value = null;
    documents.value = [];
    searchResults.value = [];
    searchQuery.value = "";
}

async function loadDocuments() {
    if (!activeKb.value) return;
    docsLoading.value = true;
    try {
        documents.value = await kbStore.fetchDocuments(activeKb.value.id);
    } finally {
        docsLoading.value = false;
    }
}

async function handleFileUpload(files: FileList | null) {
    if (!files || files.length === 0 || !activeKb.value) return;
    uploadLoading.value = true;
    try {
        await kbStore.uploadDocument(activeKb.value.id, Array.from(files));
        await loadDocuments();
        // Refresh KB list to update counts
        await kbStore.fetchKnowledgeBases();
        const updated = kbStore.knowledgeBases.find(
            (k) => k.id === activeKb.value!.id,
        );
        if (updated) activeKb.value = updated;
    } catch {
        // error is in store
    } finally {
        uploadLoading.value = false;
    }
    // Reset file input
    if (fileInputRef.value) fileInputRef.value.value = "";
}

function onDragOver(e: DragEvent) {
    e.preventDefault();
    isDragging.value = true;
}

function onDragLeave() {
    isDragging.value = false;
}

function onDrop(e: DragEvent) {
    e.preventDefault();
    isDragging.value = false;
    handleFileUpload(e.dataTransfer?.files || null);
}

function triggerFileSelect() {
    fileInputRef.value?.click();
}

async function handleDeleteDoc(doc: KbDocument) {
    if (!activeKb.value) return;
    try {
        await kbStore.deleteDocument(activeKb.value.id, doc.id);
        documents.value = documents.value.filter((d) => d.id !== doc.id);
        await kbStore.fetchKnowledgeBases();
        const updated = kbStore.knowledgeBases.find(
            (k) => k.id === activeKb.value!.id,
        );
        if (updated) activeKb.value = updated;
    } catch {
        // error is in store
    }
}

// ─── Search ──────────────────────────────────────────────────────
async function handleSearch() {
    if (!searchQuery.value.trim() || !activeKb.value) return;
    searchLoading.value = true;
    try {
        searchResults.value = await kbStore.search(activeKb.value.id, {
            query: searchQuery.value,
            top_k: 5,
        });
    } finally {
        searchLoading.value = false;
    }
}

// ─── Helpers ─────────────────────────────────────────────────────
function formatFileSize(bytes: number): string {
    if (bytes < 1024) return bytes + " B";
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
    return (bytes / (1024 * 1024)).toFixed(1) + " MB";
}

function statusColor(status: string): string {
    switch (status) {
        case "completed":
            return "bg-emerald-500/20 text-emerald-400 border-emerald-500/30";
        case "processing":
            return "bg-blue-500/20 text-blue-400 border-blue-500/30";
        case "pending":
            return "bg-yellow-500/20 text-yellow-400 border-yellow-500/30";
        case "failed":
            return "bg-red-500/20 text-red-400 border-red-500/30";
        default:
            return "bg-zinc-500/20 text-zinc-400 border-zinc-500/30";
    }
}

const formValid = computed(() => {
    return (
        form.name.trim() !== "" &&
        form.embBaseUrl.trim() !== "" &&
        form.embModel.trim() !== "" &&
        form.embDimension > 0
    );
});
</script>

<template>
    <div class="min-h-screen p-6 space-y-6">
        <!-- ═══════════════════════════════════════════════════════════ -->
        <!-- LIST VIEW                                                   -->
        <!-- ═══════════════════════════════════════════════════════════ -->
        <template v-if="viewState === 'list'">
            <!-- Header -->
            <div class="flex items-start justify-between gap-4">
                <div class="flex items-center gap-4">
                    <div
                        class="flex items-center justify-center w-12 h-12 rounded-xl bg-primary/10 border border-primary/20"
                    >
                        <Icon
                            icon="lucide:book-open"
                            class="text-2xl text-primary"
                        />
                    </div>
                    <div>
                        <h1 class="text-2xl font-bold text-foreground">
                            {{ t("knowledgeBase.title", "Knowledge Base") }}
                        </h1>
                        <p class="text-sm text-muted-foreground mt-0.5">
                            {{
                                t(
                                    "knowledgeBase.subtitle",
                                    "Manage RAG knowledge bases with embedding and reranking models",
                                )
                            }}
                        </p>
                    </div>
                </div>
                <button
                    class="flex items-center gap-2 px-4 py-2.5 rounded-lg bg-primary text-primary-foreground font-semibold text-sm hover:bg-primary/90 transition-all duration-200 shadow-lg shadow-primary/20"
                    @click="openCreate"
                >
                    <Icon icon="lucide:plus" class="text-lg" />
                    {{ t("knowledgeBase.create", "Create Knowledge Base") }}
                </button>
            </div>

            <!-- Error Banner -->
            <div
                v-if="kbStore.error"
                class="flex items-center gap-3 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm"
            >
                <Icon
                    icon="lucide:alert-circle"
                    class="text-lg flex-shrink-0"
                />
                <span>{{ kbStore.error }}</span>
            </div>

            <!-- Loading State -->
            <div
                v-if="kbStore.loading"
                class="flex items-center justify-center py-20"
            >
                <div class="flex items-center gap-3 text-muted-foreground">
                    <Icon
                        icon="lucide:loader-2"
                        class="text-2xl animate-spin"
                    />
                    <span class="text-sm">Loading...</span>
                </div>
            </div>

            <!-- Empty State -->
            <div
                v-else-if="kbStore.knowledgeBases.length === 0"
                class="flex flex-col items-center justify-center py-20"
            >
                <div
                    class="flex items-center justify-center w-20 h-20 rounded-2xl bg-primary/10 border border-primary/20 mb-6"
                >
                    <Icon
                        icon="lucide:book-open"
                        class="text-4xl text-primary/50"
                    />
                </div>
                <h3 class="text-lg font-semibold text-foreground mb-2">
                    {{
                        t(
                            "knowledgeBase.noKnowledgeBases",
                            "No knowledge bases yet",
                        )
                    }}
                </h3>
                <p
                    class="text-sm text-muted-foreground mb-6 text-center max-w-md"
                >
                    {{
                        t(
                            "knowledgeBase.noKnowledgeBasesDesc",
                            "Create a knowledge base to start building your RAG system",
                        )
                    }}
                </p>
                <button
                    class="flex items-center gap-2 px-4 py-2.5 rounded-lg bg-primary text-primary-foreground font-semibold text-sm hover:bg-primary/90 transition-all duration-200"
                    @click="openCreate"
                >
                    <Icon icon="lucide:plus" class="text-lg" />
                    {{ t("knowledgeBase.create", "Create Knowledge Base") }}
                </button>
            </div>

            <!-- Knowledge Base Cards Grid -->
            <div
                v-else
                class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4"
            >
                <div
                    v-for="kb in kbStore.knowledgeBases"
                    :key="kb.id"
                    class="group relative bg-card/50 backdrop-blur-xl border border-border/30 rounded-xl p-5 hover:border-primary/30 hover:shadow-lg hover:shadow-primary/5 transition-all duration-300"
                >
                    <!-- Card Header -->
                    <div class="flex items-start justify-between gap-3 mb-3">
                        <div class="flex-1 min-w-0">
                            <h3
                                class="text-base font-semibold text-card-foreground truncate"
                            >
                                {{ kb.name }}
                            </h3>
                            <p
                                v-if="kb.description"
                                class="text-xs text-muted-foreground mt-1 line-clamp-2"
                            >
                                {{ kb.description }}
                            </p>
                        </div>
                        <span
                            class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-semibold bg-primary/15 text-primary border border-primary/20 flex-shrink-0"
                        >
                            {{ kb.embedding_provider_config.dimension }}d
                        </span>
                    </div>

                    <!-- Model Info -->
                    <div class="space-y-1.5 mb-4">
                        <div
                            class="flex items-center gap-2 text-xs text-muted-foreground"
                        >
                            <Icon
                                icon="lucide:cpu"
                                class="text-sm flex-shrink-0"
                            />
                            <span class="truncate">{{
                                kb.embedding_provider_config.model
                            }}</span>
                        </div>
                        <div
                            v-if="kb.rerank_provider_config"
                            class="flex items-center gap-2 text-xs text-muted-foreground"
                        >
                            <Icon
                                icon="lucide:arrow-up-down"
                                class="text-sm flex-shrink-0"
                            />
                            <span class="truncate">{{
                                kb.rerank_provider_config.model
                            }}</span>
                        </div>
                    </div>

                    <!-- Stats -->
                    <div class="flex items-center gap-4 mb-4">
                        <div
                            class="flex items-center gap-1.5 text-xs text-muted-foreground"
                        >
                            <Icon icon="lucide:file-text" class="text-sm" />
                            <span
                                >{{ kb.document_count }}
                                {{
                                    t("knowledgeBase.documents", "Documents")
                                }}</span
                            >
                        </div>
                        <div
                            class="flex items-center gap-1.5 text-xs text-muted-foreground"
                        >
                            <Icon icon="lucide:layers" class="text-sm" />
                            <span
                                >{{ kb.chunk_count }}
                                {{ t("knowledgeBase.chunks", "Chunks") }}</span
                            >
                        </div>
                    </div>

                    <!-- Actions -->
                    <div
                        class="flex items-center gap-2 pt-3 border-t border-border/20"
                    >
                        <button
                            class="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium text-primary bg-primary/10 hover:bg-primary/20 border border-primary/20 transition-all duration-200"
                            @click="openDocuments(kb)"
                        >
                            <Icon icon="lucide:file-text" class="text-sm" />
                            {{ t("knowledgeBase.documents", "Documents") }}
                        </button>
                        <button
                            class="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium text-muted-foreground hover:text-foreground hover:bg-secondary/50 border border-transparent hover:border-border/30 transition-all duration-200"
                            @click="openEdit(kb)"
                        >
                            <Icon icon="lucide:pencil" class="text-sm" />
                            {{ t("knowledgeBase.edit", "Edit") }}
                        </button>
                        <button
                            class="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium text-red-400 hover:text-red-300 hover:bg-red-500/10 border border-transparent hover:border-red-500/20 transition-all duration-200 ml-auto"
                            @click="handleDeleteKb(kb)"
                        >
                            <Icon icon="lucide:trash-2" class="text-sm" />
                        </button>
                    </div>
                </div>
            </div>
        </template>

        <!-- ═══════════════════════════════════════════════════════════ -->
        <!-- DOCUMENT VIEW                                               -->
        <!-- ═══════════════════════════════════════════════════════════ -->
        <template v-else-if="viewState === 'documents' && activeKb">
            <!-- Back Header -->
            <div class="flex items-center gap-4">
                <button
                    class="flex items-center gap-2 px-3 py-2 rounded-lg text-sm font-medium text-muted-foreground hover:text-foreground hover:bg-secondary/50 border border-border/30 transition-all duration-200"
                    @click="backToList"
                >
                    <Icon icon="lucide:arrow-left" class="text-lg" />
                    {{ t("knowledgeBase.back", "Back to Knowledge Bases") }}
                </button>
                <div class="h-6 w-px bg-border/40" />
                <div class="flex items-center gap-3">
                    <Icon
                        icon="lucide:book-open"
                        class="text-xl text-primary"
                    />
                    <h2 class="text-xl font-bold text-foreground">
                        {{ activeKb.name }}
                    </h2>
                    <span class="text-xs text-muted-foreground">{{
                        activeKb.embedding_provider_config.model
                    }}</span>
                </div>
            </div>

            <!-- Error Banner -->
            <div
                v-if="kbStore.error"
                class="flex items-center gap-3 px-4 py-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm"
            >
                <Icon
                    icon="lucide:alert-circle"
                    class="text-lg flex-shrink-0"
                />
                <span>{{ kbStore.error }}</span>
            </div>

            <!-- Upload Area -->
            <div
                class="bg-card/50 backdrop-blur-xl border border-border/30 rounded-xl p-5"
            >
                <h3
                    class="text-sm font-semibold text-card-foreground mb-3 flex items-center gap-2"
                >
                    <Icon icon="lucide:upload" class="text-lg text-primary" />
                    {{ t("knowledgeBase.uploadDocuments", "Upload Documents") }}
                </h3>
                <div
                    class="border-2 border-dashed rounded-lg p-8 text-center transition-all duration-200 cursor-pointer"
                    :class="[
                        isDragging
                            ? 'border-primary bg-primary/5'
                            : 'border-border/40 hover:border-primary/40 hover:bg-primary/5',
                        uploadLoading ? 'opacity-50 pointer-events-none' : '',
                    ]"
                    @dragover="onDragOver"
                    @dragleave="onDragLeave"
                    @drop="onDrop"
                    @click="triggerFileSelect"
                >
                    <Icon
                        icon="lucide:cloud-upload"
                        class="text-3xl text-muted-foreground mx-auto mb-2"
                    />
                    <p class="text-sm text-muted-foreground">
                        {{
                            t(
                                "knowledgeBase.uploadHint",
                                "Drag and drop files here, or click to select",
                            )
                        }}
                    </p>
                    <p class="text-xs text-muted-foreground/60 mt-1">
                        {{
                            t(
                                "knowledgeBase.supportedFormats",
                                "Supported: txt, md, csv, pdf, xls, xlsx, docx",
                            )
                        }}
                    </p>
                    <div
                        v-if="uploadLoading"
                        class="flex items-center justify-center gap-2 mt-3 text-primary"
                    >
                        <Icon
                            icon="lucide:loader-2"
                            class="text-lg animate-spin"
                        />
                        <span class="text-sm">Uploading...</span>
                    </div>
                    <input
                        ref="fileInputRef"
                        type="file"
                        multiple
                        class="hidden"
                        @change="
                            handleFileUpload(
                                ($event.target as HTMLInputElement).files,
                            )
                        "
                    />
                </div>
            </div>

            <!-- Document Table -->
            <div
                class="bg-card/50 backdrop-blur-xl border border-border/30 rounded-xl p-5"
            >
                <h3
                    class="text-sm font-semibold text-card-foreground mb-4 flex items-center gap-2"
                >
                    <Icon
                        icon="lucide:file-text"
                        class="text-lg text-primary"
                    />
                    {{ t("knowledgeBase.documents", "Documents") }}
                </h3>

                <!-- Docs Loading -->
                <div
                    v-if="docsLoading"
                    class="flex items-center justify-center py-10"
                >
                    <Icon
                        icon="lucide:loader-2"
                        class="text-2xl animate-spin text-muted-foreground"
                    />
                </div>

                <!-- No Documents -->
                <div
                    v-else-if="documents.length === 0"
                    class="flex flex-col items-center py-10 text-muted-foreground"
                >
                    <Icon
                        icon="lucide:file-x"
                        class="text-4xl mb-3 opacity-50"
                    />
                    <p class="text-sm">
                        {{ t("knowledgeBase.noDocuments", "No documents yet") }}
                    </p>
                </div>

                <!-- Document Table -->
                <div v-else class="overflow-x-auto">
                    <table class="w-full text-sm">
                        <thead>
                            <tr class="border-b border-border/30">
                                <th
                                    class="text-left py-2 px-3 text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                                >
                                    {{
                                        t("knowledgeBase.filename", "Filename")
                                    }}
                                </th>
                                <th
                                    class="text-left py-2 px-3 text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                                >
                                    {{ t("knowledgeBase.fileSize", "Size") }}
                                </th>
                                <th
                                    class="text-left py-2 px-3 text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                                >
                                    {{ t("knowledgeBase.fileType", "Type") }}
                                </th>
                                <th
                                    class="text-left py-2 px-3 text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                                >
                                    {{
                                        t("knowledgeBase.chunkCount", "Chunks")
                                    }}
                                </th>
                                <th
                                    class="text-left py-2 px-3 text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                                >
                                    {{ t("knowledgeBase.status", "Status") }}
                                </th>
                                <th
                                    class="text-right py-2 px-3 text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                                >
                                    {{ t("knowledgeBase.actions", "Actions") }}
                                </th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr
                                v-for="doc in documents"
                                :key="doc.id"
                                class="border-b border-border/10 hover:bg-secondary/20 transition-colors"
                            >
                                <td class="py-3 px-3">
                                    <div class="flex items-center gap-2">
                                        <Icon
                                            icon="lucide:file"
                                            class="text-muted-foreground flex-shrink-0"
                                        />
                                        <span
                                            class="text-card-foreground truncate max-w-48"
                                            >{{ doc.filename }}</span
                                        >
                                    </div>
                                </td>
                                <td class="py-3 px-3 text-muted-foreground">
                                    {{ formatFileSize(doc.file_size) }}
                                </td>
                                <td class="py-3 px-3">
                                    <span
                                        class="px-2 py-0.5 rounded text-xs bg-secondary/50 text-muted-foreground border border-border/20"
                                    >
                                        {{ doc.file_type }}
                                    </span>
                                </td>
                                <td class="py-3 px-3 text-muted-foreground">
                                    {{ doc.chunk_count }}
                                </td>
                                <td class="py-3 px-3">
                                    <span
                                        class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium border"
                                        :class="statusColor(doc.status)"
                                    >
                                        {{ doc.status }}
                                    </span>
                                </td>
                                <td class="py-3 px-3 text-right">
                                    <button
                                        class="p-1.5 rounded-md text-red-400 hover:text-red-300 hover:bg-red-500/10 transition-all duration-200"
                                        :title="'Delete'"
                                        @click="handleDeleteDoc(doc)"
                                    >
                                        <Icon
                                            icon="lucide:trash-2"
                                            class="text-sm"
                                        />
                                    </button>
                                </td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </div>

            <!-- Search Test -->
            <div
                class="bg-card/50 backdrop-blur-xl border border-border/30 rounded-xl p-5"
            >
                <h3
                    class="text-sm font-semibold text-card-foreground mb-4 flex items-center gap-2"
                >
                    <Icon icon="lucide:search" class="text-lg text-primary" />
                    {{ t("knowledgeBase.search", "Search") }}
                </h3>
                <div class="flex items-center gap-3 mb-4">
                    <input
                        v-model="searchQuery"
                        type="text"
                        :placeholder="
                            t(
                                'knowledgeBase.searchPlaceholder',
                                'Enter a query to test search...',
                            )
                        "
                        class="flex-1 rounded-lg border border-border/50 bg-background/50 px-4 py-2 text-sm text-foreground placeholder:text-muted-foreground/60 focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/30 transition-all duration-200"
                        @keyup.enter="handleSearch"
                    />
                    <button
                        class="flex items-center gap-2 px-4 py-2 rounded-lg bg-primary text-primary-foreground font-semibold text-sm hover:bg-primary/90 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
                        :disabled="searchLoading || !searchQuery.trim()"
                        @click="handleSearch"
                    >
                        <Icon
                            v-if="searchLoading"
                            icon="lucide:loader-2"
                            class="text-sm animate-spin"
                        />
                        <Icon v-else icon="lucide:search" class="text-sm" />
                        {{ t("knowledgeBase.search", "Search") }}
                    </button>
                </div>

                <!-- Search Results -->
                <div v-if="searchResults.length > 0" class="space-y-3">
                    <h4
                        class="text-xs font-semibold text-muted-foreground uppercase tracking-wider"
                    >
                        {{ t("knowledgeBase.searchResults", "Search Results") }}
                    </h4>
                    <div
                        v-for="(result, i) in searchResults"
                        :key="i"
                        class="bg-background/40 border border-border/20 rounded-lg p-4"
                    >
                        <div class="flex items-center gap-3 mb-2">
                            <span
                                class="inline-flex items-center justify-center px-2 py-0.5 rounded-full text-xs font-bold bg-primary/15 text-primary border border-primary/20"
                            >
                                {{ (result.score * 100).toFixed(1) }}%
                            </span>
                            <span class="text-xs text-muted-foreground">
                                {{ t("knowledgeBase.source", "Source") }}:
                                <span class="text-foreground font-medium">{{
                                    result.source
                                }}</span>
                            </span>
                        </div>
                        <p
                            class="text-sm text-card-foreground leading-relaxed whitespace-pre-wrap"
                        >
                            {{ result.content }}
                        </p>
                    </div>
                </div>
                <div
                    v-else-if="searchQuery && !searchLoading"
                    class="text-center py-6 text-muted-foreground text-sm"
                >
                    No results found
                </div>
            </div>
        </template>

        <!-- ═══════════════════════════════════════════════════════════ -->
        <!-- CREATE / EDIT MODAL                                         -->
        <!-- ═══════════════════════════════════════════════════════════ -->
        <Teleport to="body">
            <div
                v-if="showModal"
                class="fixed inset-0 z-50 flex items-center justify-center p-4"
            >
                <!-- Overlay -->
                <div
                    class="absolute inset-0 bg-black/60 backdrop-blur-sm"
                    @click="handleCancelModal"
                />

                <!-- Modal Content -->
                <div
                    class="relative w-full max-w-2xl max-h-[85vh] overflow-y-auto bg-card/90 backdrop-blur-xl border border-border/30 rounded-2xl shadow-2xl"
                    @click.stop
                >
                    <!-- Modal Header -->
                    <div
                        class="flex items-center justify-between p-6 border-b border-border/20"
                    >
                        <h2 class="text-lg font-bold text-foreground">
                            {{
                                editingKb
                                    ? t(
                                          "knowledgeBase.edit",
                                          "Edit Knowledge Base",
                                      )
                                    : t(
                                          "knowledgeBase.create",
                                          "Create Knowledge Base",
                                      )
                            }}
                        </h2>
                        <button
                            class="p-2 rounded-lg text-muted-foreground hover:text-foreground hover:bg-secondary/50 transition-all duration-200"
                            @click="handleCancelModal"
                        >
                            <Icon icon="lucide:x" class="text-lg" />
                        </button>
                    </div>

                    <!-- Modal Body -->
                    <div class="p-6 space-y-5">
                        <!-- Name -->
                        <div>
                            <label
                                class="block text-sm font-medium text-foreground mb-1.5"
                            >
                                {{ t("knowledgeBase.name", "Name") }}
                                <span class="text-red-400">*</span>
                            </label>
                            <input
                                v-model="form.name"
                                type="text"
                                placeholder="My Knowledge Base"
                                class="w-full rounded-lg border border-border/50 bg-background/50 px-4 py-2.5 text-sm text-foreground placeholder:text-muted-foreground/60 focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/30 transition-all duration-200"
                            />
                        </div>

                        <!-- Description -->
                        <div>
                            <label
                                class="block text-sm font-medium text-foreground mb-1.5"
                            >
                                {{
                                    t(
                                        "knowledgeBase.description",
                                        "Description",
                                    )
                                }}
                            </label>
                            <input
                                v-model="form.description"
                                type="text"
                                placeholder="A description for this knowledge base"
                                class="w-full rounded-lg border border-border/50 bg-background/50 px-4 py-2.5 text-sm text-foreground placeholder:text-muted-foreground/60 focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/30 transition-all duration-200"
                            />
                        </div>

                        <!-- Embedding Config Section -->
                        <div class="space-y-4">
                            <div class="flex items-center gap-2">
                                <Icon icon="lucide:cpu" class="text-primary" />
                                <h3
                                    class="text-sm font-semibold text-foreground"
                                >
                                    {{
                                        t(
                                            "knowledgeBase.embeddingConfig",
                                            "Embedding Model Configuration",
                                        )
                                    }}
                                </h3>
                            </div>

                            <div
                                class="grid grid-cols-1 sm:grid-cols-2 gap-4 pl-6"
                            >
                                <!-- Base URL -->
                                <div class="sm:col-span-2">
                                    <label
                                        class="block text-xs font-medium text-muted-foreground mb-1"
                                    >
                                        {{
                                            t(
                                                "knowledgeBase.baseUrl",
                                                "Base URL",
                                            )
                                        }}
                                        <span class="text-red-400">*</span>
                                    </label>
                                    <input
                                        v-model="form.embBaseUrl"
                                        type="text"
                                        :disabled="!!editingKb"
                                        placeholder="e.g. https://api.siliconflow.cn/v1"
                                        class="w-full rounded-lg border border-border/50 bg-background/50 px-4 py-2 text-sm text-foreground placeholder:text-muted-foreground/60 focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/30 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
                                    />
                                </div>

                                <!-- API Key -->
                                <div class="sm:col-span-2">
                                    <label
                                        class="block text-xs font-medium text-muted-foreground mb-1"
                                    >
                                        {{
                                            t("knowledgeBase.apiKey", "API Key")
                                        }}
                                    </label>
                                    <input
                                        v-model="form.embApiKey"
                                        type="password"
                                        :disabled="!!editingKb"
                                        placeholder="sk-..."
                                        class="w-full rounded-lg border border-border/50 bg-background/50 px-4 py-2 text-sm text-foreground placeholder:text-muted-foreground/60 focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/30 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
                                    />
                                </div>

                                <!-- Model -->
                                <div>
                                    <label
                                        class="block text-xs font-medium text-muted-foreground mb-1"
                                    >
                                        {{ t("knowledgeBase.model", "Model") }}
                                        <span class="text-red-400">*</span>
                                    </label>
                                    <input
                                        v-model="form.embModel"
                                        type="text"
                                        :disabled="!!editingKb"
                                        placeholder="BAAI/bge-m3"
                                        class="w-full rounded-lg border border-border/50 bg-background/50 px-4 py-2 text-sm text-foreground placeholder:text-muted-foreground/60 focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/30 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
                                    />
                                </div>

                                <!-- Dimension -->
                                <div>
                                    <label
                                        class="block text-xs font-medium text-muted-foreground mb-1"
                                    >
                                        {{
                                            t(
                                                "knowledgeBase.dimension",
                                                "Dimension",
                                            )
                                        }}
                                        <span class="text-red-400">*</span>
                                    </label>
                                    <input
                                        v-model.number="form.embDimension"
                                        type="number"
                                        :disabled="!!editingKb"
                                        min="1"
                                        placeholder="1024"
                                        class="w-full rounded-lg border border-border/50 bg-background/50 px-4 py-2 text-sm text-foreground placeholder:text-muted-foreground/60 focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/30 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
                                    />
                                </div>
                            </div>
                        </div>

                        <!-- Rerank Config Section (Collapsible) -->
                        <div
                            class="border border-border/20 rounded-xl overflow-hidden"
                        >
                            <button
                                class="w-full flex items-center justify-between px-4 py-3 text-sm font-medium text-foreground hover:bg-secondary/30 transition-all duration-200"
                                @click="
                                    form.rerankEnabled = !form.rerankEnabled
                                "
                            >
                                <div class="flex items-center gap-2">
                                    <Icon
                                        icon="lucide:arrow-up-down"
                                        class="text-primary"
                                    />
                                    {{
                                        t(
                                            "knowledgeBase.rerankConfig",
                                            "Rerank Model Configuration (Optional)",
                                        )
                                    }}
                                </div>
                                <Icon
                                    :icon="
                                        form.rerankEnabled
                                            ? 'lucide:chevron-up'
                                            : 'lucide:chevron-down'
                                    "
                                    class="text-muted-foreground"
                                />
                            </button>

                            <div
                                v-if="form.rerankEnabled"
                                class="px-4 pb-4 space-y-4"
                            >
                                <div
                                    class="grid grid-cols-1 sm:grid-cols-2 gap-4"
                                >
                                    <!-- Rerank Base URL -->
                                    <div class="sm:col-span-2">
                                        <label
                                            class="block text-xs font-medium text-muted-foreground mb-1"
                                        >
                                            {{
                                                t(
                                                    "knowledgeBase.baseUrl",
                                                    "Base URL",
                                                )
                                            }}
                                        </label>
                                        <input
                                            v-model="form.rerankBaseUrl"
                                            type="text"
                                            placeholder="https://api.siliconflow.cn/v1"
                                            class="w-full rounded-lg border border-border/50 bg-background/50 px-4 py-2 text-sm text-foreground placeholder:text-muted-foreground/60 focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/30 transition-all duration-200"
                                        />
                                    </div>

                                    <!-- Rerank API Key -->
                                    <div class="sm:col-span-2">
                                        <label
                                            class="block text-xs font-medium text-muted-foreground mb-1"
                                        >
                                            {{
                                                t(
                                                    "knowledgeBase.apiKey",
                                                    "API Key",
                                                )
                                            }}
                                        </label>
                                        <input
                                            v-model="form.rerankApiKey"
                                            type="password"
                                            placeholder="sk-..."
                                            class="w-full rounded-lg border border-border/50 bg-background/50 px-4 py-2 text-sm text-foreground placeholder:text-muted-foreground/60 focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/30 transition-all duration-200"
                                        />
                                    </div>

                                    <!-- Rerank Model -->
                                    <div class="sm:col-span-2">
                                        <label
                                            class="block text-xs font-medium text-muted-foreground mb-1"
                                        >
                                            {{
                                                t(
                                                    "knowledgeBase.model",
                                                    "Model",
                                                )
                                            }}
                                        </label>
                                        <input
                                            v-model="form.rerankModel"
                                            type="text"
                                            placeholder="BAAI/bge-reranker-v2-m3"
                                            class="w-full rounded-lg border border-border/50 bg-background/50 px-4 py-2 text-sm text-foreground placeholder:text-muted-foreground/60 focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/30 transition-all duration-200"
                                        />
                                    </div>
                                </div>
                            </div>
                        </div>

                        <!-- Chunking Config -->
                        <div>
                            <div class="flex items-center gap-2 mb-3">
                                <Icon
                                    icon="lucide:scissors"
                                    class="text-primary"
                                />
                                <h3
                                    class="text-sm font-semibold text-foreground"
                                >
                                    Chunking
                                </h3>
                            </div>
                            <div class="grid grid-cols-2 gap-4 pl-6">
                                <div>
                                    <label
                                        class="block text-xs font-medium text-muted-foreground mb-1"
                                    >
                                        {{
                                            t(
                                                "knowledgeBase.chunkSize",
                                                "Chunk Size",
                                            )
                                        }}
                                    </label>
                                    <input
                                        v-model.number="form.chunkSize"
                                        type="number"
                                        min="1"
                                        placeholder="512"
                                        class="w-full rounded-lg border border-border/50 bg-background/50 px-4 py-2 text-sm text-foreground placeholder:text-muted-foreground/60 focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/30 transition-all duration-200"
                                    />
                                </div>
                                <div>
                                    <label
                                        class="block text-xs font-medium text-muted-foreground mb-1"
                                    >
                                        {{
                                            t(
                                                "knowledgeBase.chunkOverlap",
                                                "Chunk Overlap",
                                            )
                                        }}
                                    </label>
                                    <input
                                        v-model.number="form.chunkOverlap"
                                        type="number"
                                        min="0"
                                        placeholder="64"
                                        class="w-full rounded-lg border border-border/50 bg-background/50 px-4 py-2 text-sm text-foreground placeholder:text-muted-foreground/60 focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/30 transition-all duration-200"
                                    />
                                </div>
                            </div>
                        </div>
                    </div>

                    <!-- Modal Footer -->
                    <div
                        class="flex items-center justify-end gap-3 p-6 border-t border-border/20"
                    >
                        <button
                            class="px-4 py-2 rounded-lg text-sm font-medium text-muted-foreground hover:text-foreground hover:bg-secondary/50 border border-border/30 transition-all duration-200"
                            @click="handleCancelModal"
                        >
                            Cancel
                        </button>
                        <button
                            class="px-4 py-2 rounded-lg bg-primary text-primary-foreground text-sm font-semibold hover:bg-primary/90 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
                            :disabled="!formValid || kbStore.loading"
                            @click="handleSave"
                        >
                            <span
                                v-if="kbStore.loading"
                                class="flex items-center gap-2"
                            >
                                <Icon
                                    icon="lucide:loader-2"
                                    class="text-sm animate-spin"
                                />
                                Saving...
                            </span>
                            <span v-else>
                                {{ editingKb ? "Save Changes" : "Create" }}
                            </span>
                        </button>
                    </div>
                </div>
            </div>
        </Teleport>
    </div>
</template>
