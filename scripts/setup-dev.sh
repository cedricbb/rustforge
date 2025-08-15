#!/bin/bash
set -e

echo "🚀 Setting up Codev.rs development environment..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
  echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
  echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
  echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
  echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running in Docker
if [ -f /.dockerenv ]; then
  print_status "Running inside Docker container"
  IN_DOCKER=true
else
  IN_DOCKER=false
fi

# Function to check if command exists
command_exists() {
  command -v "$1" >/dev/null 2>&1
}

# Install Rust if not present
if ! command_exists cargo; then
  print_status "Installing Rust..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source ~/.cargo/env
  print_success "Rust installed successfully"
else
  print_status "Rust already installed: $(rustc --version)"
fi

# Update Rust to latest stable
print_status "Updating Rust to latest stable..."
rustup update stable
rustup default stable

# Install required components
print_status "Installing Rust components..."
rustup component add clippy rustfmt rust-analyzer

# Install development tools
print_status "Installing development tools..."
cargo install --locked cargo-watch cargo-tarpaulin cargo-audit just

# Check if Docker is available (not needed when running inside Docker)
if [ "$IN_DOCKER" = false]; then
  if ! command_exists docker; then
    print_warning "Docker not found. Please install Docker to use containerized development."
    print_status " You can continue with local development without containers."
  else
    print_success "Docker found: $(docker --version)"

    # Check if Docker Compose is available
    if ! command_exists docker-compose && ! docker compose version >/dev/null 2>&1; then
      print_warning "Docker Compose not found. Please install Docker Compose."
    else
      print_success "Docker Compose available"
    fi
  fi

  # Setup Ollama for local development (if not in Docker)
  if ! command_exists ollama; then
    print_status "Installing Ollama..."
    curl -fsSL https://ollama.ai/install.sh | sh
    print_success "Ollama installed"
  else
    print_status "Ollama already installed"
  fi

  # Start Ollama service if not running
  if ! pgrep ollama >/dev/null; then
    print_status "Starting Ollama service..."
    ollama serve &
    sleep 5
  fi

  # Pull required models
  print_status "Pulling required Ollama models..."
  ollama pull codellama:7b-code || print_warning "Failed to pull codellama:7b-code"
  ollama pull deepseek-coder:6.7b-instruct || print_warning "Failed to pull deepseek-coder:6.7b-instruct"
  ollama pull llama3:8b || print_warning "Failed to pull llama3:8b"
fi

# Create necessary directories
print_status "Creating project directories..."
mkdir -p workspace
mkdir -p data/knowledge-base
mkdir -p data/sessions
mkdir -p config
mkdir -p logs

# Set proper permissions
chmod 755 workspace data config logs
chmod 755 scripts/*.sh 2>/dev/null || true

# Create development configuration
print_status "Creating development configuration..."
cat > config/dev.toml << 'EOF'
environment = "Development"

[ai]
default_provider = "Ollama"
auto_detect_environment = true
fallback_chain = ["Ollama", "Mistral", "Claude", "OpenAI", "Gemini"]

[ai.providers.Ollama]
enabled = true
model = "codellama:7b"\
max_tokens = 4096
temperature = 0.1
endpoint = "http://localhost:11434"
timeout_seconds = 30
max_retries = 3

[ai.providers.Mistral]
enabled = false
model = "mistral-medium"

[security]
default_level = "Development"
sandbox_enabled = false

[development]
hot_reload = true
debug_mode = true
dev_server_port = 8080
mock_response = false

[logging]
level = "debug"
format = "pretty"
file_enabled = true
file_path = "logs/codev.log"

[workspace]
default_path = "./workspace"
auto_detect_project = true
ignore_patterns = [".git", "target", "node_modules", "__pycache__", "*.tmp"]
EOF

# Create .env file template
print_status "Creating environment file template..."
cat > .env.example << 'EOF'
# CoDev.rs Environment Configuration

# Environment (development, production, testing)
CODEV_ENV=development

# AI Provider Configuration
CODEV_AI_PROVIDER=ollama
OLAMA_ENDPOINT=http://localhost:11434

# External API Keys (optional)
# OPENAI_API_KEY=your-key
# ANTHROPIC_API_KEY=your-key
# MISTRAL_API_KEY=your-key
# GOOGLE_API_KEY=your-key

# Security Settings
CODEV_SECURITY_LEVEL=development

# Logging
RUST_LOG=debug
RUST_BACKTRACE=1

# Workspace
CODEV_WORKSPACE=./workspace
CODEV_DATA_DIR=./data
EOF

# Copy .env.example to .env if it doesn't exist
if [ 1 -f .env ]; then
  cp .env.example .env
  print_status "Created .env file from template"
fi

# Create Justfile for development commands
print_status "Creating Justfile for development commands..."
cat > justfile << 'EOF'
# CoDev.rs Development Commands

# Default recipe
default:
    @just --list

# Development commands
dev:
    cargo watch -x "run --bin codev-cli"

dev-tui:
    cargo watch -x "run --bin codev-tui"

# Testing
test:
    cargo test --workspace

test-integration:
    cargo test --workspace --features="integration-tests"

test-ai:
    cargo test ai_providers --features="test-ollama,test-apis"

# Quality checks
lint:
    cargo clippy --workspace -- -D warnings
    cargo fmt --check

fix:
    cargo clippy --workspace --fix --allow-staged
    cargo fmt

# Documentation
docs:
    cargo doc --workspace --open

# Docker operations
docker-dev:
    docker-compose -f docker-compose.yml up --build

docker-test:
    docker-compose -f docker-compose.test.yml up --build --abort-on-container-exit

# Ollama management
ollama-setup:
    ollama pull codellama:7b
    ollama pull codellama:13b
    ollama pull llama2:7b

ollama-test:
    curl -X POST http://localhost:11434/api/generate -d '{"model": "codellama:7b", "prompt": "fn main() {", "stream": false}'

# Release
release version:
    git tag v{{version}}
    cargo release {{version}}

# Clean
clean:
    cargo clean
    docker system prune -f
EOF

# Build the project to check everything works
print_status "Building project..."
if cargo build; then
  print_success "Project build successfully"
else
  print_error "Build failed. Please check the errors above."
  exit 1
fi

# Test Ollama connection if not in Docker
if [ "$IN_DOCKER" = false] && command_exists ollama; then
  print_status "Testing Ollama connection..."
  if curl -s http://localhost:11434/api/tags >/dev/null; then
    print_success "Ollama is running and accessible"
  else
    print_warning "Ollama service may not be running. Start it with: ollama serve"
  fi
fi

print_success "✅ Development environment setup complete!"
echo
print_status "Next steps:"
echo " 1. Source your shell: source ~/.cargo/env"
echo " 2. Start development: just dev"
echo " 3. Run tests: just test"
echo " 4. Use Docker: just docker-dev"
echo
print_status "Configuration files created:"
echo " - config/dev.toml (development config)"
echo " - .env (environment variables)"
echo " - justfile (development commands)"
echo
print_status "Happy coding with CoDev.rs! 🦀✨"