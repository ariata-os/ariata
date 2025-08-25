#!/usr/bin/env python3
"""
Generate a consolidated registry of all sources and streams from YAML configurations.
This creates _generated_registry.yaml which is used by seed.ts and other components.
"""

import sys
import yaml
from pathlib import Path
from typing import Dict, Any, List
from datetime import datetime
import json


def load_yaml(filepath: Path) -> Dict[str, Any]:
    """Load a YAML file safely."""
    if not filepath.exists():
        return {}
    
    with open(filepath, 'r', encoding='utf-8') as f:
        return yaml.safe_load(f) or {}


def collect_sources_and_streams() -> Dict[str, Any]:
    """
    Collect all source and stream configurations from the sources directory.
    
    Returns:
        Dictionary with 'sources' and 'streams' keys containing all configurations
    """
    sources_dir = Path(__file__).parent.parent / 'sources'
    registry = {
        'sources': {},
        'streams': {},
        'metadata': {
            'generated_at': datetime.utcnow().isoformat() + 'Z',
            'version': '2.0.0'
        }
    }
    
    # Walk through each source directory
    for source_dir in sorted(sources_dir.iterdir()):
        if not source_dir.is_dir() or source_dir.name.startswith(('_', '.', 'base', 'validation')):
            continue
        
        # Load source configuration
        source_yaml_path = source_dir / '_source.yaml'
        if source_yaml_path.exists():
            source_config = load_yaml(source_yaml_path)
            if source_config:
                source_name = source_dir.name
                
                # Add streams_config list to source
                source_config['streams_config'] = []
                
                # Collect all streams for this source
                for stream_dir in sorted(source_dir.iterdir()):
                    if not stream_dir.is_dir() or stream_dir.name.startswith(('_', '.')):
                        continue
                    
                    # Load stream configuration
                    stream_yaml_path = stream_dir / '_stream.yaml'
                    if stream_yaml_path.exists():
                        stream_config = load_yaml(stream_yaml_path)
                        if stream_config:
                            # Generate stream name as source_stream
                            stream_name = f"{source_name}_{stream_dir.name}"
                            
                            # Add source reference
                            stream_config['source'] = source_name
                            stream_config['name'] = stream_name
                            
                            # Add to registry
                            registry['streams'][stream_name] = stream_config
                            
                            # Add to source's stream list
                            source_config['streams_config'].append({
                                'name': stream_name,
                                'display_name': stream_config.get('display_name', stream_dir.name),
                                'description': stream_config.get('description', ''),
                                'required_scopes': stream_config.get('auth', {}).get('required_scopes', [])
                            })
                
                # Add source to registry
                registry['sources'][source_name] = source_config
                print(f"  ✓ Collected source: {source_name} with {len(source_config['streams_config'])} streams")
    
    return registry


def write_registry(registry: Dict[str, Any], output_path: Path):
    """Write the registry to a YAML file."""
    with open(output_path, 'w', encoding='utf-8') as f:
        yaml.dump(registry, f, default_flow_style=False, sort_keys=False, allow_unicode=True)


def write_python_registry(registry: Dict[str, Any], output_path: Path):
    """Write the registry as a Python module for direct import."""
    with open(output_path, 'w', encoding='utf-8') as f:
        f.write('"""Auto-generated registry of sources and streams."""\n\n')
        f.write(f'# Generated at: {registry["metadata"]["generated_at"]}\n\n')
        f.write('REGISTRY = ')
        f.write(json.dumps(registry, indent=2, ensure_ascii=False))
        f.write('\n')


def main():
    """Main function to generate the registry."""
    print("🔄 Generating source and stream registry...")
    
    # Collect all configurations
    registry = collect_sources_and_streams()
    
    # Write YAML registry
    yaml_output = Path(__file__).parent.parent / 'sources' / '_generated_registry.yaml'
    write_registry(registry, yaml_output)
    print(f"\n✅ Generated YAML registry: {yaml_output}")
    
    # Write Python registry
    py_output = Path(__file__).parent.parent / 'sources' / '_generated_registry.py'
    write_python_registry(registry, py_output)
    print(f"✅ Generated Python registry: {py_output}")
    
    # Print summary
    print(f"\n📊 Registry Summary:")
    print(f"   Sources: {len(registry['sources'])}")
    print(f"   Streams: {len(registry['streams'])}")
    
    return 0


if __name__ == '__main__':
    sys.exit(main())