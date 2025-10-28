#!/bin/bash
set -e

# Create multiple databases from comma-separated list
# Usage: POSTGRES_MULTIPLE_DATABASES=db1,db2,db3

if [ -n "$POSTGRES_MULTIPLE_DATABASES" ]; then
  echo "Creating multiple databases: $POSTGRES_MULTIPLE_DATABASES"

  for db in $(echo $POSTGRES_MULTIPLE_DATABASES | tr ',' ' '); do
    echo "Creating database: $db"
    psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" <<-EOSQL
      CREATE DATABASE $db;
      GRANT ALL PRIVILEGES ON DATABASE $db TO $POSTGRES_USER;
EOSQL

    # Enable pgvector extension on each database
    echo "Enabling pgvector extension on: $db"
    psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$db" <<-EOSQL
      CREATE EXTENSION IF NOT EXISTS vector;
EOSQL
  done

  echo "Multiple databases created successfully"
fi
