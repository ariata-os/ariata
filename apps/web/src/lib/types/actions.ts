// Actions Types - Temporal Pursuits Management
//
// "What you do reveals what you value"
// Actions track your pursuits across different time horizons

export interface Task {
	id: string;
	title: string;
	description?: string | null;
	tags?: string[] | null;
	topic_id?: string | null;
	status?: string | null; // 'active', 'on_hold', 'completed', 'abandoned'
	progress_percent?: number | null; // 0-100
	start_date?: string | null;
	target_date?: string | null;
	completed_date?: string | null;
	is_active?: boolean | null;
	created_at: string;
	updated_at: string;
}

export interface Initiative {
	id: string;
	title: string;
	description?: string | null;
	tags?: string[] | null;
	topic_id?: string | null;
	status?: string | null; // 'planning', 'active', 'on_hold', 'completed', 'abandoned'
	progress_percent?: number | null; // 0-100
	start_date?: string | null;
	target_date?: string | null;
	completed_date?: string | null;
	is_active?: boolean | null;
	created_at: string;
	updated_at: string;
}

export interface Aspiration {
	id: string;
	title: string;
	description?: string | null;
	tags?: string[] | null;
	topic_id?: string | null;
	status?: string | null; // 'dreaming', 'planning', 'pursuing', 'achieved', 'let_go'
	target_timeframe?: string | null; // e.g. "next 5 years", "by age 50", "someday"
	achieved_date?: string | null;
	is_active?: boolean | null;
	created_at: string;
	updated_at: string;
}

// Request types for creating/updating entities
export interface CreateTaskRequest {
	title: string;
	description?: string;
	tags?: string[];
	topic_id?: string;
	start_date?: string;
	target_date?: string;
}

export interface UpdateTaskRequest {
	title?: string;
	description?: string;
	tags?: string[];
	topic_id?: string;
	status?: string;
	progress_percent?: number;
	start_date?: string;
	target_date?: string;
	completed_date?: string;
}

export interface CreateInitiativeRequest {
	title: string;
	description?: string;
	tags?: string[];
	topic_id?: string;
	start_date?: string;
	target_date?: string;
}

export interface UpdateInitiativeRequest {
	title?: string;
	description?: string;
	tags?: string[];
	topic_id?: string;
	status?: string;
	progress_percent?: number;
	start_date?: string;
	target_date?: string;
	completed_date?: string;
}

export interface CreateAspirationRequest {
	title: string;
	description?: string;
	tags?: string[];
	topic_id?: string;
	target_timeframe?: string;
}

export interface UpdateAspirationRequest {
	title?: string;
	description?: string;
	tags?: string[];
	topic_id?: string;
	status?: string;
	target_timeframe?: string;
	achieved_date?: string;
}

// Discriminated union type for temporal pursuits widget
export type TemporalPursuit =
	| { type: 'task'; data: Task }
	| { type: 'initiative'; data: Initiative }
	| { type: 'aspiration'; data: Aspiration };
