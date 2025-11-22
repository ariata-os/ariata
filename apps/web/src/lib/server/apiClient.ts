/**
 * API Client for communicating with the Rust backend
 */
import { env } from '$env/dynamic/private';

export class ApiClient {
	private baseUrl: string;

	constructor(baseUrl?: string) {
		this.baseUrl = baseUrl || env.RUST_API_URL || 'http://localhost:8000';
	}

	async get(path: string): Promise<Response> {
		const url = `${this.baseUrl}/api${path}`;
		return fetch(url, {
			method: 'GET',
			headers: {
				'Content-Type': 'application/json'
			}
		});
	}

	async post(path: string, body?: any): Promise<Response> {
		const url = `${this.baseUrl}/api${path}`;
		return fetch(url, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json'
			},
			body: body ? JSON.stringify(body) : undefined
		});
	}

	async put(path: string, body?: any): Promise<Response> {
		const url = `${this.baseUrl}/api${path}`;
		return fetch(url, {
			method: 'PUT',
			headers: {
				'Content-Type': 'application/json'
			},
			body: body ? JSON.stringify(body) : undefined
		});
	}

	async delete(path: string): Promise<Response> {
		const url = `${this.baseUrl}/api${path}`;
		return fetch(url, {
			method: 'DELETE',
			headers: {
				'Content-Type': 'application/json'
			}
		});
	}
}
