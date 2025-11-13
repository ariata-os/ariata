import type { PageLoad } from './$types';

export interface OntologyOverview {
	name: string;
	domain: string;
	record_count: number;
	sample_record: Record<string, any> | null;
}

export const load: PageLoad = async ({ fetch }) => {
	try {
		const response = await fetch('/api/ontologies/overview');

		if (!response.ok) {
			throw new Error(`Failed to fetch ontologies: ${response.statusText}`);
		}

		const ontologies: OntologyOverview[] = await response.json();

		return {
			ontologies
		};
	} catch (error) {
		console.error('Error loading ontologies:', error);
		return {
			ontologies: []
		};
	}
};
