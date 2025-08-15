#!/bin/bash
set -e

echo "🤖 Initializing Ollama for CoDev.rs..."

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[OLLAMA]${NC} $1"
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

# Configuration
OLLAMA_ENDPOINT=${OLLAMA_ENDPOINT:-http://localhost:11434}
REQUIRED_MODELS=(
    "codellama:7b"
    "codellama:13b"
    "llama2:7b"
)

OPTIONAL_MODELS=(
    "deepseek-coder:6.7b"
    "phi:latest"
    "gemma:7b"
)

# Wait for Ollama to be ready
wait_for_ollama() {
    local max_attempts=60
    local attempt=1

    log_info "Waiting for Ollama to be ready at $OLLAMA_ENDPOINT..."

    while [ $attempt -le $max_attempts ]; do
        if curl -sf "$OLLAMA_ENDPOINT/api/tags" >/dev/null 2>&1; then
            log_success "Ollama is ready!"
            return 0
        fi

        log_info "Attempt $attempt/$max_attempts - Ollama not ready yet..."
        sleep 5
        attempt=$((attempt + 1))
    done

    log_error "Ollama failed to become ready after $max_attempts attempts"
    return 1
}

# Check if model exists
model_exists() {
    local model=$1
    curl -sf "$OLLAMA_ENDPOINT/api/tags" | grep -q "\"name\":\"$model\""
}

# Pull model with progress
pull_model() {
    local model=$1
    local max_retries=3
    local retry=1

    log_info "Pulling model: $model"

    while [ $retry -le $max_retries ]; do
        if curl -sf -X POST "$OLLAMA_ENDPOINT/api/pull" \
            -H "Content-Type: application/json" \
            -d "{\"name\": \"$model\"}" | \
            while IFS= read -r line; do
                # Parse JSON progress if available
                if echo "$line" | grep -q '"status"'; then
                    status=$(echo "$line" | sed -n 's/.*"status":"\([^"]*\)".*/\1/p')
                    if [ "$status" ]; then
                        echo -ne "\r${BLUE}[OLLAMA]${NC} $model: $status"
                    fi
                fi
            done; then
            echo
            log_success "Successfully pulled $model"
            return 0
        else
            log_warning "Failed to pull $model (attempt $retry/$max_retries)"
            retry=$((retry + 1))
            sleep 10
        fi
    done

    log_error "Failed to pull $model after $max_retries attempts"
    return 1
}

# Verify model works
verify_model() {
    local model=$1
    log_info "Verifying model: $model"

    local response=$(curl -sf -X POST "$OLLAMA_ENDPOINT/api/generate" \
        -H "Content-Type: application/json" \
        -d "{\"model\": \"$model\", \"prompt\": \"fn main() {\", \"stream\": false}" | \
        grep -o '"response":"[^"]*"' | sed 's/"response":"\(.*\)"/\1/' | head -1)

    if [ "$response" ] && [ "$response" != "null" ]; then
        log_success "Model $model is working correctly"
        return 0
    else
        log_error "Model $model verification failed"
        return 1
    fi
}

# Get available disk space in MB
get_available_space() {
    df /tmp | awk 'NR==2 {print int($4/1024)}'
}

# Estimate model size in MB
estimate_model_size() {
    local model=$1
    case $model in
        "codellama:7b"|"llama2:7b"|"gemma:7b")
            echo 4000  # ~4GB
            ;;
        "codellama:13b")
            echo 7500  # ~7.5GB
            ;;
        "deepseek-coder:6.7b")
            echo 3800  # ~3.8GB
            ;;
        "phi:latest")
            echo 2000  # ~2GB
            ;;
        *)
            echo 5000  # Default estimate
            ;;
    esac
}

# Check available space
check_space() {
    local required_space=0
    local available_space=$(get_available_space)

    log_info "Available disk space: ${available_space}MB"

    for model in "${REQUIRED_MODELS[@]}"; do
        if ! model_exists "$model"; then
            local size=$(estimate_model_size "$model")
            required_space=$((required_space + size))
        fi
    done

    if [ $required_space -gt $available_space ]; then
        log_error "Insufficient disk space. Required: ${required_space}MB, Available: ${available_space}MB"
        return 1
    fi

    log_info "Sufficient disk space available"
    return 0
}

# Main initialization
main() {
    log_info "Starting Ollama initialization for CoDev.rs"

    # Wait for Ollama
    if ! wait_for_ollama; then
        exit 1
    fi

    # Check disk space
    if ! check_space; then
        log_warning "Proceeding with limited space - some models may fail"
    fi

    # Pull required models
    log_info "Installing required models..."
    failed_models=()

    for model in "${REQUIRED_MODELS[@]}"; do
        if model_exists "$model"; then
            log_info "Model $model already exists"
            if ! verify_model "$model"; then
                log_warning "Model $model exists but verification failed"
            fi
        else
            if pull_model "$model"; then
                verify_model "$model" || log_warning "Model $model pulled but verification failed"
            else
                failed_models+=("$model")
            fi
        fi
    done

    # Pull optional models if space allows
    if [ ${#failed_models[@]} -eq 0 ]; then
        log_info "Installing optional models..."

        for model in "${OPTIONAL_MODELS[@]}"; do
            if ! model_exists "$model"; then
                local size=$(estimate_model_size "$model")
                local available=$(get_available_space)

                if [ $size -lt $available ]; then
                    pull_model "$model" || log_warning "Failed to pull optional model $model"
                else
                    log_warning "Skipping $model - insufficient space (${size}MB needed, ${available}MB available)"
                fi
            fi
        done
    fi

    # Summary
    log_info "Ollama initialization complete!"

    # List installed models
    log_info "Installed models:"
    curl -sf "$OLLAMA_ENDPOINT/api/tags" | \
        grep -o '"name":"[^"]*"' | \
        sed 's/"name":"\(.*\)"/  - \1/' || log_warning "Could not list models"

    if [ ${#failed_models[@]} -gt 0 ]; then
        log_warning "Failed to install required models: ${failed_models[*]}"
        log_warning "CoDev.rs may have limited functionality"
        exit 1
    else
        log_success "All required models installed successfully!"

        # Create health check endpoint
        log_info "Setting up health monitoring..."
        curl -sf -X POST "$OLLAMA_ENDPOINT/api/generate" \
            -H "Content-Type: application/json" \
            -d '{"model": "codellama:7b", "prompt": "// Health check", "stream": false}' \
            >/dev/null && log_success "Health check configured"
    fi
}

# Run main function
main "$@"