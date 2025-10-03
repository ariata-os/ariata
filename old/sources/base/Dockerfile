FROM python:3.12-slim

WORKDIR /app

# Install system dependencies (for both services)
RUN apt-get update && apt-get install -y \
    gcc \
    g++ \
    python3-dev \
    libavcodec-dev \
    libavformat-dev \
    libavutil-dev \
    libswscale-dev \
    && rm -rf /var/lib/apt/lists/*

# Install uv for faster dependency installation
RUN pip install uv

# Copy pyproject.toml and uv.lock for dependency installation
COPY sources/base/pyproject.toml sources/base/
COPY sources/base/uv.lock* sources/base/

# Install dependencies with uv
WORKDIR /app/sources/base
RUN uv sync --no-dev

# Return to app root
WORKDIR /app

# Copy application code
COPY sources/ ./sources/
COPY scripts/ ./scripts/

# Set Python path
ENV PYTHONPATH=/app

# Create service user for security
RUN useradd -m -u 1000 service && chown -R service:service /app
USER service

# Change to base directory to run services
WORKDIR /app/sources/base

# Ensure Python can find the modules
ENV PYTHONPATH=/app:$PYTHONPATH

# Default command (can be overridden in docker-compose.yml)
CMD ["echo", "Please specify a service to run via docker-compose.yml"]