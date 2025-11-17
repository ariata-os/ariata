// Axiology Types - Values and Character Patterns
//
// "What you do reveals what you value"
// Axiology tracks your value system and character development

export interface Telos {
	id: string;
	title: string;
	description?: string | null;
	is_active?: boolean | null;
	topic_id?: string | null;
	created_at: string;
	updated_at: string;
}

export interface UpdateTelosRequest {
	title: string;
	description?: string;
	topic_id?: string;
}

// Simple axiology types (temperaments, virtues, vices, values)
// These share a common simple structure
interface SimpleAxiologyItem {
	id: string;
	title: string;
	description?: string | null;
	topic_id?: string | null;
	is_active?: boolean | null;
	created_at: string;
	updated_at: string;
}

export interface Temperament extends SimpleAxiologyItem {}
export interface Virtue extends SimpleAxiologyItem {}
export interface Vice extends SimpleAxiologyItem {}
export interface Value extends SimpleAxiologyItem {}

export interface Habit {
	id: string;
	title: string;
	description?: string | null;
	frequency?: string | null; // 'daily', 'weekly', 'monthly'
	time_of_day?: string | null; // 'morning', 'afternoon', 'evening', 'night'
	topic_id?: string | null;
	streak_count?: number | null;
	last_completed_date?: string | null;
	is_active?: boolean | null;
	created_at: string;
	updated_at: string;
}

export interface Preference {
	id: string;
	title: string;
	description?: string | null;
	preference_domain?: string | null; // 'work_environment', 'people', 'places', 'communication', 'activities'
	valence?: string | null; // 'strong_preference', 'mild_preference', 'neutral', 'mild_aversion', 'strong_aversion'
	person_id?: string | null;
	place_id?: string | null;
	topic_id?: string | null;
	is_active?: boolean | null;
	created_at: string;
	updated_at: string;
}

// Request types for simple axiology items
export interface CreateSimpleRequest {
	title: string;
	description?: string;
	topic_id?: string;
}

export interface UpdateSimpleRequest {
	title?: string;
	description?: string;
	topic_id?: string;
}

export interface CreateHabitRequest {
	title: string;
	description?: string;
	frequency?: string;
	time_of_day?: string;
	topic_id?: string;
}

export interface UpdateHabitRequest {
	title?: string;
	description?: string;
	frequency?: string;
	time_of_day?: string;
	topic_id?: string;
	streak_count?: number;
	last_completed_date?: string;
}

export interface CreatePreferenceRequest {
	title: string;
	description?: string;
	preference_domain?: string;
	valence?: string;
	person_id?: string;
	place_id?: string;
	topic_id?: string;
}

export interface UpdatePreferenceRequest {
	title?: string;
	description?: string;
	preference_domain?: string;
	valence?: string;
	person_id?: string;
	place_id?: string;
	topic_id?: string;
}
