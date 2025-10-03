"""
Dynamic Schema Reader for PostgreSQL Tables
============================================

This module provides utilities for reading table schemas from PostgreSQL's
information_schema at runtime. This eliminates the need for SQLAlchemy models
and allows Python to work with tables created by Drizzle.
"""

from typing import Dict, List, Any, Optional
from sqlalchemy import text
from sqlalchemy.orm import Session
import logging

logger = logging.getLogger(__name__)


class SchemaReader:
    """
    Read PostgreSQL table schemas dynamically from information_schema.
    
    This allows Python code to work with tables created by Drizzle without
    needing to maintain duplicate model definitions.
    
    Features:
    - Schema caching per session to reduce queries
    - Bulk operations for efficiency
    - Thread-safe cache management
    """
    
    # Class-level cache shared across instances (thread-safe with GIL)
    _global_schema_cache: Dict[str, List[Dict[str, Any]]] = {}
    _cache_timestamp: Dict[str, float] = {}
    CACHE_TTL = 300  # Cache for 5 minutes
    
    def __init__(self, db_session: Session):
        """
        Initialize with a database session.
        
        Args:
            db_session: SQLAlchemy database session
        """
        self.db = db_session
        # Instance cache for this session
        self._session_cache: Dict[str, List[Dict[str, Any]]] = {}
    
    def get_table_columns(self, table_name: str, schema: str = 'public') -> List[Dict[str, Any]]:
        """
        Get column information for a table from information_schema.
        
        Uses multi-level caching:
        1. Session cache (immediate)
        2. Global cache (5-minute TTL)
        3. Database query (fallback)
        
        Args:
            table_name: Name of the table
            schema: Database schema (default: 'public')
            
        Returns:
            List of column definitions with name, type, nullable, etc.
        """
        import time
        
        cache_key = f"{schema}.{table_name}"
        
        # Check session cache first (fastest)
        if cache_key in self._session_cache:
            logger.debug(f"Schema cache hit (session): {cache_key}")
            return self._session_cache[cache_key]
        
        # Check global cache with TTL
        if cache_key in self._global_schema_cache:
            cache_age = time.time() - self._cache_timestamp.get(cache_key, 0)
            if cache_age < self.CACHE_TTL:
                logger.debug(f"Schema cache hit (global): {cache_key}")
                result = self._global_schema_cache[cache_key]
                self._session_cache[cache_key] = result
                return result
            else:
                logger.debug(f"Schema cache expired: {cache_key}")
        
        query = text("""
            SELECT 
                column_name,
                data_type,
                character_maximum_length,
                is_nullable,
                column_default,
                ordinal_position
            FROM information_schema.columns
            WHERE table_schema = :schema
                AND table_name = :table_name
            ORDER BY ordinal_position
        """)
        
        result = self.db.execute(query, {
            'schema': schema,
            'table_name': table_name
        })
        
        columns = []
        for row in result:
            columns.append({
                'name': row.column_name,
                'type': row.data_type,
                'max_length': row.character_maximum_length,
                'nullable': row.is_nullable == 'YES',
                'default': row.column_default,
                'position': row.ordinal_position
            })
        
        # Cache the result in both caches
        import time
        self._session_cache[cache_key] = columns
        self._global_schema_cache[cache_key] = columns
        self._cache_timestamp[cache_key] = time.time()
        logger.debug(f"Schema cached: {cache_key}")
        return columns
    
    def table_exists(self, table_name: str, schema: str = 'public') -> bool:
        """
        Check if a table exists in the database.
        
        Args:
            table_name: Name of the table
            schema: Database schema (default: 'public')
            
        Returns:
            True if table exists, False otherwise
        """
        query = text("""
            SELECT EXISTS (
                SELECT 1
                FROM information_schema.tables
                WHERE table_schema = :schema
                    AND table_name = :table_name
            )
        """)
        
        result = self.db.execute(query, {
            'schema': schema,
            'table_name': table_name
        })
        
        return result.scalar()
    
    def build_insert_sql(
        self,
        table_name: str,
        data: Dict[str, Any],
        returning: Optional[List[str]] = None,
        schema: str = 'public'
    ) -> tuple[str, Dict[str, Any]]:
        """
        Build an INSERT SQL statement based on table schema and provided data.
        
        Args:
            table_name: Name of the table
            data: Dictionary of column names to values
            returning: Optional list of columns to return
            schema: Database schema (default: 'public')
            
        Returns:
            Tuple of (SQL query string, parameters dict)
        """
        # Get table columns
        columns = self.get_table_columns(table_name, schema)
        column_names = {col['name'] for col in columns}
        
        # Filter data to only include valid columns
        filtered_data = {
            key: value
            for key, value in data.items()
            if key in column_names
        }
        
        if not filtered_data:
            raise ValueError(f"No valid columns found in data for table {table_name}")
        
        # Build SQL
        columns_str = ', '.join(filtered_data.keys())
        placeholders = ', '.join([f':{key}' for key in filtered_data.keys()])
        
        sql = f"INSERT INTO {schema}.{table_name} ({columns_str}) VALUES ({placeholders})"
        
        if returning:
            returning_str = ', '.join(returning)
            sql += f" RETURNING {returning_str}"
        
        return sql, filtered_data
    
    def build_bulk_insert_sql(
        self,
        table_name: str,
        records: List[Dict[str, Any]],
        schema: str = 'public'
    ) -> tuple[str, List[Dict[str, Any]]]:
        """
        Build a bulk INSERT SQL statement for multiple records.
        
        Args:
            table_name: Name of the table
            records: List of dictionaries with column names to values
            schema: Database schema (default: 'public')
            
        Returns:
            Tuple of (SQL query string, list of parameter dicts)
        """
        if not records:
            raise ValueError("No records provided for bulk insert")
        
        # Get table columns
        columns = self.get_table_columns(table_name, schema)
        column_names = {col['name'] for col in columns}
        
        # Get common columns across all records
        common_columns = set()
        for record in records:
            if not common_columns:
                common_columns = set(record.keys()) & column_names
            else:
                common_columns &= set(record.keys())
        
        if not common_columns:
            raise ValueError(f"No common valid columns found in records for table {table_name}")
        
        # Sort columns for consistent ordering
        sorted_columns = sorted(common_columns)
        
        # Build SQL with numbered placeholders for each record
        columns_str = ', '.join(sorted_columns)
        values_clauses = []
        filtered_records = []
        
        for i, record in enumerate(records):
            placeholders = ', '.join([f':r{i}_{col}' for col in sorted_columns])
            values_clauses.append(f"({placeholders})")
            
            # Create flattened parameters
            filtered_record = {}
            for col in sorted_columns:
                filtered_record[f'r{i}_{col}'] = record.get(col)
            filtered_records.append(filtered_record)
        
        values_str = ', '.join(values_clauses)
        sql = f"INSERT INTO {schema}.{table_name} ({columns_str}) VALUES {values_str}"
        
        # Flatten parameters for execution
        params = {}
        for record in filtered_records:
            params.update(record)
        
        return sql, params
    
    def get_table_indexes(self, table_name: str, schema: str = 'public') -> List[Dict[str, Any]]:
        """
        Get index information for a table.
        
        Args:
            table_name: Name of the table
            schema: Database schema (default: 'public')
            
        Returns:
            List of index definitions
        """
        query = text("""
            SELECT 
                i.relname as index_name,
                a.attname as column_name,
                ix.indisprimary as is_primary,
                ix.indisunique as is_unique
            FROM pg_class t
            JOIN pg_index ix ON t.oid = ix.indrelid
            JOIN pg_class i ON i.oid = ix.indexrelid
            JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey)
            JOIN pg_namespace ns ON ns.oid = t.relnamespace
            WHERE t.relkind = 'r'
                AND ns.nspname = :schema
                AND t.relname = :table_name
            ORDER BY i.relname, a.attnum
        """)
        
        result = self.db.execute(query, {
            'schema': schema,
            'table_name': table_name
        })
        
        indexes = []
        for row in result:
            indexes.append({
                'name': row.index_name,
                'column': row.column_name,
                'is_primary': row.is_primary,
                'is_unique': row.is_unique
            })
        
        return indexes
    
    def clear_cache(self, table_name: Optional[str] = None, schema: str = 'public'):
        """
        Clear the schema cache.
        
        Args:
            table_name: Optional table name to clear specific cache
            schema: Database schema (default: 'public')
        """
        if table_name:
            cache_key = f"{schema}.{table_name}"
            self._schema_cache.pop(cache_key, None)
        else:
            self._schema_cache.clear()