import type { PageLoad } from './$types';

export interface TimelineBlock {
	start_time: string;
	end_time: string | null;
	source_ontology: string;
	boundary_type: string;
	fidelity: number;
	metadata: Record<string, unknown>;
}

export const load: PageLoad = async ({ fetch, url }) => {
	// Get date from URL params, default to Nov 9, 2025 (Monday in Rome seed data)
	const dateParam = url.searchParams.get('date') || '2025-11-09';
	const hourParam = url.searchParams.get('hour') || '0';

	try {
		// Calculate next day (24-hour window may span into it)
		const selectedDate = new Date(dateParam + 'T00:00:00');
		const nextDate = new Date(selectedDate);
		nextDate.setDate(nextDate.getDate() + 1);
		const nextDateStr = nextDate.toISOString().split('T')[0];

		// Fetch timeline blocks for both days (window may span midnight)
		const [dayBlocks, nextDayBlocks] = await Promise.all([
			fetch(`/api/timeline/day/${dateParam}`).then((r) => r.json()),
			fetch(`/api/timeline/day/${nextDateStr}`).then((r) => r.json())
		]);

		// Combine and deduplicate blocks from both days
		// Both API calls extend their query window, so the same midnight-spanning
		// block can appear in both responses - deduplicate by start_time + ontology
		const combinedBlocks = [
			...(Array.isArray(dayBlocks) ? dayBlocks : []),
			...(Array.isArray(nextDayBlocks) ? nextDayBlocks : [])
		];
		const seen = new Set<string>();
		const allBlocks = combinedBlocks.filter((block) => {
			const key = `${block.start_time}-${block.source_ontology}`;
			if (seen.has(key)) return false;
			seen.add(key);
			return true;
		}) as TimelineBlock[];

		return {
			timelineBlocks: allBlocks,
			selectedDate: dateParam,
			selectedHour: parseInt(hourParam, 10),
			error: null
		};
	} catch (error) {
		console.error('Failed to load timeline data:', error);
		return {
			timelineBlocks: [],
			selectedDate: dateParam,
			selectedHour: parseInt(hourParam, 10),
			error: error instanceof Error ? error.message : 'Failed to load timeline data'
		};
	}
};
