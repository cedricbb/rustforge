#!/bin/bash
set -e

echo "🚀 Starting CoDev.rs Agent..."

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Function to print colored output
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Default environment
export CODEV_ENV=${CODEV_ENV:-production}
export RUST_LOG=${RUST_LOG:-info}
export RUST_BACKTRACE=${RUST_BACKTRACE:-0}

log_info "Environment: $CODEV_ENV"
log_info "Log level: $RUST_LOG"

# Wait for dependencies
wait_for_service() {
    local service=$1
    local host=$2
    local port=$3
    local max_attempts=30
    local attempt=1

    log_info "Waiting for $service at $host:$port..."

    while [ $attempt -le $max_attempts ]; do
        if nc -z "$host" "$port" 2>/dev/null; then
            log_success "$service is ready!"
            return 0
        fi

        log_info "Attempt $attempt/$max_attempts - $service not ready yet..."
        sleep 2
        attempt=$((attempt + 1))
    done

    log_error "$service failed to become ready after $max_attempts attempts"
    return 1
}

# Wait for Ollama if configured
if [ "${OLLAMA_ENDPOINT}" ] && [ "${OLLAMA_ENDPOINT}" != "disabled" ]; then
    # Extract host and port from endpoint
    OLLAMA_HOST=$(echo "$OLLAMA_ENDPOINT" | sed -E 's|https?://([^:]+).*|\1|')
    OLLAMA_PORT=$(echo "$OLLAMA_ENDPOINT" | sed -E 's|https?://[^:]+:?([0-9]+)?.*|\1|')
    OLLAMA_PORT=${OLLAMA_PORT:-11434}

    if ! wait_for_service "Ollama" "$OLLAMA_HOST" "$OLLAMA_PORT"; then
        log_warning "Ollama not available, will try other providers"
    fi
fi

# Wait for Redis if configured
if [ "${REDIS_URL}" ] || [ "${CODEV_ENV}" = "production" ]; then
    REDIS_HOST=${REDIS_HOST:-redis}
    REDIS_PORT=${REDIS_PORT:-6379}

    if ! wait_for_service "Redis" "$REDIS_HOST" "$REDIS_PORT"; then
        log_warning "Redis not available, some features may be limited"
    fi
fi

# Create necessary directories
log_info "Creating necessary directories..."
mkdir -p /app/{workspace,data,config,logs}

# Set proper permissions
log_info "Setting permissions..."
chmod 755 /app/{workspace,data,config,logs}

# Copy default config if none exists
if [ ! -f "/app/config/codev.toml" ]; then
    log_info "Creating default configuration..."

    case "$CODEV_ENV" in
        "development")
            cat > /app/config/codev.toml << 'EOF'
environment = "Development"

[ai]
default_provider = "Ollama"
auto_detect_environment = true

[ai.providers.Ollama]
enabled = true
model = "codellama:7b"
endpoint = "http://ollama:11434"

[security]
default_level = "Development"
sandbox_enabled = false

[development]
hot_reload = true
debug_mode = true

[logging]
level = "debug"
format = "json"
file_enabled = true
EOF
            ;;
        "production")
            cat > /app/config/codev.toml << 'EOF'
environment = "Production"

[ai]
default_provider = "Ollama"
auto_detect_environment = false

[ai.providers.Ollama]
enabled = true
model = "codellama:7b"
endpoint = "http://ollama:11434"

[security]
default_level = "Production"
sandbox_enabled = true

[logging]
level = "info"
format = "json"
file_enabled = true
EOF
            ;;
    esac
fi

# Validate configuration
log_info "Validating configuration..."
if codev --version >/dev/null 2>&1; then
    log_success "CoDev.rs binary is working"
else
    log_error "CoDev.rs binary validation failed"
    exit 1
fi

# Health check function
health_check() {
    # Basic health checks
    if [ ! -d "/app/workspace" ]; then
        log_error "Workspace directory missing"
        return 1
    fi

    if [ ! -f "/app/config/codev.toml" ]; then
        log_error "Configuration file missing"
        return 1
    fi

    # Check if we can connect to Ollama (if configured)
    if [ "${OLLAMA_ENDPOINT}" ] && [ "${OLLAMA_ENDPOINT}" != "disabled" ]; then
        if ! curl -sf "${OLLAMA_ENDPOINT}/api/tags" >/dev/null 2>&1; then
            log_warning "Cannot connect to Ollama at ${OLLAMA_ENDPOINT}"
        else
            log_success "Ollama connection verified"
        fi
    fi

    return 0
}

# Run health check
if ! health_check; then
    log_error "Health check failed"
    exit 1
fi

log_success "CoDev.rs Agent initialized successfully"

# Handle different run modes
case "${1:-daemon}" in
    "daemon")
        log_info "Starting CoDev.rs in daemon mode..."
        exec codev daemon --config /app/config/codev.toml
        ;;
    "cli")
        log_info "Starting CoDev.rs CLI..."
        shift
        exec codev "$@"
        ;;
    "interactive")
        log_info "Starting interactive shell..."
        exec /bin/bash
        ;;
    "health")
        log_info "Running health check..."
        health_check
        exit $?
        ;;
    *)
        log_info "Executing command: $*"
        exec "$@"
        ;;
esac