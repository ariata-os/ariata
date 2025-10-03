#!/usr/bin/env python3
"""
Validate all YAML configuration files in the sources directory.
Replaces generate_registry.py since we no longer need a generated registry.
"""

import sys
import yaml
from pathlib import Path
from typing import Dict, Any, List, Optional, Tuple
from datetime import datetime
from pydantic import ValidationError

# Add sources to path to import types
sys.path.insert(0, str(Path(__file__).parent.parent / 'sources'))
try:
    from base.types import (
        validate_source_config,
        validate_stream_config,
        validate_table_schema
    )
    VALIDATION_AVAILABLE = True
except ImportError:
    print("⚠️  Warning: Pydantic validation not available. Install pydantic to enable validation.")
    print("    pip install pydantic")
    VALIDATION_AVAILABLE = False


def load_yaml(filepath: Path) -> Dict[str, Any]:
    """Load a YAML file safely."""
    if not filepath.exists():
        return {}

    with open(filepath, 'r', encoding='utf-8') as f:
        return yaml.safe_load(f) or {}


def validate_yaml(
    data: Dict[str, Any],
    yaml_type: str,
    filepath: Path,
    strict: bool = False
) -> Tuple[Optional[Any], List[str]]:
    """
    Validate YAML data against appropriate schema.
    
    Args:
        data: The YAML data to validate
        yaml_type: Type of YAML ('source' or 'stream')
        filepath: Path to the YAML file (for error reporting)
        strict: If True, raise exception on validation errors
        
    Returns:
        Tuple of (validated_model or None, list of validation errors)
    """
    if not VALIDATION_AVAILABLE:
        return None, []
    
    errors = []
    validated_model = None
    
    try:
        if yaml_type == 'source':
            validated_model = validate_source_config(data)
        elif yaml_type == 'stream':
            # For stream configs, we need to validate the schema section separately
            validated_model = validate_stream_config(data)
            
            # If there's a schema section, validate it
            if 'schema' in data:
                try:
                    validate_table_schema(data['schema'])
                except ValidationError as e:
                    for error in e.errors():
                        field_path = 'schema -> ' + ' -> '.join(str(loc) for loc in error['loc'])
                        errors.append(f"{filepath}: {field_path}: {error['msg']}")
        else:
            errors.append(f"Unknown YAML type: {yaml_type}")
    except ValidationError as e:
        for error in e.errors():
            field_path = ' -> '.join(str(loc) for loc in error['loc'])
            errors.append(f"{filepath}: {field_path}: {error['msg']}")
        
        if strict:
            raise ValidationError(f"Validation failed for {filepath}") from e
    except Exception as e:
        errors.append(f"{filepath}: Unexpected error: {str(e)}")
        if strict:
            raise
    
    return validated_model, errors


def validate_all_configs(strict: bool = False, verbose: bool = False) -> Tuple[int, List[str]]:
    """
    Validate all YAML configuration files.
    
    Args:
        strict: If True, fail on first validation error
        verbose: If True, show progress for each file
        
    Returns:
        Tuple of (error count, list of validation errors)
    """
    sources_dir = Path(__file__).parent.parent / 'sources'
    validation_errors = []
    validated_count = 0
    
    # Walk through each source directory
    for source_dir in sources_dir.iterdir():
        if not source_dir.is_dir() or source_dir.name.startswith(('_', '.', 'base', 'validation')):
            continue
        
        # Validate source config
        source_yaml_path = source_dir / '_source.yaml'
        if source_yaml_path.exists():
            if verbose:
                print(f"Validating {source_yaml_path.relative_to(sources_dir)}...")
            
            source_config = load_yaml(source_yaml_path)
            if source_config:
                _, errors = validate_yaml(source_config, 'source', source_yaml_path, strict)
                validation_errors.extend(errors)
                validated_count += 1
        
        # Walk through stream directories
        for stream_dir in source_dir.iterdir():
            if not stream_dir.is_dir() or stream_dir.name.startswith(('_', '.')):
                continue
            
            # Validate stream config (which now includes schema)
            stream_yaml_path = stream_dir / '_stream.yaml'
            if stream_yaml_path.exists():
                if verbose:
                    print(f"Validating {stream_yaml_path.relative_to(sources_dir)}...")
                
                stream_config = load_yaml(stream_yaml_path)
                if stream_config:
                    _, errors = validate_yaml(stream_config, 'stream', stream_yaml_path, strict)
                    validation_errors.extend(errors)
                    validated_count += 1
    
    if verbose:
        print(f"\nValidated {validated_count} files")
    
    return len(validation_errors), validation_errors


def main():
    """Main function to validate YAML configurations."""
    import argparse
    
    parser = argparse.ArgumentParser(description='Validate YAML configurations in sources directory')
    parser.add_argument('--strict', action='store_true', help='Fail on validation errors')
    parser.add_argument('--verbose', '-v', action='store_true', help='Show detailed validation output')
    args = parser.parse_args()
    
    print("🔍 Validating YAML configurations...")
    
    error_count, validation_errors = validate_all_configs(strict=args.strict, verbose=args.verbose)
    
    # Report validation errors
    if validation_errors:
        print(f"\n⚠️  YAML validation errors found ({error_count} issues):")
        
        # Show detailed or summary based on verbose flag
        if args.verbose:
            for error in validation_errors:
                print(f"   ❌ {error}")
        else:
            for error in validation_errors[:10]:  # Show first 10 errors in non-verbose
                print(f"   ❌ {error}")
            if len(validation_errors) > 10:
                print(f"   ... and {len(validation_errors) - 10} more errors")
                print("   Use --verbose to see all errors")
        print()
        
        if args.strict:
            print("❌ Validation failed. Please fix the errors above.")
            sys.exit(1)
    else:
        if VALIDATION_AVAILABLE:
            print("✅ All YAML files passed validation!")
        else:
            print("⚠️  Validation skipped (pydantic not installed)")
    
    print("✨ Validation complete!")
    return 0 if error_count == 0 else 1


if __name__ == '__main__':
    sys.exit(main())