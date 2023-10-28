#!/usr/bin/env bash

UUID_DB_NAME_FILE="dbname.csv"

DB_USER="${POSTGRES_USER:=postgres}"
DB_PASSWORD="${POSTGRES_PASSWORD:=password}"
DB_PORT="${POSTGRES_PORT:=5432}"
DB_HOST="${POSTGRES_HOST:=localhost}"

if [[ -z "${UUID_DB_NAME_FILE}" ]]; then
    echo "${UUID_DB_NAME_FILE}" not exist, please create this file.
    exit 1
fi

while IFS=',' read -r database; do
    echo "Dropping database: $database"
    PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -U "${DB_USER}" -p "${DB_PORT}" \
        -d "postgres" -c "DROP DATABASE IF EXISTS ${database}"
done <${UUID_DB_NAME_FILE}
