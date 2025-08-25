/**
 * Config loader that supports both YAML and JSON formats
 * Falls back to JSON if YAML is not available
 */

import fs from 'fs';
import path from 'path';
import yaml from 'js-yaml';

const CONFIG_DIR = path.join(process.cwd(), 'assets', 'config');

export interface ConfigLoader {
	loadConfig(name: string): any;
}

class YamlJsonConfigLoader implements ConfigLoader {
	private cache = new Map<string, any>();

	loadConfig(name: string): any {
		// Check cache first
		if (this.cache.has(name)) {
			return this.cache.get(name);
		}

		// Try YAML first
		const yamlPath = path.join(CONFIG_DIR, `${name}.yaml`);
		const jsonPath = path.join(CONFIG_DIR, `${name}.json`);

		let config: any = null;

		if (fs.existsSync(yamlPath)) {
			const yamlContent = fs.readFileSync(yamlPath, 'utf8');
			config = yaml.load(yamlContent);
		} else if (fs.existsSync(jsonPath)) {
			const jsonContent = fs.readFileSync(jsonPath, 'utf8');
			config = JSON.parse(jsonContent);
		} else {
			throw new Error(`Config file not found: ${name} (tried .yaml and .json)`);
		}

		// Cache the config
		this.cache.set(name, config);
		return config;
	}
}

// Export singleton instance
export const configLoader = new YamlJsonConfigLoader();

// Helper functions for specific configs
export function loadSourceConfigs() {
	return configLoader.loadConfig('source_configs');
}

export function loadStreamConfigs() {
	return configLoader.loadConfig('stream_configs');
}

export function loadSignalConfigs() {
	return configLoader.loadConfig('signal_configs');
}

export function loadDefaults() {
	return configLoader.loadConfig('defaults');
}