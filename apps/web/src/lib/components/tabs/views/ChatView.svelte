<script lang="ts">
	import type { Tab } from "$lib/tabs/types";
	import { windowShellStore } from "$lib/stores/window-shell.svelte";
	import ChatInput from "$lib/components/ChatInput.svelte";
	import MediaLightbox from "$lib/components/MediaLightbox.svelte";
	import {
		getSelectedModel,
		getDefaultModel,
		getModels,
		setSelectedModel,
		initializeSelectedModel,
		getInitializationPromise,
	} from "$lib/stores/models.svelte";
	import Markdown from "$lib/components/Markdown.svelte";
	import Bloub from "$lib/bloub/Bloub.svelte";
	import { EXPRESSIONS } from "$lib/bloub/bot/expressions";
	import { DEFAULT_SHAPE, SHAPES } from "$lib/bloub/bot/skins";
	import { getRandomThinkingLabel } from "$lib/utils/thinkingLabels";

	// A poke morphs the eyes to one random expression — and, roughly one poke
	// in five, the body to one random shape, so the circle stays the norm.
	// Both hold while the pointer stays and settle back to resting one second
	// after it leaves. Re-entering re-rolls.
	let interviewExpression = $state<string | null>(null);
	let interviewShape = $state<string | null>(null);
	let interviewHoverTimer: ReturnType<typeof setTimeout> | undefined;
	function pokeCompanion() {
		rouseCompanion();
		const others = EXPRESSIONS.filter(
			(e) => e.id !== "neutre" && e.id !== interviewExpression,
		);
		interviewExpression =
			others[Math.floor(Math.random() * others.length)].id;
		const shapes = SHAPES.filter(
			(s) => s.id !== DEFAULT_SHAPE && s.id !== interviewShape,
		);
		interviewShape =
			Math.random() < 0.2
				? shapes[Math.floor(Math.random() * shapes.length)].id
				: null;
		clearTimeout(interviewHoverTimer);
	}
	function settleCompanion() {
		clearTimeout(interviewHoverTimer);
		interviewHoverTimer = setTimeout(() => {
			interviewExpression = null;
			interviewShape = null;
		}, 1000);
	}

	// After a quiet stretch the bot dozes off instead of blinking at an empty
	// room forever. Anything happening — a hover, a send, the model speaking —
	// rouses it and re-arms the timer.
	const COMPANION_DOZE_MS = 90_000;
	let interviewAsleep = $state(false);
	let interviewSleepTimer: ReturnType<typeof setTimeout> | undefined;
	// While the bot thinks, one rotating gerund rides beside it — the bot's
	// three-dot morph is already the ellipsis, so the word comes bare.
	let interviewWord = $state("");
	function rouseCompanion() {
		interviewAsleep = false;
		clearTimeout(interviewSleepTimer);
		interviewSleepTimer = setTimeout(
			() => (interviewAsleep = true),
			COMPANION_DOZE_MS,
		);
	}
	import StoppedNotice from "$lib/components/StoppedNotice.svelte";
	import Icon from "$lib/components/Icon.svelte";
	import SelectionPopover from "$lib/components/SelectionPopover.svelte";
	import ContextIndicator from "$lib/components/ContextIndicator.svelte";

	// ── the narrative interview ────────────────────────────────────────────
	// One fixed chat (seeded at boot; the server forces interview mode by this
	// id — see chat_handler). Mirrors narrative_draft::INTERVIEW_CHAT_ID.
	const INTERVIEW_CHAT_ID = "chat_narrative_interview";
	const INTERVIEW_OPENING =
		"# The story of your life\n\n" +
		"Your server keeps the record of your life \u2014 where you go, what you " +
		"say, how you sleep. But the record can't say what any of it meant. " +
		"That part is yours to tell.\n\n" +
		"People understand predominantly through stories. They're accessible, " +
		"they carry context, and they can mix facts and feelings in a way that " +
		"captures the human experience. The goal here is to write yours, so " +
		"your server can make better sense of your data \u2014 not by inferring or " +
		"guessing, but by giving structure to your history: your past, goals, " +
		"ambitions, relationships, places, temperaments. Everything is a lot, " +
		"so we'll take it a piece at a time.\n\n" +
		"What you say here stays on your server. The model conducting this is " +
		"sent your words under a no-retention agreement and keeps nothing.\n\n" +
		"We start with the chapters of your life \u2014 five to ten of them, rough " +
		"names and rough years. One person's might run:\n\n" +
		"| Chapter | Years |\n" +
		"|---|---|\n" +
		"| Childhood travels | 1997 \u2013 2003 |\n" +
		"| Minnesota lower school | 2003 \u2013 2009 |\n" +
		"| Wisconsin | 2009 \u2013 2016 |\n" +
		"| College | 2016 \u2013 2020 |\n" +
		"| Locked in DC | 2020 \u2013 2021 |\n" +
		"| Vanderbilt & Atmos | 2021 \u2013 2023 |\n" +
		"| USDP | 2023 \u2013 2025 |\n" +
		"| Virtues | 2025 \u2013 now |\n\n" +
		"The same chapters, drawn on the one wire a life is:";

	/** The lifeline plate renders between the two parts (see the message
	 *  template); the ask comes after the person has seen the shape. */
	const INTERVIEW_OPENING_ASK =
		"Yours will look nothing like these. Rough names and rough years are " +
		"enough \u2014 what would your chapters be?";

	/** The narrative interview opens ALREADY SPEAKING: an authored first line,
	 *  shown free (never persisted, no model call). The interview prompt knows
	 *  this opening was delivered and picks up from the reply.
	 *
	 *  Called from BOTH load paths — the tab-change effect and onMount. It
	 *  lived inline in the first one only, so switching to an open interview
	 *  tab greeted you and deep-linking to /chat/chat_narrative_interview
	 *  (a fresh page load, a restored tab, the Home link) opened a blank room
	 *  with no explanation of what it was for.
	 *
	 *  PREPENDS rather than requiring an empty room: the opening is never
	 *  persisted, so a reload mid-interview would otherwise start the
	 *  transcript at the person's first reply with no trace of what was
	 *  asked. The backend rebuilds model context from its own store, so the
	 *  synthetic message rides the UI only. */
	function applyInterviewOpening(convId: string | null | undefined) {
		if (convId !== INTERVIEW_CHAT_ID) return;
		if (chat.messages[0]?.id === "interview-opening") return;
		chat.messages = [
			{
				id: "interview-opening",
				role: "assistant",
				// Two parts on purpose: the lifeline plate renders between
				// them, so the ask lands after the shape has been seen.
				parts: [
					{ type: "text", text: INTERVIEW_OPENING },
					{ type: "text", text: INTERVIEW_OPENING_ASK },
				],
			},
			...chat.messages,
		] as unknown as typeof chat.messages;
	}
	import { normalizeImage } from "$lib/multimodal/normalizeImage";
	import { CitationPanel } from "$lib/components/citations";
	import { buildCitationContextFromParts } from "$lib/citations";
	import type { Citation } from "$lib/types/Citation";
	import UserMessage from "$lib/components/UserMessage.svelte";
	import ThinkingBlock from "$lib/components/ThinkingBlock.svelte";
	import SubagentPanel from "$lib/components/SubagentPanel.svelte";
	import { onMount, onDestroy, tick } from "svelte";
	import { fade, fly } from "svelte/transition";
	import { cubicInOut } from "svelte/easing";
	import { chatSessions } from "$lib/stores/chatSessions.svelte";
	import { mobileLayout } from "$lib/stores/mobileLayout.svelte";
	import { chatInstances } from "$lib/stores/chatInstances.svelte";
	import { animateChatEdit } from "$lib/ai/aiPresence";
	import { pendingPrompt } from "$lib/stores/pendingPrompt.svelte";
	import { notebookStore } from "$lib/stores/notebook.svelte";
	import {
		updateChat,
		deleteChat,
		getChat,
		getChatUsage,
		getAssistantProfile,
		getProfile,
		setChatTitle,
		cancelChat,
	} from "$lib/api/client";
	import { contextMenu, type ContextMenuItem } from "$lib/stores/contextMenu.svelte";
	import type { Chat } from "@ai-sdk/svelte";
	// Active page editing imports
	import { editAllowListStore, type EditableResourceType } from "$lib/stores/editAllowList.svelte";
	import PageBindingInline from "$lib/components/chat/PageBindingInline.svelte";
	import ChapterLifeline from "$lib/components/chat/ChapterLifeline.svelte";
	import PageEditResult from "$lib/components/chat/PageEditResult.svelte";
	import EditDiffCard from "$lib/components/chat/EditDiffCard.svelte";
	import InterviewClosedCard from "$lib/components/chat/InterviewClosedCard.svelte";
	import { setupStateStore } from "$lib/stores/setupState.svelte";
	import CodeInterpreterCard from "$lib/components/chat/CodeInterpreterCard.svelte";
	import AppletProposalCard from '$lib/components/chat/AppletProposalCard.svelte';
	import CompactionCheckpoint from "$lib/components/chat/CompactionCheckpoint.svelte";
	import ContextViewPanel from "$lib/components/chat/ContextViewPanel.svelte";
	import { ChatError } from "$lib/components/chat";
	import { createYjsDocument } from "$lib/yjs";
	import type { EntityResult } from "$lib/components/RefPicker.svelte";
	import type { AgentModeId } from "$lib/config/agentModes";

	// Generate a random 16-char hex ID (matches backend format)
	function generateHex16(): string {
		const bytes = new Uint8Array(8);
		crypto.getRandomValues(bytes);
		return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('');
	}

	// Type for tool result parts in messages
	interface ToolResultPart {
		type: string;
		state?: string;
		toolCallId: string;
		output?: {
			page_id?: string;
			title?: string;
		};
	}

	// Props
	let { tab, active }: { tab: Tab; active: boolean } = $props();


	// Extract conversationId from tab route (format: /chat/chat_abc123 or / for new chat)
	// Returns the full chat ID including 'chat_' prefix, or undefined for new chat
	// Strips query params like ?view=context
	// svelte-ignore state_referenced_locally
	function extractConversationId(route: string): string | undefined {
		// Strip query params first
		const pathOnly = route.split('?')[0];
		if (pathOnly === '/' || pathOnly === '/chat') return undefined;
		// Route format: /chat/chat_abc123 → extract chat_abc123 (full ID)
		const match = pathOnly.match(/^\/chat\/(chat_[^/]+)$/);
		return match?.[1];
	}

	// Check if route has ?view=context query param
	function isContextViewRoute(route: string): boolean {
		return route.includes('?view=context');
	}

	// Derived: are we showing the context panel?
	const isContextView = $derived(isContextViewRoute(tab.route));

	// Check if route represents a new/unsaved chat
	function isNewChat(route: string): boolean {
		const pathOnly = route.split('?')[0];
		return pathOnly === '/' || pathOnly === '/chat';
	}

	// Temporary ("ghost") chat — opened via /?temporary=1. Never persisted to
	// history; nothing is written to the sidebar/session list. The request also
	// carries a `temporary` flag so the backend can skip storage.
	function isTemporaryRoute(route: string): boolean {
		return /[?&]temporary=1\b/.test(route);
	}
	// svelte-ignore state_referenced_locally
	let isGhost = $state(isTemporaryRoute(tab.route));

	// Capture initial conversationId from tab prop (intentionally captures initial value only)
	// svelte-ignore state_referenced_locally
	const initialConversationId = extractConversationId(tab.route);
	
	// UI state
	let conversationId = $state(initialConversationId || `chat_${generateHex16()}`);
	let messagesContainer: HTMLDivElement | null = $state(null);
	let scrollContainer: HTMLDivElement | null = $state(null);
	let enableTransitions = $state(false);
	// A NEW chat has nothing to load. Starting this at a blanket `true` meant
	// the composer painted docked at the bottom for one frame and then jumped
	// to its centered empty-state position once the tab effect flipped it —
	// with transitions still disabled, that jump was the launch flicker. The
	// route is known synchronously, so loading starts true only when there is
	// actually a conversation to fetch. Initial value only, on purpose — the
	// tab-change effect below owns every later transition.
	// svelte-ignore state_referenced_locally
	let isLoading = $state(!isNewChat(tab.route));
	let isAwaitingResponse = $state(false);
	// Track C: messages typed while the assistant is still streaming are queued
	// and sent automatically when the turn finishes (Cursor-style chips above the
	// composer). Local to the view — a tab drag-away mid-queue is an accepted edge.
	let queuedMessages = $state<string[]>([]);

	// Track D: highlight-to-reference. Select text in a message → comment bar →
	// stage a reference chip above the composer that scopes the next message.
	// Empty note = quote; typed note = quote + comment. Ephemeral. The in-text
	// mark uses the app's own --color-highlight token (one warm marker, not a
	// per-ref rainbow) — references are distinguished by being listed, not colored.
	type StagedRef = {
		id: string;
		messageId: string;
		text: string;
		range: Range;
	};
	type SelectionDraft = {
		text: string;
		messageId: string;
		rect: { top: number; left: number; bottom: number; width: number };
		range: Range;
	};
	let stagedRefs = $state<StagedRef[]>([]);
	let selectionDraft = $state<SelectionDraft | null>(null);

	function handleWindowMouseup(e: MouseEvent) {
		const sel = window.getSelection();
		const text = sel && !sel.isCollapsed ? sel.toString().trim() : "";
		if (text && sel && sel.rangeCount > 0) {
			const range = sel.getRangeAt(0);
			const node = range.commonAncestorContainer;
			const el = (node.nodeType === 1 ? node : node.parentElement) as HTMLElement | null;
			const wrapper = el?.closest(".message-wrapper") as HTMLElement | null;
			// Only chat messages; ignore selections inside the popover itself.
			if (!wrapper || el?.closest(".vref-bar")) return;
			const rect = range.getBoundingClientRect();
			selectionDraft = {
				text,
				messageId: wrapper.getAttribute("data-message-id") || "",
				rect: { top: rect.top, left: rect.left, bottom: rect.bottom, width: rect.width },
				range: range.cloneRange(),
			};
			return;
		}
		// Collapsed selection = a click → dismiss the popover if clicking outside it.
		if (selectionDraft && !(e.target as HTMLElement)?.closest(".vref-bar")) {
			selectionDraft = null;
		}
	}

	function addStagedRef() {
		if (!selectionDraft) return;
		const d = selectionDraft;
		stagedRefs = [
			...stagedRefs,
			{
				id: crypto?.randomUUID?.() ?? `ref-${stagedRefs.length}-${d.text.length}`,
				messageId: d.messageId,
				text: d.text,
				range: d.range,
			},
		];
		selectionDraft = null;
		window.getSelection()?.removeAllRanges();
		repaintHighlights();
	}

	function removeStagedRef(id: string) {
		stagedRefs = stagedRefs.filter((r) => r.id !== id);
		repaintHighlights();
	}

	function clearStagedRefs() {
		stagedRefs = [];
		repaintHighlights();
	}

	// Paint staged refs with the CSS Custom Highlight API under one name — no DOM
	// mutation, no reflow, themed via --color-highlight. Only committed refs are
	// painted; the pending selection keeps the browser's own native highlight so
	// it never double-marks (and Cmd+C keeps copying it).
	function repaintHighlights() {
		const cssAny = CSS as any;
		if (typeof CSS === "undefined" || !cssAny.highlights || typeof (window as any).Highlight === "undefined") return;
		const ranges = stagedRefs.map((r) => r.range);
		if (ranges.length === 0) {
			cssAny.highlights.delete("vref");
			return;
		}
		try {
			cssAny.highlights.set("vref", new (window as any).Highlight(...ranges));
		} catch {
			/* range invalidated by a re-render — drop silently */
		}
	}

	// Repaint whenever the staged set changes.
	$effect(() => {
		void stagedRefs;
		repaintHighlights();
	});

	function serializeRef(r: StagedRef): string {
		return `> ${r.text.replace(/\s*\n\s*/g, " ")}`;
	}

	// Track E1: multimodal attachments. Files are read to base64 data URLs (so they
	// round-trip to the provider and render on reload) and sent as AI SDK file parts.
	type Attachment = {
		id: string;
		mediaType: string;
		url: string; // data URL
		filename: string;
		size: number;
		kind: "image" | "pdf" | "audio" | "text";
		width?: number;
		height?: number;
	};

	function formatFileSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}
	let attachments = $state<Attachment[]>([]);
	let dragActive = $state(false);

	// Click an in-message image to open it in a shared-element lightbox.
	let lightbox = $state<{ src: string; alt: string; rect: DOMRect } | null>(null);
	function openLightbox(e: MouseEvent, src: string, alt: string) {
		const el = e.currentTarget as HTMLImageElement;
		lightbox = { src, alt, rect: el.getBoundingClientRect() };
	}
	// The catalog, read from the one store that loads it. This used to be a
	// SECOND fetch of /api/models with a `.catch(() => {})`, so the list the
	// attachment gate judged against and the list everything else used were
	// different objects that could disagree about what models exist. There is
	// no picker in the composer, so the only readers left are the capability
	// gate and the two recovery buttons below.
	const availableModels = $derived(getModels());

	// Text/code/doc extensions — MIME is unreliable for these, so check the name too.
	const TEXT_EXT =
		/\.(md|markdown|txt|text|csv|tsv|json|html?|xml|ya?ml|toml|ini|env|log|ts|tsx|js|jsx|mjs|cjs|py|rb|rs|go|java|c|h|cpp|cc|cs|php|swift|kt|sh|bash|zsh|sql|css|scss)$/i;

	function attachmentKind(file: File): Attachment["kind"] | null {
		const mt = (file.type || "").toLowerCase();
		if (mt.startsWith("image/")) return "image";
		if (mt === "application/pdf") return "pdf";
		if (mt.startsWith("audio/")) return "audio";
		if (mt.startsWith("text/") || mt === "application/json" || TEXT_EXT.test(file.name))
			return "text";
		return null;
	}

	function readAsDataURL(file: File): Promise<string> {
		return new Promise((resolve, reject) => {
			const r = new FileReader();
			r.onload = () => resolve(r.result as string);
			r.onerror = () => reject(r.error);
			r.readAsDataURL(file);
		});
	}

	function readAsText(file: File): Promise<string> {
		return new Promise((resolve, reject) => {
			const r = new FileReader();
			r.onload = () => resolve(r.result as string);
			r.onerror = () => reject(r.error);
			r.readAsText(file);
		});
	}

	function base64Utf8(s: string): string {
		const bytes = new TextEncoder().encode(s);
		let bin = "";
		for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i]);
		return btoa(bin);
	}

	async function addFiles(files: File[]) {
		const MAX = 100 * 1024 * 1024; // 100 MB, matches the media backend cap
		const MAX_TEXT = 100 * 1024; // inline-text cap (~25k tokens) before truncating
		for (const file of files) {
			const kind = attachmentKind(file);
			if (!kind || file.size > MAX) continue;
			try {
				let mediaType = file.type || "application/octet-stream";
				let url: string;
				let width: number | undefined;
				let height: number | undefined;

				if (kind === "image") {
					const norm = await normalizeImage(file);
					url = norm.dataUrl;
					mediaType = norm.mediaType;
					width = norm.width || undefined;
					height = norm.height || undefined;
				} else if (kind === "text") {
					let text = await readAsText(file);
					if (text.length > MAX_TEXT) text = text.slice(0, MAX_TEXT) + "\n…[truncated]";
					mediaType = "text/plain";
					url = `data:text/plain;base64,${base64Utf8(text)}`;
				} else {
					url = await readAsDataURL(file);
				}

				attachments = [
					...attachments,
					{
						id: crypto?.randomUUID?.() ?? `att-${attachments.length}-${file.size}`,
						mediaType,
						url,
						filename: file.name,
						size: file.size,
						kind,
						width,
						height,
					},
				];
			} catch {
				/* unreadable / undecodable file — skip */
			}
		}
	}

	function removeAttachment(id: string) {
		attachments = attachments.filter((a) => a.id !== id);
	}

	// Capability gate: does the active model support every attached modality? If not,
	// surface a switch to a model that does (or note none is available).
	const capabilityIssue = $derived.by(() => {
		if (attachments.length === 0) return null;
		const model =
			availableModels.find((m) => m.id === selectedModelValue?.id) ??
			selectedModelValue ??
			null;
		// No catalog means no capabilities to judge, not a model that lacks
		// them. Reading absent flags as "unsupported" told anyone whose
		// catalog failed to load that they could not attach an image, while
		// the box would have taken it happily.
		if (!model || availableModels.length === 0) return null;
		const needs = {
			image: attachments.some((a) => a.kind === "image"),
			pdf: attachments.some((a) => a.kind === "pdf"),
			audio: attachments.some((a) => a.kind === "audio"),
		};
		const lacks: string[] = [];
		if (needs.image && !model?.supportsVision) lacks.push("images");
		if (needs.pdf && !model?.supportsPdf) lacks.push("PDFs");
		if (needs.audio && !model?.supportsAudio) lacks.push("audio");
		if (lacks.length === 0) return null;
		const candidate =
			availableModels.find(
				(m) =>
					(!needs.image || m.supportsVision) &&
					(!needs.pdf || m.supportsPdf) &&
					(!needs.audio || m.supportsAudio),
			) ?? null;
		return { lacks, modelName: model?.displayName ?? "This model", candidate };
	});

	function switchToCapableModel() {
		const candidate = capabilityIssue?.candidate;
		if (candidate) selectedModelValue = candidate;
	}

	// Runtime recovery: when a picked model errors (unsupported tools, context
	// overflow, a gateway quirk), let the user drop to the Recommended model and
	// re-run in one click — a plain retry would just re-hit the same model.
	const recommendedFallback = $derived.by(() => {
		const rec = getDefaultModel();
		const currentId = selectedModelValue?.id ?? getDefaultModel()?.id;
		// Only worth offering when we'd actually change models.
		return rec && rec.id !== currentId ? rec : null;
	});

	function switchToRecommendedAndRetry() {
		const rec = getDefaultModel();
		if (rec) {
			selectedModelValue = rec;
			setSelectedModel(rec);
		}
		chat.regenerate();
	}
	let loadedMessages = $state<any[]>([]);

	// Track tab route to reset state when switching conversations
	// svelte-ignore state_referenced_locally
	let previousTabRoute = $state<string>(tab.route);
	let preferredName = $state<string | undefined>(undefined);
	let onboardingStatus = $state<string>('active');
	let onboardingStarted = false; // guard to prevent double-trigger

	// AbortController for cancelling in-flight requests on tab switch
	let tabSwitchAbortController: AbortController | null = null;

	// UI preferences from assistant profile
	let uiPreferences = $state<{
		contextIndicator?: {
			alwaysVisible?: boolean;
			showThreshold?: number;
		};
	}>({});

	// Keep a map of message metadata (agentId, provider, etc.) for rendering
	let messageMetadata = $state<
		Map<string, { agentId?: string; provider?: string; stopped?: boolean }>
	>(new Map());

	// Citation panel state
	let citationPanelOpen = $state(false);
	let selectedCitation = $state<Citation | null>(null);

	// The Notebook (room) this chat lives in — at most one. Its id is sent with
	// each message (drives the agent's active-space context + server-side
	// binding). Read-only here now: the picker that used to set it from this
	// view is gone, so the binding is seeded from the session row and changed
	// where the filing happens — in the notebook.
	let chatNotebookId = $state<string | null>(null);
	// Which conversation chatNotebookId was seeded for. Seeding happens ONCE per
	// conversation (when its session row is available, or once the session list
	// has finished loading and confirms there's no row yet) so a later session
	// refresh can never clobber a room the user just picked locally.
	let seededNotebookFor = $state<string | null>(null);

	$effect(() => {
		const id = conversationId;
		if (seededNotebookFor === id) return;
		const session = chatSessions.sessions.find((s) => s.conversation_id === id);
		if (session) {
			chatNotebookId = session.notebook_id ?? null;
			seededNotebookFor = id;
		} else if (!chatSessions.isLoading) {
			// Sessions are loaded and this chat has no row yet (brand-new, not yet
			// persisted) — start unfiled; the create path binds it from the first
			// message's notebookId.
			chatNotebookId = null;
			seededNotebookFor = id;
		}
	});

	// Open citation panel with selected citation
	function openCitationPanel(citation: Citation) {
		selectedCitation = citation;
		citationPanelOpen = true;
	}

	// Close citation panel
	function closeCitationPanel() {
		citationPanelOpen = false;
		selectedCitation = null;
	}

	// Helper to get the first bound page from the edit allow list
	function getBoundPage() {
		return editAllowListStore.items.find((i) => i.type === 'page');
	}

	function handlePageClear() {
		const pages = editAllowListStore.items.filter((i) => i.type === 'page');
		for (const page of pages) {
			editAllowListStore.remove('page', page.id);
		}
	}

	function handleRemoveItem(type: string, id: string) {
		editAllowListStore.remove(type as EditableResourceType, id);
	}

	function handlePageSelect(pageId: string, pageTitle: string) {
		// Create Yjs document for the page and bind
		// NOTE: No auto-open - user can open the page manually if they want to see it
		handlePageClear();
		const yjsDoc = createYjsDocument(pageId);
		editAllowListStore.addPage(pageId, pageTitle, yjsDoc);
	}

	/**
	 * Handle permission allow for AI edit.
	 * Adds permission then regenerates the AI's last response (which had permission_needed).
	 * regenerate() removes that assistant message and re-requests — no duplicate user messages.
	 */
	async function handlePermissionAllow(entityId: string, entityType: string, title: string) {
		// Add to allow list (await ensures backend has the permission before retry)
		if (entityType === 'page') {
			const yjsDoc = createYjsDocument(entityId);
			await editAllowListStore.addPage(entityId, title, yjsDoc);
		} else {
			// folder / action / wiki_entry — no Yjs doc, granted generically by (type, id)
			await editAllowListStore.add({
				type: entityType as EditableResourceType,
				id: entityId,
				title
			});
		}

		// Regenerate = remove last assistant message + re-request
		if (chat.status === 'ready') {
			try {
				await chat.regenerate();
			} catch (error) {
				console.error('[ChatView] Failed to regenerate after permission grant:', error);
			}
		}
	}

	/**
	 * Handle permission deny for AI edit
	 */
	function handlePermissionDeny() {
		// User denied permission - no action needed
		// The tool result already shows the permission was needed
	}



	function handleSelectEntities(entities: EntityResult[]) {
		// Add each entity to the edit allow list
		for (const entity of entities) {
			// Map entity_type to our EditableResourceType
			const type = entity.entity_type === 'page' ? 'page' :
			             entity.entity_type === 'folder' ? 'folder' : 'page';

			// For pages, create Yjs document for real-time sync
			if (entity.entity_type === 'page') {
				const yjsDoc = createYjsDocument(entity.id);
				editAllowListStore.addPage(entity.id, entity.name, yjsDoc);
			} else {
				editAllowListStore.add({
					type: type as 'page' | 'folder' | 'wiki_entry',
					id: entity.id,
					title: entity.name
				});
			}
		}
	}

	// Track tool calls that were already complete when we mounted (loaded from history)
	// Only auto-open pages created AFTER mount (during streaming)
	let initialCompletedToolCalls: Set<string> | null = null;
	let initialLoadComplete = false;

	/**
	 * Handle create_page tool result - auto-open the new page
	 * Called from $effect when create_page completes during streaming
	 */
	function handlePageCreated(pageId: string, title: string) {
		// Auto-bind and open the newly created page in split view
		handlePageClear();
		// Don't create Yjs doc here — PageContent will create one when the tab mounts.
		// Creating a second doc causes two WebSocket connections to the same room,
		// which races with the server's Y.Text initialization.
		editAllowListStore.addPage(pageId, title);

		// Open the page BESIDE the chat (Category A) — never navigate the chat in place.
		windowShellStore.openRouteBeside(`/page/${pageId}`);
	}

	// Effect to handle create_page side effects (auto-open new pages)
	// Only triggers for pages created during this session, not when reopening old chats
	$effect(() => {
		if (!chat?.messages) return;

		// Don't auto-open during initial load - wait until loading is complete
		if (isLoading) return;

		// First run after load: capture already-completed tool calls (loaded from history)
		if (initialCompletedToolCalls === null) {
			initialCompletedToolCalls = new Set();
			for (const message of chat.messages) {
				if (message.role !== 'assistant') continue;
				for (const part of message.parts as ToolResultPart[]) {
					if (part.type === 'tool-create_page' && part.state === 'output-available') {
						initialCompletedToolCalls.add(part.toolCallId);
					}
				}
			}
			initialLoadComplete = true;
			return; // Don't auto-open on first run
		}

		// Only process new pages after initial load is complete
		if (!initialLoadComplete) return;

		// Subsequent runs: only auto-open for NEW completions (not loaded from history)
		for (const message of chat.messages) {
			if (message.role !== 'assistant') continue;

			for (const part of message.parts as ToolResultPart[]) {
				if (part.type === 'tool-create_page' && part.state === 'output-available') {
					const output = part.output;
					if (output?.page_id && !initialCompletedToolCalls.has(part.toolCallId)) {
						handlePageCreated(output.page_id, output.title ?? "");
						initialCompletedToolCalls.add(part.toolCallId); // Mark as handled
					}
				}
			}
		}
	});

	// (The interview's write_it_up auto-open lives in chatInstances.onData —
	// the backend sends a transient data-narrative-document part, because tool
	// parts land in the messages array mutably where no effect observes them.)



	// Effect to drive the AI presence animation when a chat `edit_page` lands.
	// Mirrors the create_page effect: seed historical edits on the first settled
	// run (so we don't replay them), then animate only new ones, deduped by
	// edit_id. The animation is a no-op if the page isn't open in a pane.
	let editAnimSeeded = false;
	const animatedEditIds = new Set<string>();
	$effect(() => {
		if (!chat?.messages || isLoading) return;

		const collectNew = (animate: boolean) => {
			for (const message of chat.messages) {
				if (message.role !== "assistant") continue;
				for (const part of message.parts as ToolResultPart[]) {
					if (part.type !== "tool-edit_page" || part.state !== "output-available")
						continue;
					const output = part.output as any;
					const edit = output?.edit;
					if (!edit?.edit_id || animatedEditIds.has(edit.edit_id)) continue;
					animatedEditIds.add(edit.edit_id);
					if (animate && output?.applied) {
						animateChatEdit(edit.page_id, edit.replace || "");
					}
				}
			}
		};

		// First settled run: seed history without animating.
		if (!editAnimSeeded) {
			collectNew(false);
			editAnimSeeded = true;
			return;
		}
		collectNew(true);
	});

	// Context usage state
	interface ContextUsageState {
		percentage: number;
		tokens: number;
		window: number;
		status: "healthy" | "warning" | "critical";
	}
	let contextUsage = $state<ContextUsageState | undefined>(undefined);

	// Fetch context usage from API
	async function refreshContextUsage() {
		if (!conversationId || isNewChat(tab.route)) return;

		try {
			const data = await getChatUsage<{
				usage_percentage: number;
				total_tokens: number;
				context_window: number;
			}>(conversationId);
			const status: "healthy" | "warning" | "critical" =
				data.usage_percentage >= 85
					? "critical"
					: data.usage_percentage >= 70
						? "warning"
						: "healthy";

			contextUsage = {
				percentage: data.usage_percentage,
				tokens: data.total_tokens,
				window: data.context_window,
				status,
			};

		} catch {
			// Non-critical, continue without usage data
		}
	}


	// Handle context indicator click - open context tab in split view
	function handleContextClick() {
		const currentPane = windowShellStore.findTabPane(tab.id);
		windowShellStore.openChatContext(conversationId, currentPane);
	}

	// Handle compaction completion from ContextViewPanel - refresh messages
	async function handleCompacted() {
		if (!conversationId) return;
		try {
			const data = await getChat<{ messages?: any[] }>(conversationId);
			loadedMessages = data.messages || [];
			chat.messages = deduplicateMessages(loadedMessages).map((msg: any) => ({
				id: msg.id,
				role: msg.role as "user" | "assistant" | "checkpoint",
				parts: convertMessageToParts(msg),
			})) as unknown as typeof chat.messages;
		} catch {
			// Non-critical refresh — leave the current messages in place on failure.
		}
	}

	// Helper function to convert database messages to Chat parts
	function convertMessageToParts(msg: any) {
		// Carry agent/provider + the user-stopped flag (subject='cancelled') so the
		// "Stopped" notice survives a reload.
		const stopped = msg.subject === "cancelled";
		if (msg.agentId || msg.provider || stopped) {
			messageMetadata.set(msg.id, {
				agentId: msg.agentId,
				provider: msg.provider,
				stopped,
			});
		}

		// If message already has parts array (e.g., checkpoint messages), use it directly
		if (msg.parts && Array.isArray(msg.parts) && msg.parts.length > 0) {
			return msg.parts;
		}

		// Otherwise, construct parts from individual fields (legacy format)
		const parts: any[] = [];

		if (msg.reasoning) {
			parts.push({
				type: "reasoning" as const,
				text: msg.reasoning,
				state: "done" as const,
			});
		}

		if (msg.content) {
			parts.push({
				type: "text" as const,
				text: msg.content,
			});
		}

		if (msg.tool_calls && Array.isArray(msg.tool_calls)) {
			for (const toolCall of msg.tool_calls) {
				parts.push({
					type: `tool-${toolCall.tool_name}` as const,
					toolCallId:
						toolCall.tool_call_id ||
						`${msg.id}_${toolCall.tool_name}_${Date.now()}`,
					toolName: toolCall.tool_name,
					input: toolCall.arguments,
					state: "output-available" as const,
					output: toolCall.result,
				});
			}
		}

		return parts;
	}

	// Helper function to deduplicate messages by ID
	function deduplicateMessages(messages: any[]): any[] {
		if (!messages || messages.length === 0) return [];
		const seen = new Set<string>();
		return messages.filter((msg) => {
			if (seen.has(msg.id)) {
				return false;
			}
			seen.add(msg.id);
			return true;
		});
	}

	// Chat instance - fetched from shared store to survive remounts
	let chat = $state<Chat>(null!);
	let currentChatConversationId = $state<string | null>(null);

	// The id that goes on the wire — ONLY when the person moved the picker off
	// what we prefilled for them. What the picker is showing is not a choice.
	//
	// The box resolves every unpinned turn from the slot (see
	// `api/model_choice.rs`), which is what lets a slot swap reach a
	// conversation already in progress, and what keeps a client that failed to
	// load the catalog able to chat at all. This function returned `""` in
	// that case, and the box rejected it with a 400 listing 244 allowed ids
	// and never the one it refused.
	function getCurrentModel(): string | undefined {
		const id = selectedModelValue?.id;
		return id && id !== prefilledModelId ? id : undefined;
	}

	// Getter for the chat's Notebook (room) ID — sent with each message so the agent
	// gets the active-space context block and the server keeps the binding fresh.
	function getNotebookId(): string | null {
		return chatNotebookId;
	}

	// Get or create chat instance for the current conversationId
	function ensureChatInstance() {
		if (currentChatConversationId !== conversationId) {
			// Release old instance if we had one
			if (currentChatConversationId) {
				chatInstances.release(currentChatConversationId);
			}
			// Get or create new instance with model, space, active page, persona, and agent mode getters
			chat = chatInstances.getOrCreate({
				conversationId,
				getModel: getCurrentModel,
				getNotebookId,
				getActivePageContext: () => {
					const page = getBoundPage();
					if (!page) return null;

					// Include the current Yjs content so AI edits match what's in the editor
					const content = page.yjsDoc?.ytext.toString() || '';

					return {
						page_id: page.id,
						page_title: page.title || undefined,
						content: content
					};
				},
				getPersona: () => selectedPersona,
				getAgentMode: () => selectedAgentMode,
				getChatMode: () => chatMode,
				getTemporary: () => isGhost,
			});
			currentChatConversationId = conversationId;
		}
	}

	// Initialize chat on first render
	$effect(() => {
		ensureChatInstance();
	});

	// Watch for tab.route changes to reset state when switching conversations
	$effect(() => {
		const currentTabRoute = tab.route;
		const currentTabConversationId = extractConversationId(currentTabRoute);

		// If the tab's route changed, reset the chat state
		if (currentTabRoute !== previousTabRoute) {
			// IMPORTANT: Skip reset if we're just transitioning from 'new' to a real chat ID
			// This happens after the first message is sent - we're not switching conversations,
			// just updating the tab's route to reflect the persisted chat
			const isSameConversation =
				isNewChat(previousTabRoute) &&
				currentTabConversationId === conversationId;

			if (isSameConversation) {
				// Just update the tracking variable, don't reset state
				previousTabRoute = currentTabRoute;
				return;
			}

			// Cancel any in-flight requests from previous tab
			tabSwitchAbortController?.abort();
			tabSwitchAbortController = new AbortController();
			const signal = tabSwitchAbortController.signal;

			previousTabRoute = currentTabRoute;

			// Generate new conversationId for new chats, or use the extracted conversationId
			const newConversationId =
				currentTabConversationId || `chat_${generateHex16()}`;
			conversationId = newConversationId;

			// Reset chat state
			chat.messages = [];
			loadedMessages = [];
			messageMetadata = new Map();
			contextUsage = undefined;
			titleGenerated = false;
			isAwaitingResponse = false;
			// Reset page create tracking (for auto-open)
			initialCompletedToolCalls = null;
			initialLoadComplete = false;
			// NOTE: We no longer unbind the active page when switching chats.
			// Binding is now additive/persistent to the chat session context.
			// handlePageClear();

			// Load conversation if switching to an existing one
			if (currentTabConversationId && !isNewChat(currentTabRoute)) {
				isLoading = true;
				(async () => {
					try {
						// Raw fetch (not getChat): this load carries an AbortSignal so
						// switching tabs mid-load cancels it. The client wrapper has no
						// signal channel, so this site stays on fetch by design.
						const response = await fetch(
							`/api/chats/${currentTabConversationId}`,
							{ signal },
						);
						if (signal.aborted) return; // Check if we were aborted
						if (response.ok) {
							const data = await response.json();
							if (signal.aborted) return; // Check again after parsing
							loadedMessages = data.messages || [];
							chat.messages = deduplicateMessages(
								loadedMessages,
							).map((msg: any) => ({
								id: msg.id,
								role: msg.role as "user" | "assistant" | "checkpoint",
								parts: convertMessageToParts(msg),
							})) as unknown as typeof chat.messages;
							applyInterviewOpening(currentTabConversationId);
							// The picker is deliberately left alone on a tab
							// switch. It used to be re-seeded from the model
							// that last answered THIS conversation, which is
							// neither the person's choice nor what the next
							// turn will use — a chat is not pinned to a model.
							// Leaving it holds their pick across chats, and an
							// unpicked picker keeps showing the slot default.
							await Promise.all([
								refreshContextUsage(),
								editAllowListStore.init(currentTabConversationId),
							]);
						}
					} catch (error) {
						// Ignore abort errors - they're expected when switching tabs
						if (
							error instanceof Error &&
							error.name === "AbortError"
						)
							return;
						console.error(
							"[ChatView] Error loading conversation on tab change:",
							error,
						);
					} finally {
						if (!signal.aborted) {
							isLoading = false;
							// Scroll to bottom after loading existing chat
							setTimeout(() => scrollToBottom("instant"), 10);
						}
					}
				})();
			} else {
				// New chat - set chatId so permissions can sync when granted
				editAllowListStore.setChatId(newConversationId);
				isLoading = false;
			}
		}
	});

	// Load conversation data on mount
	onMount(() => {
		// (The notebook list used to be fetched here for the breadcrumb's name
		// and accent. The app layout already loads it, and nothing in this view
		// renders a notebook's name any more.)

		// Claim any prompt handed off from Home / ⌘K / "Ask this notebook"
		// (consume-once, synchronously — so only this freshly-opened chat sends it).
		const initialPrompt = pendingPrompt.take();
		// If the ask came from a notebook, bind this new chat to it before the
		// first message so the create path files it + grounds retrieval there.
		const seededNotebook = pendingPrompt.takeNotebook();
		if (seededNotebook) {
			chatNotebookId = seededNotebook;
			seededNotebookFor = conversationId;
		}
		(async () => {
			// Stage 1: Models must load first (other code depends on model list)
			await getInitializationPromise();

			// Stage 2: Profile fetches + conversation load in parallel (independent)
			const tabConversationId = extractConversationId(tab.route);

			let profileDefaultModelId: string | undefined;
			let profileDefaultPersona: string | undefined;

			const profilePromise = (async () => {
				try {
					const profile = await getAssistantProfile<{
						ui_preferences?: Record<string, unknown>;
						chat_model_id?: string;
						persona?: string;
					}>();
					if (profile.ui_preferences) {
						uiPreferences = profile.ui_preferences;
					}
					profileDefaultModelId = profile.chat_model_id;
					profileDefaultPersona = profile.persona;
				} catch (error) {
					console.error("Failed to load assistant profile:", error);
				}
			})();

			const namePromise = (async () => {
				try {
					const profile = await getProfile();
					preferredName = profile.preferred_name ?? undefined;
					onboardingStatus = profile.onboarding_status || 'active';
				} catch {
					// Non-critical, continue without preferred name
				}
			})();

			const conversationPromise = tabConversationId ? (async () => {
				try {
					const data = await getChat<{
						messages?: any[];
						conversation?: { model?: string };
					}>(tabConversationId);
					loadedMessages = data.messages || [];
					chat.messages = deduplicateMessages(loadedMessages).map(
						(msg: any) => ({
							id: msg.id,
							role: msg.role as "user" | "assistant" | "checkpoint",
							parts: convertMessageToParts(msg),
						}),
					) as unknown as typeof chat.messages;
				} catch (error) {
					console.error("[ChatView] Error loading conversation:", error);
				}
			})() : null;

			await Promise.all([profilePromise, namePromise, conversationPromise]);

			// After the load, not inside it: a failed fetch must still leave
			// the interview speaking rather than showing a blank room.
			applyInterviewOpening(tabConversationId);

			// What the picker SHOWS, for every chat old or new: the owner's
			// standing preference, else the Virtues default. Deliberately not
			// the model that last answered this conversation — a chat is not
			// pinned to the model it opened with, so showing the last one
			// would name a model the next turn may not use.
			prefillModelDisplay(profileDefaultModelId);

			// Stage 3: Post-load tasks (depend on conversation being loaded)
			if (tabConversationId) {
				await Promise.all([
					refreshContextUsage(),
					editAllowListStore.init(tabConversationId),
				]);
			} else {
				// New chat - set defaults from profile
				editAllowListStore.setChatId(conversationId);
				if (profileDefaultPersona) {
					selectedPersona = profileDefaultPersona;
				}
			}

			isLoading = false;
			setTimeout(() => {
				scrollToBottom("instant");
				enableTransitions = true;
			}, 50);

			// Auto-send the handed-off prompt on a brand-new chat. handleChatSubmit
			// queues internally if the instance isn't "ready" yet, so this is safe.
			if (initialPrompt && isNewChat(tab.route)) {
				handleChatSubmit(initialPrompt);
			}

			// Auto-start onboarding for new users with no messages
			// DISABLED for demo — onboarding was repeating the same message
			// if (onboardingStatus === 'new' && loadedMessages.length === 0) {
			// 	setTimeout(() => startOnboarding(), 100);
			// }
		})();

		return () => {
			if (inactivityTimer) clearTimeout(inactivityTimer);
			if (refreshDataTimeout) clearTimeout(refreshDataTimeout);
			tabSwitchAbortController?.abort();
		};
	});

	// Release chat instance on destroy
	onDestroy(() => {
		if (currentChatConversationId) {
			chatInstances.release(currentChatConversationId);
		}
		// Clear any staged highlight ranges from the global CSS highlight registry.
		stagedRefs = [];
		repaintHighlights();
	});

	// Derive thinking state from chat status
	const isThinking = $derived.by(() => {
		const status = chat?.status;
		return status === "submitted" || status === "streaming";
	});

	// Deduplicated messages for rendering
	const uniqueMessages = $derived(chat?.messages ? deduplicateMessages(chat.messages) : []);

	// Get the last assistant message
	const lastAssistantMessage = $derived.by(() => {
		for (let i = uniqueMessages.length - 1; i >= 0; i--) {
			if (uniqueMessages[i].role === "assistant") {
				return uniqueMessages[i];
			}
		}
		return null;
	});

	// Whether the last assistant message has any visible content yet
	// (text, reasoning, or tool calls). Used to keep the optimistic thinking
	// indicator showing until real content takes over.
	const lastAssistantHasVisibleContent = $derived.by(() => {
		if (!lastAssistantMessage) return false;
		return lastAssistantMessage.parts.some((p: any) =>
			(p.type === 'text' && p.text) ||
			(p.type === 'reasoning' && p.text) ||
			p.type?.startsWith('tool-')
		);
	});

	// Track thinking duration
	let thinkingStartTime = $state<number | null>(null);
	let thinkingDuration = $state(0);

	$effect(() => {
		if (isThinking && !thinkingStartTime) {
			thinkingStartTime = Date.now();
		} else if (!isThinking && thinkingStartTime) {
			thinkingDuration = (Date.now() - thinkingStartTime) / 1000;
			thinkingStartTime = null;
		}
	});

	// Local input state
	let input = $state("");
	let inputFocused = $state(false);

	// Auto-focus chat input when new chat tab becomes active
	$effect(() => {
		if (active && isEmpty && !isLoading) {
			// Small delay to ensure DOM is ready
			setTimeout(() => {
				inputFocused = true;
			}, 50);
		}
	});

	// Title generation state
	let titleGenerated = $state(false);
	let inactivityTimer: ReturnType<typeof setTimeout> | null = null;
	let refreshDataTimeout: ReturnType<typeof setTimeout> | null = null;

	// Model selection state - use bindable for ChatInput toolbar
	let selectedModelValue = $state<
		import("$lib/config/models").ModelOption | undefined
	>(undefined);
	/** The id we put in the picker ourselves, so `getCurrentModel` can tell a
	 *  prefill apart from a choice. Picking this exact model back is the same
	 *  as not choosing: either way the box resolves the slot. */
	let prefilledModelId = $state<string | undefined>(undefined);

	/** Show what the next turn will use: the owner's pin, else the Virtues
	 *  default. Records what it showed; sends nothing. */
	function prefillModelDisplay(profileDefaultModelId?: string) {
		// Re-seed on every mount UNLESS a recovery button picked something this
		// session. The store's seeder only runs once per session, so changing
		// your pin in Settings used to leave the attachment capability gate
		// judging against the model you were pinned to before — warning that
		// you cannot send an image to a model that was no longer answering.
		const picked =
			selectedModelValue && selectedModelValue.id !== prefilledModelId;
		if (!picked) setSelectedModel(undefined);
		initializeSelectedModel(profileDefaultModelId);
		const shown = getSelectedModel() ?? getDefaultModel();
		if (!shown) return; // catalog not loaded — the box still knows
		selectedModelValue = shown;
		prefilledModelId = shown.id;
	}

	// Agent mode and persona selection state - used for tool filtering on backend
	let selectedAgentMode = $state<AgentModeId>('chat');
	let selectedPersona = $state<string>('default');

	// Retrieval scope. 'scoped' (grounded in a notebook's items only) still
	// exists on the wire and in the retriever — what's gone is the pill above
	// the composer that switched it, which was a permanent piece of chrome for
	// a setting almost nobody moved. Every chat is 'open': the whole graph,
	// with the notebook up-weighted when there is one. If scoped comes back it
	// belongs somewhere it can be explained, not as a two-state word.
	const chatMode = 'open' as const;

	// Sync selected model with store (only on initial load). Still a prefill,
	// not a choice — record it as one.
	$effect(() => {
		const storeModel = getSelectedModel();
		if (storeModel && !selectedModelValue) {
			selectedModelValue = storeModel;
			prefilledModelId = storeModel.id;
		}
	});

	// Safety timeout
	let thinkingTimeout: ReturnType<typeof setTimeout> | null = null;
	$effect(() => {
		if (isThinking) {
			thinkingTimeout = setTimeout(() => {
				if (chat.status === "error") {
					chat.clearError();
				} else if (
					chat.status === "streaming" ||
					chat.status === "submitted"
				) {
					if (chat.clearError) {
						chat.clearError();
					}
				}
			}, 300000); // 5 minutes to match backend streaming timeout

			return () => {
				if (thinkingTimeout) {
					clearTimeout(thinkingTimeout);
					thinkingTimeout = null;
				}
			};
		} else if (thinkingTimeout) {
			clearTimeout(thinkingTimeout);
			thinkingTimeout = null;
		}
	});

	// Derived state for layout mode
	// Also gate on isLoading to prevent flashing "new chat" while fetching an existing conversation
	let isEmpty = $derived(uniqueMessages.length === 0 && !isLoading);

	// The interview's close. Three witnesses, any one suffices: the tool
	// result in this session (the transient data part — see chatInstances),
	// a write_it_up part in the loaded transcript (a reload after the close),
	// or the box saying the document stands (the HTTP path wrote it, or the
	// transcript's tool part didn't survive). Once closed, the composer
	// retires: the drafter runs once, so a message typed here now would reach
	// nothing — the page is where corrections go.
	const interviewClosedPart = $derived.by(() => {
		if (currentChatConversationId !== INTERVIEW_CHAT_ID) return null;
		for (let i = uniqueMessages.length - 1; i >= 0; i--) {
			const m = uniqueMessages[i] as any;
			if (m.role !== "assistant") continue;
			for (const part of m.parts ?? []) {
				if (part.type === "tool-write_it_up" && part.state === "output-available" && part.output?.document_page_id) {
					return part.output as {
						document_page_id: string;
						document_already_existed?: boolean;
						chapters_written?: number;
						chapters_error?: string;
					};
				}
			}
		}
		return null;
	});
	const interviewClosed = $derived(
		currentChatConversationId === INTERVIEW_CHAT_ID &&
			(chatInstances.narrativeDocumentPageId !== null ||
				interviewClosedPart !== null ||
				setupStateStore.done("narrative_identity_ready")),
	);
	const interviewDocumentPageId = $derived(
		interviewClosedPart?.document_page_id ?? chatInstances.narrativeDocumentPageId,
	);

	// The companion's activity feed: a new message or a status change wakes it
	// and re-arms the doze timer.
	$effect(() => {
		if (currentChatConversationId !== INTERVIEW_CHAT_ID) return;
		void uniqueMessages.length;
		void chat.status;
		rouseCompanion();
		return () => clearTimeout(interviewSleepTimer);
	});

	$effect(() => {
		if (currentChatConversationId !== INTERVIEW_CHAT_ID) return;
		if (chat.status !== "submitted" && chat.status !== "streaming") return;
		interviewWord = getRandomThinkingLabel();
		const rotate = setInterval(() => {
			interviewWord = getRandomThinkingLabel();
		}, 4000);
		return () => clearInterval(rotate);
	});

	// The chat's title, from the persisted session so it stays in step with the
	// sidebar. It is no longer DRAWN here: a title fixed to the top-left of the
	// pane restated what the tab above it already said, and doubled as a rename
	// affordance nobody looked for. Renaming lives where the name is shown —
	// right-click the tab, or the sidebar row.
	const chatTitle = $derived(
		chatSessions.sessions.find((s) => s.conversation_id === conversationId)?.title ?? "",
	);

	// Adopt the stored title into the tab label.
	//
	// The route registry stamps a new chat tab "Chat" because at parse time the
	// title isn't known — it lives in the session list, which may not have
	// loaded yet. Openers that HAVE a title (the Desk, Home, chat history) pass
	// it along, so this only bites on the paths that don't: a deep link, a
	// restored tab, "open beside". The label then stayed "Chat" forever, which
	// was survivable while the pane also printed the title and is not now.
	//
	// Placeholders only — a label the user has renamed by hand must not be
	// overwritten by the server's copy.
	const PLACEHOLDER_LABELS = new Set(["Chat", "New Chat", "Temporary Chat"]);
	$effect(() => {
		if (!chatTitle || !tab) return;
		if (!PLACEHOLDER_LABELS.has(tab.label)) return;
		windowShellStore.updateTab(tab.id, { label: chatTitle });
	});

	// A real, saved chat the user can act on (not the empty new-chat state, not a ghost).
	const canManageChat = $derived(!isEmpty && !isGhost);

	async function deleteThisChat() {
		try {
			windowShellStore.closeTabsByRoute(`/chat/${conversationId}`);
			await deleteChat(conversationId);
			chatSessions.remove(conversationId);
			windowShellStore.invalidateViewCache("chat");
		} catch (e) {
			console.error("[ChatView] Failed to delete chat:", e);
		}
	}

	function openChatMenu(e: MouseEvent) {
		const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
		const pinned = !!windowShellStore.findTab((t) => t.id === tab.id)?.tab.pinned;
		const items: ContextMenuItem[] = [
			{
				id: "pin",
				label: pinned ? "Unpin tab" : "Pin tab",
				icon: pinned ? "ri:unpin-line" : "ri:pushpin-line",
				action: () => windowShellStore.togglePin(tab.id),
			},
			{
				id: "delete",
				label: "Delete chat",
				icon: "ri:delete-bin-line",
				variant: "destructive",
				dividerBefore: true,
				action: deleteThisChat,
			},
		];
		contextMenu.show(
			{ x: rect.right, y: rect.bottom },
			items,
			{
				anchor: { x: rect.left, y: rect.top, width: rect.width, height: rect.height },
				placement: "bottom-end",
			},
		);
	}

	// Generate title after first assistant response
	async function generateTitle() {
		if (titleGenerated || chat.messages.length < 2) return;
		// The interview keeps the name it was seeded with. Its transcript is
		// the most private text on the box, and a generated title puts a
		// summary of it in the sidebar — this chat had renamed itself after
		// the person's own childhood. The server refuses this too (the id
		// decides, never the client); this only saves the round trip.
		if (conversationId === INTERVIEW_CHAT_ID) {
			titleGenerated = true;
			return;
		}

		try {
			const data = await setChatTitle<{ title?: string }>({
				chatId: conversationId,
				messages: chat.messages.map((m) => ({
					role: m.role,
					content: m.parts.find((p) => p.type === "text")?.text || "",
				})),
			});

			// Only mark done once we actually have a title, so an ok-but-empty
			// response retries on the next turn instead of giving up silently.
			if (data.title) {
				titleGenerated = true;
				windowShellStore.updateTab(tab.id, { label: data.title });
				// Optimistically seed the shared session store so the header
				// breadcrumb (and any store-bound surface) updates immediately,
				// without waiting on the server-persist → refetch round-trip.
				chatSessions.applyTitle(conversationId, data.title);
			}
		} catch (error) {
			// Title generation is non-critical
		}
	}

	function scrollToBottom(behavior: ScrollBehavior = "smooth") {
		if (scrollContainer) {
			scrollContainer.scrollTo({
				top: scrollContainer.scrollHeight,
				behavior,
			});
		}
	}

	async function handleChatStop() {
		// Stop the client-side stream
		chat.stop();

		// Mark the in-flight assistant message as user-stopped so the "Stopped"
		// notice shows immediately (reload reads the persisted subject='cancelled').
		const stoppedId = lastAssistantMessage?.id;
		if (stoppedId) {
			const existing = messageMetadata.get(stoppedId) ?? {};
			messageMetadata.set(stoppedId, { ...existing, stopped: true });
			// $state(Map) doesn't track .set() — reassign so the chip re-renders live.
			messageMetadata = new Map(messageMetadata);
		}

		// Also notify the backend to cancel the agent loop
		try {
			await cancelChat(conversationId);
		} catch (e) {
			console.error('[ChatView] Failed to cancel chat:', e);
		}
	}

	async function startOnboarding() {
		if (onboardingStarted || !chat) return;
		onboardingStarted = true;

		// Generate a fresh conversationId if the current one is stale (from a deleted DB)
		if (!isNewChat(tab.route)) {
			conversationId = `chat_${generateHex16()}`;
		}

		ensureChatInstance();

		// Use the AI SDK flow: send a greeting to trigger the backend's onboarding detection.
		// The backend sees is_new_user + appends NEW_USER_PROMPT to the system prompt.
		// Tool calls (set_assistant_name, set_user_name) work natively through the SDK.
		try {
			await chat.sendMessage({ text: "👋" });
		} catch (error) {
			console.error('[ChatView] Onboarding error:', error);
		}

		onboardingStatus = 'active';

		// Update tab route to reflect the new chat
		const newRoute = `/chat/${conversationId}`;
		previousTabRoute = newRoute;
		windowShellStore.updateTab(tab.id, { route: newRoute });
		windowShellStore.invalidateViewCache('chat');
	}

	async function handleChatSubmit(value: string) {
		let messageToSend = value.trim();

		// Track D: prepend any staged highlight references as quoted context +
		// comments. Only present on a direct send (cleared before queue-drain).
		if (stagedRefs.length > 0) {
			const refsBlock = stagedRefs.map(serializeRef).join("\n\n");
			messageToSend = refsBlock + (messageToSend ? `\n\n${messageToSend}` : "");
			clearStagedRefs();
		}

		// Track E1: block sending if an attachment isn't supported by the active
		// model — the capability banner prompts a switch instead.
		if (capabilityIssue) return;

		if (!messageToSend && attachments.length === 0) return;

		if (chat.status !== "ready") {
			// Queue text; attachments stay staged and ride along when the drain
			// effect re-sends this once the current turn finishes.
			queuedMessages = [...queuedMessages, messageToSend];
			input = "";
			return;
		}
		input = "";

		// Capture + clear attachments as AI SDK file parts.
		const files = attachments.map((a) => ({
			type: "file" as const,
			mediaType: a.mediaType,
			url: a.url,
			filename: a.filename,
		}));
		attachments = [];

		// New turn → clear any leftover Deep Research panel from the previous turn.
		chatInstances.clearSubagents(conversationId);

		// Optimistic: show thinking indicator immediately (before network round-trip)
		isAwaitingResponse = true;
		await tick(); // Flush DOM so the indicator renders before the network call

		// Auto-scroll to bottom on submit
		scrollToBottom("smooth");

		try {
			// Sync permissions to backend BEFORE sending (so AI tool calls have them during streaming)
			// add_permission endpoint handles chat creation via INSERT OR IGNORE INTO chats.
			// Ghost chats are never created server-side, so skip this.
			if (!isGhost && isNewChat(tab.route) && editAllowListStore.hasItems) {
				await editAllowListStore.markChatCreated();
			}

			// No `text` key when nothing was typed: the SDK appends a text part
			// for any non-null text, and an EMPTY text block is a 400 from
			// Anthropic that then rode in this chat's history forever (every
			// later Claude turn failed; Grok did not care). An attachment on
			// its own is a complete message — no filler words on the person's
			// behalf.
			await chat.sendMessage(
				files.length > 0
					? messageToSend
						? { text: messageToSend, files }
						: { files }
					: { text: messageToSend },
			);

			if (chat.messages.length >= 2 && !isGhost && !titleGenerated) {
				await generateTitle();
				// Update tab route if it's a new chat
				if (isNewChat(tab.route)) {
					// Update previousTabRoute first to prevent the tab-switch effect
					// from treating this as a tab change and resetting state
					const newRoute = `/chat/${conversationId}`;
					previousTabRoute = newRoute;
					windowShellStore.updateTab(tab.id, {
						route: newRoute,
					});
					// Ensure chat is marked as created (may already be done above if hasItems)
					await editAllowListStore.markChatCreated();
					// Invalidate the Chats view cache so it refreshes with the new chat
					windowShellStore.invalidateViewCache('chat');
				}
				await chatSessions.refresh();
			}

			if (refreshDataTimeout) {
				clearTimeout(refreshDataTimeout as any);
			}
			refreshDataTimeout = setTimeout(() => {
				refreshContextUsage();
				refreshDataTimeout = null;
			}, 2000);
		} catch (error) {
			console.error("[handleChatSubmit] Error:", error);
			input = "";
		} finally {
			isAwaitingResponse = false;
		}
	}

	// Track C: drain the queue when the assistant goes idle.
	$effect(() => {
		if (
			chat.status === "ready" &&
			queuedMessages.length > 0 &&
			!isAwaitingResponse
		) {
			const [next, ...rest] = queuedMessages;
			queuedMessages = rest;
			handleChatSubmit(next);
		}
	});

	function removeQueued(index: number) {
		queuedMessages = queuedMessages.filter((_, i) => i !== index);
	}

	// Flip the current (empty) chat into a temporary/ghost chat, or back. Only
	// allowed before the first message — we can't retroactively un-persist a turn.
	function toggleGhost() {
		if (!isEmpty) return;
		isGhost = !isGhost;
	}

	// Publish this chat's state to the phone shell, whose top-right button is
	// modal: ghost toggle while the chat is empty (compose would be a no-op),
	// compose once a conversation exists. Only the active view speaks; the
	// cleanup keeps a stale claim from surviving a view swap.
	$effect(() => {
		if (!mobileLayout.isMobile || !active) return;
		mobileLayout.setChatChrome({ empty: isEmpty, ghost: isGhost, toggleGhost });
		return () => mobileLayout.setChatChrome(null);
	});
</script>

<svelte:window onmouseup={handleWindowMouseup} />

{#if selectionDraft}
	<SelectionPopover
		rect={selectionDraft.rect}
		onAdd={addStagedRef}
		onClose={() => (selectionDraft = null)}
	/>
{/if}

{#if lightbox}
	<MediaLightbox
		src={lightbox.src}
		alt={lightbox.alt}
		originRect={lightbox.rect}
		onClose={() => (lightbox = null)}
	/>
{/if}

{#if !chat}
	<!-- wait for chat to initialize -->
{:else if isContextView}
	<ContextViewPanel {conversationId} {active} onCompacted={handleCompacted} />
{:else}
	<div
		class="chat-root"
		role="presentation"
		ondragover={(e) => {
			if (e.dataTransfer?.types.includes("Files")) {
				e.preventDefault();
				dragActive = true;
			}
		}}
		ondragleave={(e) => {
			// Only clear when leaving the root, not when crossing child boundaries.
			if (!e.relatedTarget || !(e.currentTarget as HTMLElement).contains(e.relatedTarget as Node)) {
				dragActive = false;
			}
		}}
		ondrop={(e) => {
			e.preventDefault();
			dragActive = false;
			if (e.dataTransfer?.files?.length) addFiles(Array.from(e.dataTransfer.files));
		}}
	>
		{#snippet renderFilePart(part: any, compact = false)}
			{@const mt = part.mediaType || ""}
			{#if mt.startsWith("image/")}
				<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
				<img
					src={part.url}
					alt={part.filename || "image"}
					class="msg-image"
					class:compact-img={compact}
					onclick={(e) => openLightbox(e, part.url, part.filename || "image")}
				/>
			{:else if mt.startsWith("audio/")}
				<audio src={part.url} controls class="msg-audio"></audio>
			{:else}
				<a class="msg-file" href={part.url} download={part.filename || "file"}>
					<Icon icon={mt === "application/pdf" ? "ri:file-pdf-fill" : "ri:file-text-line"} width="16" />
					<span>{part.filename || "Document"}</span>
				</a>
			{/if}
		{/snippet}

		<div class="chat-container">
			<!-- Main chat area -->
			<div class="chat-area" class:ghost={isGhost}>
				<!-- Top-right chrome: temporary-chat toggle + live context ring -->
				<div class="chat-topbar-right">
					{#if !isGhost && contextUsage && extractConversationId(tab.route)}
						<ContextIndicator
							conversationId={extractConversationId(tab.route)!}
							usagePercentage={contextUsage.percentage}
							totalTokens={contextUsage.tokens}
							contextWindow={contextUsage.window}
							status={contextUsage.status}
							onclick={handleContextClick}
						/>
					{/if}
					<!-- On the phone the ghost toggle lives in the shell's top bar
					     (the modal top-right slot), not here. -->
					{#if (isEmpty || isGhost) && !mobileLayout.isMobile}
						<button
							type="button"
							class="ghost-toggle"
							class:active={isGhost}
							disabled={!isEmpty}
							onclick={toggleGhost}
							aria-pressed={isGhost}
							title={isGhost ? "Temporary chat — won't be saved" : "Start a temporary chat"}
						>
							<Icon icon="ri:ghost-line" width="16" />
						</button>
					{/if}
					{#if canManageChat}
						<button
							type="button"
							class="chat-menu-btn"
							onclick={openChatMenu}
							aria-haspopup="menu"
							aria-label="Chat options"
							title="Chat options"
						>
							<Icon icon="ri:more-2-fill" width="16" />
						</button>
					{/if}
				</div>
				<div class="page-container" class:is-empty={isEmpty}>
					<!-- Messages area -->
					<div
						bind:this={scrollContainer}
						class="flex-1 overflow-y-auto chat-layout"
						class:visible={!isEmpty}
					>
						<div class="messages-container">
							{#each uniqueMessages as message, messageIndex (message.id)}
								{@const isUserMessage = message.role === "user"}
								{@const exchangeIndex = isUserMessage
									? uniqueMessages
											.slice(0, messageIndex)
											.filter((m) => m.role === "user")
											.length
									: -1}
								<div
									class="flex {isUserMessage
										? 'justify-end'
										: 'justify-start'}"
									id={isUserMessage
										? `exchange-${exchangeIndex}`
										: undefined}
								>
									<div
										class="message-wrapper"
										class:user-has-attachment={isUserMessage &&
											message.parts.some((p: any) => p.type === "file")}
										data-message-id={message.id}
										data-role={message.role}
										data-agent-id={messageMetadata.get(
											message.id,
										)?.agentId || "general"}
										data-loading={message.role ===
											"assistant" &&
											!message.parts.some(
												(p: any) =>
													p.type === "text" && p.text,
											)}
									>
										{#if message.role === "checkpoint"}
											<!-- Compaction checkpoint message -->
											{@const checkpointPart = message.parts.find((p: any) => p.type === "checkpoint")}
											{#if checkpointPart}
												<CompactionCheckpoint
													version={(checkpointPart as any).version}
													messagesSummarized={(checkpointPart as any).messagesSummarized || (checkpointPart as any).messages_summarized}
													summary={(checkpointPart as any).summary}
													timestamp={(checkpointPart as any).timestamp}
												/>
											{/if}
										{:else if message.role === "assistant"}
											{@const citationContext =
												buildCitationContextFromParts(
													message.parts,
												)}
											{@const isLastMessage =
												message.id ===
												uniqueMessages[
													uniqueMessages.length - 1
												]?.id}
											{@const isStreaming =
												(chat.status === "streaming" || chat.status === "submitted") &&
												isLastMessage}
											{@const messageReasoningParts =
												message.parts.filter(
													(p: any) =>
														p.type === "reasoning",
												)}
											{@const messageToolParts =
												message.parts.filter((p: any) =>
													p.type.startsWith("tool-"),
												)}
											{@const messageReasoning =
												messageReasoningParts
													.map(
														(p: any) =>
															p.text || "",
													)
													.filter(Boolean)
													.join("\n")}
											{@const hasThinkingContent =
												messageReasoning ||
												messageToolParts.length > 0}

											{@const subagents =
												isLastMessage
													? chatInstances.getSubagents(
															conversationId,
														)
													: []}
											{#if subagents.length > 0}
												<SubagentPanel
													{subagents}
													variant={selectedAgentMode === 'council' ? 'voice' : 'research'}
												/>
											{/if}

											{#if currentChatConversationId !== INTERVIEW_CHAT_ID && (hasThinkingContent || (isStreaming && isLastMessage))}
												<!-- Interview room excluded: the companion below is
												     its one indicator, and the model's reasoning
												     about the person must never surface as chrome
												     in the room built on their own account. -->
												<ThinkingBlock
													isThinking={isStreaming &&
														isLastMessage &&
														chat.status ===
															"streaming"}
													toolCalls={messageToolParts}
													reasoningContent={messageReasoning}
													{isStreaming}
													duration={isLastMessage
														? thinkingDuration
														: 0}
												/>
											{/if}

											{#each message.parts as part, partIndex (part.type === "text" ? `text-${partIndex}` : (part as any).toolCallId || `part-${partIndex}`)}
												{#if part.type === "text" && part.text.trim()}
													<div
														class="text-base text-foreground assistant-response"
													>
														<Markdown
															content={part.text}
															{isStreaming}
															citations={citationContext}
															onCitationClick={openCitationPanel}
														/>
														{#if message.id === "interview-opening" && partIndex === 0}
															<!-- The horizontal of the table above it: the same
															     fictional life on one wire, α toward Ω. -->
															<ChapterLifeline />
														{/if}
													</div>
											{:else if part.type === "file"}
												{@render renderFilePart(part as any)}
											{:else if part.type.startsWith("tool-") && (part as any).state === "output-available" && (part as any).output?.permission_needed}
												<!-- Any gated tool (run_action, delete_action, …) awaiting the user's "I allow" -->
												{@const output = (part as any).output}
												<PageBindingInline
													entityId={output.entity_id}
													entityType={output.entity_type}
													entityTitle={output.entity_title}
													message={output.message}
													permissionMode={true}
													onAllow={(id, type, title) => handlePermissionAllow(id, type, title)}
													onDeny={() => handlePermissionDeny()}
												/>
											{:else if part.type === "tool-write_it_up" && (part as any).state === "output-available"}
												{@const output = (part as any).output}
												<!-- The interview's close, in the transcript: the two doors. -->
												<InterviewClosedCard
													pageId={output?.document_page_id ?? null}
													chaptersWritten={output?.chapters_written ?? 0}
													alreadyExisted={output?.document_already_existed ?? false}
													chaptersError={output?.chapters_error ?? null}
												/>
											{:else if part.type === "tool-create_page" && (part as any).state === "output-available"}
												{@const output = (part as any).output}
												{#if output?.page_id}
													<PageEditResult
														type="page_created"
														title={output.title}
														pageId={output.page_id}
														onOpenPage={(id) => {
													// Open the created page beside the chat (Category A).
													windowShellStore.openRouteBeside(`/page/${id}`);
												}}
														/>
												{/if}
											{:else if part.type === "tool-edit_page" && (part as any).state === "output-available"}
												{@const output = (part as any).output}
												{#if output?.needs_binding}
													<PageBindingInline
														entityId={output.page_id}
														entityTitle={output.page_title}
														message={output.message}
														onBind={handlePageSelect}
													/>
												{:else if output?.edit}
													{@const editPageId = output.edit.page_id}
													<EditDiffCard
														status={output.applied ? 'applied' : 'failed'}
														pageId={editPageId}
														find={output.edit.find || ''}
														replace={output.edit.replace || ''}
														isFullReplace={!output.edit.find}
														onViewPage={editPageId ? () => {
															// View the edited page beside the chat (Category A).
															windowShellStore.openRouteBeside(`/page/${editPageId}`);
														} : undefined}
													/>
												{/if}
											{:else if part.type === "tool-setup_applet" && (part as any).state === "output-available"}
											{@const out = (part as any).output}
											{#if out?.applet_id && out?.status !== "check_failed"}
												<!-- The gate, in the conversation. An applet that crosses a
												     boundary is created disabled and the model cannot enable
												     it; that invariant stands. What changes is that approving
												     no longer means walking to another page to find a toggle. -->
												<AppletProposalCard
													appletId={out.applet_id}
													name={out.name}
													description={out.description}
													schedule={out.schedule}
													capabilities={out.capabilities ?? []}
													estimatedCostPerDay={out.estimated_cost_per_day}
													gated={out.gated}
													lifecycle={out.lifecycle}
													updated={out.status === "updated"}
												/>
											{/if}
											{:else if part.type === "tool-code_interpreter"}
												{@const toolPart = part as any}
												{@const isRunning = toolPart.state === "pending" || toolPart.state === "input-available"}
												{@const isError = toolPart.state === "output-error"}
												<CodeInterpreterCard
													status={isRunning ? 'running' : isError ? 'error' : 'success'}
													code={toolPart.input?.code || ''}
													output={toolPart.output}
												/>
											{:else if part.type === "tool-generate_image"}
												{@const gen = part as any}
												{#if gen.state === "output-available" && gen.output?.url}
													<figure class="generated-image">
														<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
														<img
															src={gen.output.url}
															alt={gen.output.prompt || gen.input?.prompt || "Generated image"}
															class="msg-image"
															onclick={(e) => openLightbox(e, gen.output.url, gen.output.prompt || gen.input?.prompt || "Generated image")}
														/>
													</figure>
												{:else if gen.state === "output-error"}
													<div class="tool-error mb-3 text-sm text-error p-3 bg-error-subtle rounded-lg">
														Image generation failed{gen.errorText ? ` — ${gen.errorText}` : ""}.
													</div>
												{:else}
													<div class="generating-image">
														<Icon icon="ri:image-add-line" width="16" />
														<span>Generating image…</span>
													</div>
												{/if}
												{:else if part.type.startsWith("tool-") && (part as any).state === "output-error"}
													<div
														class="tool-error mb-3 text-sm text-error p-3 bg-error-subtle rounded-lg"
													>
														<span
															class="font-medium"
															>Error:</span
														>
														{(part as any).toolName}
														failed
														{#if (part as any).errorText}
															- {(part as any)
																.errorText}
														{/if}
													</div>
												{/if}
											{/each}
											{#if messageMetadata.get(message.id)?.stopped}
												<StoppedNotice />
											{/if}
										{:else}
											{@const fileParts = message.parts.filter((p: any) => p.type === "file")}
											{#if fileParts.length > 0}
												<div class="msg-attachments">
													{#each fileParts as fp, i (i)}
														{@render renderFilePart(fp as any, true)}
													{/each}
												</div>
											{/if}
											{@const userText = message.parts
												.filter((p: any) => p.type === "text")
												.map((p: any) => p.text)
												.join("")}
											{#if userText.trim()}
												<UserMessage text={userText} />
											{/if}
										{/if}
									</div>
								</div>
							{/each}


							<!-- Optimistic thinking indicator: shows immediately on submit,
							     only until the AI SDK creates the assistant message (at text-start).
							     Once the assistant message exists, the in-message ThinkingBlock takes over. -->
							{#if currentChatConversationId === INTERVIEW_CHAT_ID}
								<!-- The interview's resident, hanging out below the last turn:
								     idle between turns, the three-dot thinking morph while the
								     model composes. Engine vendored from bloub (MIT) — see
								     lib/bloub/README.md. -->
								<div class="flex justify-start">
									<div
										class="interview-companion"
										role="presentation"
										onmouseenter={pokeCompanion}
										onmouseleave={settleCompanion}
									>
										<Bloub
											size={54}
											state={chat.status === "submitted" ||
											chat.status === "streaming"
												? "thinking"
												: interviewAsleep
													? "sleep"
													: "idle"}
											shape={interviewShape ?? DEFAULT_SHAPE}
											expression={interviewExpression ??
												"neutre"}
											ink="var(--color-foreground)"
											paper="var(--color-background)"
										/>
										{#if chat.status === "submitted" || chat.status === "streaming"}
											<span class="companion-word"
												>{interviewWord}</span
											>
										{/if}
									</div>
								</div>
							{:else if isAwaitingResponse && !lastAssistantMessage}
								<div class="flex justify-start">
									<div class="message-wrapper" data-role="assistant">
										<ThinkingBlock
											isThinking={true}
											toolCalls={[]}
											reasoningContent=""
											isStreaming={true}
											duration={0}
										/>
									</div>
								</div>
							{/if}

							<ChatError
											error={chat.error ?? null}
											onRetry={() => chat.regenerate()}
											recommendedName={recommendedFallback?.displayName}
											onSwitchAndRetry={recommendedFallback
												? switchToRecommendedAndRetry
												: undefined}
										/>
						</div>
					</div>

					{#if isEmpty && !isGhost && mobileLayout.isMobile}
						<!-- The phone's opening image: the mark assembling itself in
						     the space a conversation will fill. Desktop centers the
						     composer instead; the phone docks it permanently, which
						     left this expanse truly blank. Decorative, so hidden
						     from the tree and transparent to touches. -->
						<div class="init-hero" aria-hidden="true" out:fade={{ duration: 200 }}>
							<svg class="init-mark" viewBox="0 0 12 10.5" width="30" height="26.25" fill="currentColor">
								<circle class="init-dot init-dot-1" cx="6" cy="2.4" r="1.5" />
								<circle class="init-dot init-dot-2" cx="2.6" cy="8.1" r="1.5" />
								<circle class="init-dot init-dot-3" cx="9.4" cy="8.1" r="1.5" />
							</svg>
							<span class="init-word">Virtues</span>
						</div>
					{/if}

					{#if isEmpty && isGhost}
						<div
							class="ghost-hero"
							in:fade={{ duration: 300 }}
							out:fly={{ y: -14, duration: 300, easing: cubicInOut }}
						>
							<!-- The title alone: the tiled ghost field and the inverted
							     composer already say what this mode is — an icon and an
							     explainer on top of them was the same fact three times. -->
							<h1 class="ghost-hero-title">Temporary Chat</h1>
						</div>
					{/if}

					<!-- ChatInput -->
					<div
						class="chat-input-wrapper"
						class:is-empty={isEmpty}
						class:has-messages={!isEmpty}
						class:transitions-enabled={enableTransitions}
						class:focused={inputFocused}
						class:drag-active={dragActive}
					>
						{#if dragActive}
							<div class="drop-hint">
								<Icon icon="ri:download-2-line" width="15" />
								<span>Drop to attach &middot; images, PDFs, or audio</span>
							</div>
						{/if}
						{#if attachments.length > 0}
							<div class="attachments">
								{#each attachments as a (a.id)}
									<div class="attachment">
										{#if a.kind === "image"}
											<img src={a.url} alt={a.filename} class="attachment-thumb" />
										{:else}
											<span class="attachment-icon">
												<Icon
													icon={a.kind === "pdf"
														? "ri:file-pdf-fill"
														: a.kind === "audio"
															? "ri:music-2-line"
															: "ri:file-text-line"}
													width="18"
												/>
											</span>
										{/if}
										<div class="attachment-meta">
											<span class="attachment-name">{a.filename}</span>
											<span class="attachment-size">{formatFileSize(a.size)}</span>
										</div>
										<button
											type="button"
											class="attachment-remove"
											aria-label="Remove attachment"
											onclick={() => removeAttachment(a.id)}
										>
											<Icon icon="ri:close-line" width="13" />
										</button>
									</div>
								{/each}
							</div>
						{/if}
						{#if capabilityIssue}
							<div class="capability-banner">
								<span>
									{capabilityIssue.modelName} can't read {capabilityIssue.lacks.join(" or ")}.
								</span>
								{#if capabilityIssue.candidate}
									<button type="button" class="capability-switch" onclick={switchToCapableModel}>
										Switch to {capabilityIssue.candidate.displayName}
									</button>
								{:else}
									<span class="capability-none">No available model can read {capabilityIssue.lacks.join(" or ")} yet.</span>
								{/if}
							</div>
						{/if}
						{#if stagedRefs.length > 0}
							<div class="staged-refs">
								{#each stagedRefs as r (r.id)}
									<div class="staged-ref">
										<Icon
											icon="ri:double-quotes-l"
											width="13"
											class="staged-ref-mark"
										/>
										<div class="staged-ref-body">
											<span class="staged-ref-quote">{r.text}</span>
										</div>
										<button
											type="button"
											class="queued-remove"
											aria-label="Remove reference"
											onclick={() => removeStagedRef(r.id)}
										>
											<Icon icon="ri:close-line" width="13" />
										</button>
									</div>
								{/each}
							</div>
						{/if}
						{#if queuedMessages.length > 0}
							<div class="queued-messages">
								{#each queuedMessages as q, i (i)}
									<div class="queued-chip">
										<span class="queued-text">{q}</span>
										<button
											type="button"
											class="queued-remove"
											aria-label="Remove queued message"
											onclick={() => removeQueued(i)}
										>
											<Icon icon="ri:close-line" width="13" />
										</button>
									</div>
								{/each}
							</div>
						{/if}
						{#if interviewClosed}
							<!-- The interview is over: no composer, the two doors instead. -->
							<InterviewClosedCard
								standing
								pageId={interviewDocumentPageId}
								chaptersWritten={interviewClosedPart?.chapters_written ?? 0}
								chaptersError={interviewClosedPart?.chapters_error ?? null}
							/>
						{:else}
						<ChatInput
							allowEmptySubmit={stagedRefs.length > 0 || attachments.length > 0}
							onAttach={addFiles}
							bind:value={input}
							bind:focused={inputFocused}
							disabled={false}
							sendDisabled={chat.status !== "ready"}
							isStreaming={chat.status === "streaming"}
							maxWidth="max-w-3xl"
							placeholder={isGhost ? "Write a message (temporary)…" : "Write a message..."}
							onSubmit={(text) => handleChatSubmit(text)}
							onStop={() => handleChatStop()}
						/>
						{/if}

					</div>
				</div>
			</div>
		</div>
	</div>

	<CitationPanel
		citation={selectedCitation}
		open={citationPanelOpen}
		onClose={closeCitationPanel}
	/>
{/if}

<style>
	.loading-container {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
	}

	.loading-spinner {
		width: 24px;
		height: 24px;
		border: 2px solid var(--color-border);
		border-top-color: var(--color-primary);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.chat-root {
		height: 100%;
		width: 100%;
		display: flex;
		position: relative;
	}

	/* Track D in-text reference mark — the app's own warm highlight token, one
	   marker for every staged reference. Painted via the CSS Custom Highlight API
	   (no DOM mutation, no reflow), so it's theme-aware for free. */
	:global(::highlight(vref)) {
		background-color: var(--color-highlight);
		color: var(--color-highlight-foreground);
	}

	.staged-refs {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
		margin-bottom: 0.5rem;
	}

	.staged-ref {
		display: flex;
		align-items: baseline;
		gap: 0.4375rem;
		padding: 0.375rem 0.5rem 0.375rem 0.625rem;
		border: 1px solid var(--color-border-subtle);
		border-radius: 0.625rem;
		background: var(--color-surface-elevated);
	}

	.staged-ref :global(.staged-ref-mark) {
		flex-shrink: 0;
		color: var(--color-foreground-subtle);
		transform: translateY(1px);
	}

	.staged-ref-body {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 0.0625rem;
	}

	.staged-ref-quote {
		font-size: 0.8125rem;
		color: var(--color-foreground-muted);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.chat-container {
		display: flex;
		height: 100%;
		width: 100%;
		position: relative;
	}

	.chat-area {
		flex: 1;
		height: 100%;
		position: relative;
		overflow: hidden;
	}

	/* `.chat-topbar` and its title/input rules lived here. The pane no longer
	   prints the chat's name at all — the tab above it does, and one name per
	   thing is the rule. `.chat-topbar-right` (context ring, ghost toggle, ⋮)
	   stays; it holds controls, not a label. */

	.chat-topbar-right {
		position: absolute;
		top: 8px;
		right: 12px;
		z-index: 6;
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.ghost-toggle {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		border-radius: 9px;
		color: var(--color-foreground-subtle);
		background: color-mix(in srgb, var(--color-surface) 72%, transparent);
		backdrop-filter: blur(8px);
		-webkit-backdrop-filter: blur(8px);
		transition:
			color 0.15s ease,
			background-color 0.15s ease;
		cursor: pointer;
	}

	/* 28px of visible chip, 44pt of reachable square — the chip is deliberately
	   small and floats over the transcript, so the target grows around it
	   rather than under it. */
	@media (max-width: 768px), (pointer: coarse) {
		/* (The composer's phone padding lives in the docked-composer block at
		   the end of these styles — it must follow the base rules to win the
		   cascade.) */

		.ghost-toggle {
			position: relative;
		}

		.ghost-toggle::after {
			content: "";
			position: absolute;
			top: 50%;
			left: 50%;
			width: 44px;
			height: 44px;
			transform: translate(-50%, -50%);
		}
	}

	.ghost-toggle:hover:not(:disabled) {
		color: var(--color-foreground);
		background: var(--hover-bg);
	}

	.ghost-toggle.active {
		color: var(--color-primary);
		background: color-mix(in srgb, var(--color-primary) 14%, transparent);
	}

	.ghost-toggle:disabled {
		cursor: default;
	}

	.chat-menu-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		border-radius: 9px;
		color: var(--color-foreground-subtle);
		background: color-mix(in srgb, var(--color-surface) 72%, transparent);
		backdrop-filter: blur(8px);
		-webkit-backdrop-filter: blur(8px);
		transition:
			color 0.15s ease,
			background-color 0.15s ease;
		cursor: pointer;
	}

	.chat-menu-btn:hover {
		color: var(--color-foreground);
		background: var(--hover-bg);
	}

	.chat-topbar-right > :global(*) {
		animation: topbar-pop-in 260ms cubic-bezier(0.34, 1.4, 0.64, 1) backwards;
	}

	@keyframes topbar-pop-in {
		from {
			opacity: 0;
			transform: scale(0.7);
		}
		to {
			opacity: 1;
			transform: scale(1);
		}
	}

	/* Ghost/temporary chat — faint tiled ghost field, theme-aware via mask. The
	   field reveals as a circle expanding from the composer (screen center) so the
	   ghosts ripple outward from the middle. */
	.chat-area.ghost::before {
		content: "";
		position: absolute;
		inset: 0;
		z-index: 0;
		pointer-events: none;
		background: var(--color-foreground);
		opacity: 0.035;
		-webkit-mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'%3E%3Cpath d='M12 2a8 8 0 0 0-8 8v10l2.5-2 2.5 2 2.5-2 2.5 2 2.5-2 2.5 2V10a8 8 0 0 0-8-8z' fill='%23000'/%3E%3C/svg%3E");
		mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'%3E%3Cpath d='M12 2a8 8 0 0 0-8 8v10l2.5-2 2.5 2 2.5-2 2.5 2 2.5-2 2.5 2V10a8 8 0 0 0-8-8z' fill='%23000'/%3E%3C/svg%3E");
		-webkit-mask-size: 46px 46px;
		mask-size: 46px 46px;
		-webkit-mask-repeat: repeat;
		mask-repeat: repeat;
		animation: ghost-wave-in 900ms cubic-bezier(0.22, 1, 0.36, 1) both;
	}

	@keyframes ghost-wave-in {
		from {
			opacity: 0;
			clip-path: circle(0% at 50% 50%);
		}
		to {
			opacity: 0.035;
			clip-path: circle(120% at 50% 50%);
		}
	}

	/* Ghost mode inverts the composer: the pill you type into flips to the
	   theme's ink, so the mode is a material change under your fingers, not a
	   label you have to remember reading. Done by remapping the pill's tokens
	   — everything inside (placeholder, buttons, the model pill) follows on
	   its own. The originals are captured one scope up because a custom
	   property cannot swap with itself in place. */
	.chat-area.ghost {
		--ghost-pill-bg: var(--color-foreground);
		--ghost-pill-ink: var(--color-surface);
	}
	.chat-area.ghost :global(.chat-input-wrapper.bg-surface) {
		/* Both token families: the Tailwind theme tokens (--foreground, used
		   by text-foreground et al.) and the component tokens (--color-*). */
		--surface: var(--ghost-pill-bg);
		--foreground: var(--ghost-pill-ink);
		--color-surface: var(--ghost-pill-bg);
		--color-foreground: var(--ghost-pill-ink);
		--color-foreground-muted: color-mix(in srgb, var(--ghost-pill-ink) 65%, transparent);
		--color-foreground-subtle: color-mix(in srgb, var(--ghost-pill-ink) 45%, transparent);
		--color-border-strong: transparent;
		--color-border: color-mix(in srgb, var(--ghost-pill-ink) 18%, transparent);
		--hover-bg: color-mix(in srgb, var(--ghost-pill-ink) 12%, transparent);
		background: var(--ghost-pill-bg);
		color: var(--ghost-pill-ink);
	}

	/* The send control flips with the pill: ink circle, pill-colored glyph —
	   otherwise `.btn-primary` (secondary bg, surface ink) lands dark-on-dark
	   inside the inverted pill. */
	.chat-area.ghost :global(.chat-input-wrapper .btn-primary) {
		background-color: var(--ghost-pill-ink);
		color: var(--ghost-pill-bg);
	}

	@media (prefers-reduced-motion: reduce) {
		.chat-area.ghost::before,
		.chat-topbar-right > :global(*) {
			animation: none;
		}
		.chat-input-wrapper.transitions-enabled,
		.chat-layout {
			transition: opacity 0.2s ease;
		}
	}

	.ghost-hero {
		position: absolute;
		left: 0;
		right: 0;
		bottom: calc(50% + 52px);
		z-index: 2;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.375rem;
		text-align: center;
		padding: 0 1.5rem;
		pointer-events: none;
	}

	.ghost-hero-title {
		font-family: var(--font-serif);
		font-size: 1.75rem;
		font-weight: 400;
		color: var(--color-foreground);
	}

	/* ── The phone's opening image ──
	   The ∴ mark and wordmark, seated in the upper half of the empty room —
	   above center so the (bottom-docked) composer and rising keyboard never
	   crowd it. The entrance is the mark ASSEMBLING: three dots settle into
	   the trivet one by one, then the word surfaces under them. All
	   keyframes are from-only with `backwards` fill — an explicit `to` with
	   a fill-mode is what once pinned a disabled button solid ink (see the
	   airlock's rise animation for the same rule). */
	.init-hero {
		position: absolute;
		left: 0;
		right: 0;
		bottom: calc(50% + 72px);
		z-index: 2;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.875rem;
		text-align: center;
		pointer-events: none;
		color: var(--color-foreground);
	}

	.init-mark {
		overflow: visible;
	}

	.init-word {
		/* The masthead's register, verbatim: serif, never bold, hairline
		   stroke for logo presence at text weight. */
		font-family: var(--font-serif);
		font-size: 1.375rem;
		font-weight: 400;
		letter-spacing: 0.03em;
		-webkit-text-stroke: 0.2px currentColor;
	}

	@media (prefers-reduced-motion: no-preference) {
		.init-dot {
			animation: init-dot-settle 0.55s cubic-bezier(0.22, 1, 0.36, 1) backwards;
			transform-origin: center;
			transform-box: fill-box;
		}
		.init-dot-1 {
			animation-delay: 0.15s;
		}
		.init-dot-2 {
			animation-delay: 0.32s;
		}
		.init-dot-3 {
			animation-delay: 0.49s;
		}
		.init-word {
			animation: init-word-rise 0.7s cubic-bezier(0.22, 1, 0.36, 1) 0.75s backwards;
		}
	}

	@keyframes init-dot-settle {
		from {
			opacity: 0;
			transform: scale(0.3) translateY(3px);
		}
	}

	@keyframes init-word-rise {
		from {
			opacity: 0;
			transform: translateY(8px);
		}
	}


	.page-container {
		height: 100%;
		position: relative;
	}

	.chat-layout {
		height: 100%;
		opacity: 0;
		pointer-events: none;
		/* Fade + rise in as the composer glides down (matched to the ~400ms glide). */
		transform: translateY(10px);
		transition:
			opacity 0.32s ease,
			transform 0.4s cubic-bezier(0.76, 0, 0.24, 1);
		position: relative;
		z-index: 1;
		/* Keep scroll position stable as streamed content grows above the fold */
		overflow-anchor: auto;
		/* Use standard scrollbar styling — preserves overlay scrollbar behavior on macOS
		   (unlike ::-webkit-scrollbar which forces classic scrollbars that steal layout space) */
		scrollbar-width: thin;
		scrollbar-color: var(--color-border) transparent;
	}

	.chat-layout.visible {
		opacity: 1;
		transform: translateY(0);
		pointer-events: auto;
	}

	.interview-companion {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0 0 0.5rem;
		/* The bloub viewBox is ±158 around a body of radius 100, so the SVG
		   carries (58/316)·size of built-in whitespace per side; pull the ball's
		   edge back onto the column's left margin, and its top toward the
		   conversation's tail. Keep the px in step with the size= prop. */
		margin-left: calc(54px * -58 / 316);
		margin-top: calc(54px * -58 / 316);
	}

	.companion-word {
		font-size: 0.8125rem;
		color: var(--color-foreground);
		opacity: 0.5;
	}

	.messages-container {
		max-width: 48rem;
		margin: 0 auto;
		width: 100%;
		padding: 1.5rem 2rem 10rem 2rem;
		display: flex;
		flex-direction: column;
		gap: 1rem;
		position: relative;
		z-index: 1;
		/* Stop a growing streamed message from reflowing/repainting the whole list.
		   Safe here: the sticky .chat-input-wrapper is a sibling of the scroller,
		   not a descendant, so layout containment doesn't affect it. */
		contain: layout paint;
	}

	.chat-input-wrapper {
		position: absolute;
		bottom: 0;
		left: 0;
		right: 0;
		margin: 0 auto;
		width: 100%;
		max-width: 48rem;
		padding: 0 2rem 2rem 2rem;
		/* 78px is what this measured when it was written as "1rem plus the
		   floating tab bar's reserve": the bar is gone, but on desktop — where
		   no bar ever rendered — that sum had become the composer's resting
		   inset off the window edge, so the number stays and the derivation
		   goes. The phone override below is where the bar's room actually
		   came out. */
		padding-bottom: 78px;
		background-color: var(--color-surface);
		background-image: var(--background-image);
		background-blend-mode: multiply;
		box-sizing: border-box;
		z-index: 10;
		/* Docked resting state. The empty state centers itself relative to this same
		   bottom-anchored box (bottom:50% + translateY) so the whole center→dock
		   travel is one interpolatable transition — no snap, no position swap. */
		transform: translateY(0);
		will-change: bottom, transform;
	}

	.queued-messages {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
		margin-bottom: 0.5rem;
	}

	.queued-chip {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.375rem 0.625rem;
		border: 1px solid var(--color-border);
		border-radius: 0.625rem;
		background: var(--color-surface-elevated);
		font-size: 0.8125rem;
		color: var(--color-foreground-muted);
	}

	.queued-text {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.queued-remove {
		flex-shrink: 0;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 0.125rem;
		border-radius: 0.375rem;
		color: var(--color-foreground-muted);
		transition: background-color 0.15s ease;
	}

	.queued-remove:hover {
		background: var(--color-border);
	}

	/* Track E1 — composer attachment previews */
	.attachments {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		margin-bottom: 0.5rem;
	}

	.attachment {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.375rem 0.5rem 0.375rem 0.375rem;
		border: 1px solid var(--color-border-subtle);
		border-radius: 0.625rem;
		background: var(--color-surface-elevated);
		max-width: 15rem;
	}

	.attachment-thumb {
		width: 2.25rem;
		height: 2.25rem;
		border-radius: 0.4rem;
		object-fit: cover;
		flex-shrink: 0;
		display: block;
	}

	.attachment-icon {
		width: 2.25rem;
		height: 2.25rem;
		border-radius: 0.4rem;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--color-surface);
		color: var(--color-foreground-muted);
		flex-shrink: 0;
	}

	.attachment-meta {
		display: flex;
		flex-direction: column;
		gap: 0.0625rem;
		min-width: 0;
	}

	.attachment-name {
		font-size: 0.8125rem;
		color: var(--color-foreground);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.attachment-size {
		font-size: 0.6875rem;
		color: var(--color-foreground-subtle);
	}

	.attachment-remove {
		flex-shrink: 0;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 0.125rem;
		border-radius: 0.375rem;
		color: var(--color-foreground-muted);
		transition: background-color 0.15s ease;
	}

	.attachment-remove:hover {
		background: var(--color-border);
	}

	.capability-banner {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 0.5rem;
		margin-bottom: 0.5rem;
		padding: 0.375rem 0.625rem;
		border: 1px solid var(--color-warning, var(--color-border));
		border-radius: 0.625rem;
		background: var(--color-warning-subtle, var(--color-surface-elevated));
		font-size: 0.8125rem;
		color: var(--color-foreground);
	}

	.capability-switch {
		color: var(--color-primary);
		font-weight: 500;
	}

	.capability-switch:hover {
		text-decoration: underline;
	}

	.capability-none {
		color: var(--color-foreground-muted);
	}

	/* Track E1 — in-message media */
	.msg-attachments {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		margin-bottom: 0.5rem;
	}

	.msg-image {
		max-width: min(420px, 100%);
		max-height: 420px;
		border-radius: 0.75rem;
		border: 1px solid var(--color-border-subtle);
		display: block;
		cursor: zoom-in;
	}

	.msg-audio {
		width: min(420px, 100%);
	}

	.msg-file {
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.5rem 0.75rem;
		border: 1px solid var(--color-border-subtle);
		border-radius: 0.625rem;
		background: var(--color-surface-elevated);
		font-size: 0.875rem;
		color: var(--color-foreground);
		text-decoration: none;
	}

	.msg-file:hover {
		border-color: var(--color-border-strong);
	}

	/* Track E2 — generated image */
	.generated-image {
		margin: 0.5rem 0;
	}

	.generating-image {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		margin: 0.5rem 0;
		padding: 0.5rem 0.75rem;
		border: 1px solid var(--color-border-subtle);
		border-radius: 0.625rem;
		background: var(--color-surface-elevated);
		font-size: 0.8125rem;
		color: var(--color-foreground-muted);
	}

	/* Track E1 — in-place drag affordance: the composer becomes the dropzone
	   (no full-screen scrim — context stays visible, the cue points at the
	   exact landing spot). Drop still works anywhere over the chat root. */
	.drop-hint {
		position: absolute;
		left: 50%;
		bottom: calc(100% - 0.5rem);
		transform: translateX(-50%);
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.3rem 0.7rem;
		border-radius: var(--radius-full);
		background: var(--color-primary);
		color: var(--color-on-primary, #fff);
		font-size: 0.75rem;
		font-weight: 500;
		white-space: nowrap;
		box-shadow: 0 6px 18px -6px color-mix(in srgb, var(--color-primary) 60%, transparent);
		pointer-events: none;
		z-index: 11;
		animation: drop-hint-in 0.18s cubic-bezier(0.22, 1, 0.36, 1);
	}

	@keyframes drop-hint-in {
		from {
			opacity: 0;
			transform: translateX(-50%) translateY(0.35rem);
		}
		to {
			opacity: 1;
			transform: translateX(-50%) translateY(0);
		}
	}

	/* Accent ring + gentle lift on the actual composer box while dragging. */
	.chat-input-wrapper.drag-active :global(.chat-input-container .chat-input-wrapper) {
		border-color: var(--color-primary);
		box-shadow:
			0 0 0 3px color-mix(in srgb, var(--color-primary) 22%, transparent),
			0 10px 28px -14px color-mix(in srgb, var(--color-primary) 50%, transparent);
		transition:
			border-color 0.15s ease,
			box-shadow 0.15s ease;
	}

	.chat-input-wrapper.transitions-enabled {
		/* Deliberate ~600ms ease-in-out glide. The surface mask fades in only near
		   the end (delayed) so it doesn't read as a panel sliding over the messages. */
		transition:
			bottom 0.4s cubic-bezier(0.76, 0, 0.24, 1),
			transform 0.4s cubic-bezier(0.76, 0, 0.24, 1),
			background-color 0.22s ease 0.2s;
	}

	.chat-input-wrapper.is-empty {
		/* Centered relative to the same bottom anchor: bottom edge to mid-container,
		   then nudged down half its own height → exact vertical center, any height. */
		bottom: 50%;
		transform: translateY(50%);
		/* Nothing to mask when centered — let the background (incl. ghost field)
		   show through instead of a solid surface block around the composer. */
		background-color: transparent;
		background-image: none;
	}

	/* Phones dock the composer PERMANENTLY — no centered empty state, no
	   center→dock travel to animate, nothing to snap. The input rests on the
	   bottom and the keyboard pushes it up (main's content box shrinks under
	   it via --keyboard-inset). This block sits after the base rules on
	   purpose: it ties them on specificity, and cascade order is what lets it
	   win — an earlier version lived above them and silently lost. */
	@media (max-width: 768px), (pointer: coarse) {
		.chat-input-wrapper,
		.chat-input-wrapper.is-empty {
			bottom: 0;
			transform: translateY(0);
			/* Snug above the keys: the home-indicator gap collapses as the
			   keyboard inset grows, so the pill hugs the keyboard when it's up
			   and clears the indicator when it's not. */
			padding-bottom: calc(
				1rem + max(env(safe-area-inset-bottom) - var(--keyboard-inset, 0px), 0px)
			);
		}
	}

	.hero-section {
		text-align: center;
		opacity: 0;
		max-height: 0;
		overflow: hidden;
	}

	.hero-section.transitions-enabled {
		transition:
			opacity 0.3s ease-in-out,
			max-height 0.3s ease-in-out;
	}

	.hero-section.visible {
		opacity: 1;
		max-height: 150px;
	}

	.hero-title {
		text-align: center;
	}

	.message-wrapper {
		position: relative;
		width: 100%;
		padding: 0.5rem 0;
		min-width: 0;
		overflow-wrap: break-word;
		word-break: break-word;
		/* Isolate each message's layout/paint so a re-render of one (e.g. the
		   streaming tail) can't reflow siblings. */
		contain: layout paint;
	}

	.message-wrapper :global(h1),
	.message-wrapper :global(h2),
	.message-wrapper :global(h3),
	.message-wrapper :global(h4) {
		margin-top: 0;
	}

	/* User message card styling — hugs its content (left-aligned). Radius mirrors
	   the composer's language: big enough that a one-line bubble caps into a pill
	   (radius ≥ half its height) to match the input, but still leaves flat sides
	   once the text wraps, so 2+ lines read as a clean rounded rect, not a lozenge. */
	.message-wrapper[data-role="user"] {
		background: var(--color-surface-elevated);
		border: 1px solid var(--color-border);
		border-radius: 1.5rem;
		padding: 10px 16px;
		width: fit-content;
		max-width: 80%;
	}

	/* A user turn with attachments hugs its content (photo + caption) instead of
	   spanning full width with dead space. Text-only user turns are unchanged.
	   Direct class (set in the loop) — robust vs :has()/snippet scoping. */
	.message-wrapper.user-has-attachment {
		width: fit-content;
		max-width: 100%;
	}

	/* User-attached images render as compact thumbnails (class is on the <img>
	   itself); assistant/generated images keep the larger size. */
	.msg-image.compact-img {
		max-width: min(260px, 100%);
		max-height: 260px;
	}

	/* Assistant response text - spacing after thinking block */
	.assistant-response {
		padding-top: 4px;
	}

	.shiny-title {
		overflow: visible;
		padding-bottom: 0.25rem;
	}

	.chat-input-wrapper.focused .shiny-title {
		background-image: linear-gradient(
			90deg,
			var(--color-primary) 0%,
			var(--color-primary) 30%,
			transparent 55%,
			var(--color-foreground) 80%,
			var(--color-foreground) 100%
		);
		background-position: 100% center;
		background-size: 300% auto;
		-webkit-background-clip: text;
		background-clip: text;
		color: var(--color-foreground);
		-webkit-text-fill-color: transparent;
		animation: shiny-title 1.18s cubic-bezier(0.3, 0.9, 0.4, 1) forwards;
	}

	@keyframes shiny-title {
		0% {
			background-position: 100% center;
		}
		3% {
			background-position: 100% center;
		}
		100% {
			background-position: 0% center;
		}
	}

</style>
