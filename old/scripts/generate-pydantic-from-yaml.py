#!/usr/bin/env python3
"""
Generate Pydantic models from YAML stream definitions.

This script reads _stream.yaml files from the sources directory and generates
corresponding Pydantic models in Python from the embedded schema section.

Usage: python3 scripts/generate-pydantic-from-yaml.py
"""

import os
import yaml
from pathlib import Path
from typing import Dict, List, Any, Optional
from datetime import datetime


# Type mapping from YAML to Python types
TYPE_MAPPING = {
    'string': 'str',
    'text': 'str',
    'integer': 'int',
    'bigint': 'int',
    'float': 'float',
    'double': 'float',
    'decimal': 'float',
    'real': 'float',
    'boolean': 'bool',
    'timestamp': 'datetime',
    'date': 'date',
    'time': 'time',
    'json': 'Any',  # JSON can be any valid JSON value (object, array, string, number, etc.)
    'jsonb': 'Any',  # JSONB can be any valid JSON value (object, array, string, number, etc.)
    'uuid': 'UUID',
    'array': 'List[Any]',
    'binary': 'bytes',
}


def yaml_to_python_type(yaml_type: str) -> str:
    """Convert YAML type to Python type string."""
    return TYPE_MAPPING.get(yaml_type, 'Any')


def snake_to_pascal(name: str) -> str:
    """Convert snake_case to PascalCase."""
    return ''.join(word.capitalize() for word in name.split('_'))


def generate_field_definition(column: Dict[str, Any]) -> str:
    """Generate Pydantic field definition from column spec."""
    name = column['name']
    yaml_type = column['type']
    nullable = column.get('nullable', True)
    max_length = column.get('max_length')
    description = column.get('description', '')
    default = column.get('default')
    
    # Get Python type
    python_type = yaml_to_python_type(yaml_type)
    
    # Handle nullable
    if nullable:
        python_type = f"Optional[{python_type}]"
        default_value = "None"
    else:
        default_value = "..."
    
    # Build field arguments
    field_args = [default_value]
    field_kwargs = []
    
    if description:
        field_kwargs.append(f'description="{description}"')
    
    if max_length and yaml_type in ['string', 'text']:
        field_kwargs.append(f'max_length={max_length}')
    
    # Build field definition
    if field_kwargs:
        field_str = f"Field({', '.join([str(field_args[0])] + field_kwargs)})"
    else:
        field_str = default_value if default_value == "None" else f"Field({default_value})"
    
    return f"    {name}: {python_type} = {field_str}"


def generate_pydantic_model(stream_config: Dict[str, Any], source_name: str, stream_name: str) -> str:
    """Generate complete Pydantic model from stream configuration."""
    
    schema = stream_config.get('schema', {})
    if not schema:
        return None
    
    table_name = schema.get('table_name', f'stream_{stream_config["name"]}')
    description = schema.get('description', stream_config.get('description', ''))
    columns = schema.get('columns', [])
    
    # Get processor config for minio_fields
    processor = stream_config.get('processor', {})
    minio_fields = processor.get('minio_fields', [])
    
    # Get storage config
    storage = stream_config.get('storage', {})
    storage_strategy = storage.get('strategy', 'postgres_only')
    
    # Class name from stream name
    class_name = snake_to_pascal(stream_config['name'])
    
    # Start building the model
    lines = [
        '"""',
        f'Auto-generated Pydantic model for {stream_config["name"]} stream.',
        '',
        f'Generated from: sources/{source_name}/{stream_name}/_stream.yaml',
        f'Generated at: {datetime.now().isoformat()}',
        '"""',
        '',
        'from pydantic import BaseModel, Field',
        'from datetime import datetime, date, time',
        'from typing import Optional, Dict, List, Any',
        'from uuid import UUID',
        '',
        '',
        f'class Stream{class_name}(BaseModel):',
        f'    """{description}"""',
        '',
    ]
    
    # Separate fields by requirement
    required_fields = []
    optional_fields = []
    auto_fields = []  # id, created_at, updated_at
    
    for column in columns:
        field_def = generate_field_definition(column)
        
        # Categorize fields
        name = column['name']
        if name in ['id', 'created_at', 'updated_at']:
            auto_fields.append(field_def)
        elif column.get('nullable', True):
            optional_fields.append(field_def)
        else:
            required_fields.append(field_def)
    
    # Add fields in order: required, optional, auto-managed
    if required_fields:
        lines.append('    # Required fields')
        lines.extend(required_fields)
        lines.append('')
    
    if optional_fields:
        lines.append('    # Optional fields')
        lines.extend(optional_fields)
        lines.append('')
    
    if auto_fields:
        lines.append('    # Auto-managed fields')
        lines.extend(auto_fields)
        lines.append('')
    
    # Add config class
    lines.extend([
        '    class Config:',
        '        """Model configuration."""',
        f'        table_name = "{table_name}"',
        f'        storage_strategy = "{storage_strategy}"',
    ])
    
    if minio_fields:
        lines.append(f'        minio_fields = {minio_fields}')
    else:
        lines.append('        minio_fields = []')
    
    lines.extend([
        '        orm_mode = True',
        '        validate_assignment = True',
        '',
        '    def dict_for_db(self) -> Dict[str, Any]:',
        '        """',
        '        Get dictionary for database insertion.',
        '        Excludes auto-managed fields and MinIO fields.',
        '        """',
        '        data = self.dict(exclude_unset=True)',
        '        # Remove auto-managed fields',
        '        for field in ["id", "created_at", "updated_at"]:',
        '            data.pop(field, None)',
        '        # Remove MinIO fields (they go to object storage)',
        '        for field in self.Config.minio_fields:',
        '            data.pop(field, None)',
        '        return data',
    ])
    
    return '\n'.join(lines)


def process_stream_yaml(yaml_path: Path) -> Optional[tuple[Path, str]]:
    """Process a single _stream.yaml file."""
    
    # Parse path to get source and stream names
    parts = yaml_path.parts
    source_idx = parts.index('sources')
    source_name = parts[source_idx + 1]
    stream_name = parts[source_idx + 2]
    
    # Load YAML
    with open(yaml_path) as f:
        config = yaml.safe_load(f)
    
    # Skip if no schema section
    if 'schema' not in config:
        print(f"  ⚠️  No schema section found in {yaml_path}")
        return None
    
    # Generate Pydantic model
    model_code = generate_pydantic_model(config, source_name, stream_name)
    if not model_code:
        return None
    
    # Output path
    output_path = yaml_path.parent / 'models.py'
    
    return output_path, model_code


def main():
    """Main function to generate all Pydantic models."""
    print("🔄 Generating Pydantic models from YAML files...")
    print()
    
    # Find all _stream.yaml files
    sources_dir = Path(__file__).parent.parent / 'sources'
    yaml_files = list(sources_dir.glob('**/_stream.yaml'))
    
    print(f"Found {len(yaml_files)} stream files")
    print()
    
    generated_count = 0
    
    for yaml_path in sorted(yaml_files):
        print(f"Processing: {yaml_path.relative_to(sources_dir.parent)}")
        
        result = process_stream_yaml(yaml_path)
        if result:
            output_path, model_code = result
            
            # Write model file
            with open(output_path, 'w') as f:
                f.write(model_code)
            
            print(f"  ✅ Generated: {output_path.relative_to(sources_dir.parent)}")
            generated_count += 1
    
    print()
    print(f"✨ Generated {generated_count} Pydantic model files!")
    print()
    print("Next steps:")
    print("1. Review the generated models in each stream directory")
    print("2. Update processors to use the Pydantic models")
    print("3. Remove SchemaReader usage from processors")


if __name__ == '__main__':
    main()