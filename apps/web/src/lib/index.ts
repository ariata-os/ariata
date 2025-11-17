export { default as Modules } from "./components/Modules.svelte"
export { default as Sidebar } from "./components/Sidebar.svelte"
export { default as Page } from "./components/Page.svelte"
export { default as Breadcrumbs } from "./components/Breadcrumbs.svelte"
export { default as Button } from "./components/Button.svelte"
export { default as DevicePairing } from "./components/DevicePairing.svelte"
export { default as TagInput } from "./components/TagInput.svelte"

// Tool components (new modular structure)
export { default as BaseTool } from "./components/tools/BaseTool.svelte"
export { default as LocationMap } from "./components/tools/LocationMap.svelte"
export { default as Pursuits } from "./components/tools/Pursuits.svelte"
export { default as WebSearch } from "./components/tools/WebSearch.svelte"

// Legacy exports (for backward compatibility - can be removed later)
export { default as PursuitsWidget } from "./components/PursuitsWidget.svelte"
export { default as SearchResultsWidget } from "./components/SearchResultsWidget.svelte"
