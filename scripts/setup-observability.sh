#!/bin/bash

# Chalkbyte API - Observability Stack Setup Script
# This script sets up and verifies the OpenTelemetry observability stack

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
COMPOSE_FILE="docker-compose.observability.yml"
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo -e "${BLUE}╔════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  Chalkbyte Observability Stack Setup      ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════╝${NC}"
echo ""

# Function to print status
print_status() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

# Check if Docker is running
check_docker() {
    print_info "Checking Docker..."
    if ! docker info > /dev/null 2>&1; then
        print_error "Docker is not running. Please start Docker and try again."
        exit 1
    fi
    print_status "Docker is running"
}

# Check if docker-compose is available
check_docker_compose() {
    print_info "Checking Docker Compose..."
    if ! command -v docker-compose &> /dev/null; then
        print_error "docker-compose is not installed. Please install it and try again."
        exit 1
    fi
    print_status "Docker Compose is available"
}

# Create necessary directories
create_directories() {
    print_info "Creating necessary directories..."
    mkdir -p "$PROJECT_ROOT/storage/logs"
    mkdir -p "$PROJECT_ROOT/observability/grafana/provisioning/datasources"
    mkdir -p "$PROJECT_ROOT/observability/grafana/provisioning/dashboards"
    mkdir -p "$PROJECT_ROOT/observability/grafana/dashboards"
    print_status "Directories created"
}

# Check if .env file exists and has required variables
check_env_file() {
    print_info "Checking environment variables..."

    if [ ! -f "$PROJECT_ROOT/.env" ]; then
        print_warning ".env file not found. Creating from example..."
        if [ -f "$PROJECT_ROOT/.env.example" ]; then
            cp "$PROJECT_ROOT/.env.example" "$PROJECT_ROOT/.env"
        else
            touch "$PROJECT_ROOT/.env"
        fi
    fi

    # Check for required variables
    if ! grep -q "OTEL_EXPORTER_OTLP_ENDPOINT" "$PROJECT_ROOT/.env"; then
        print_warning "Adding OTEL_EXPORTER_OTLP_ENDPOINT to .env"
        echo "" >> "$PROJECT_ROOT/.env"
        echo "# OpenTelemetry Configuration" >> "$PROJECT_ROOT/.env"
        echo "OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317" >> "$PROJECT_ROOT/.env"
        echo "ENVIRONMENT=development" >> "$PROJECT_ROOT/.env"
    fi

    print_status "Environment variables configured"
}

# Start the observability stack
start_stack() {
    print_info "Starting observability stack..."
    cd "$PROJECT_ROOT"

    docker-compose -f "$COMPOSE_FILE" up -d

    print_status "Observability stack started"
}

# Wait for services to be healthy
wait_for_services() {
    print_info "Waiting for services to be ready..."

    services=(
        "http://localhost:13133|OpenTelemetry Collector"
        "http://localhost:3200/ready|Tempo"
        "http://localhost:3100/ready|Loki"
        "http://localhost:9090/-/ready|Prometheus"
        "http://localhost:3001/api/health|Grafana"
    )

    max_attempts=30
    attempt=0

    for service_info in "${services[@]}"; do
        IFS='|' read -r url name <<< "$service_info"

        attempt=0
        while [ $attempt -lt $max_attempts ]; do
            if curl -sf "$url" > /dev/null 2>&1; then
                print_status "$name is ready"
                break
            fi

            attempt=$((attempt + 1))
            if [ $attempt -eq $max_attempts ]; then
                print_warning "$name is not responding (this might be okay)"
                break
            fi

            sleep 2
        done
    done
}

# Display service URLs
display_urls() {
    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║          Service URLs                      ║${NC}"
    echo -e "${BLUE}╚════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${GREEN}Grafana:${NC}              http://localhost:3001"
    echo -e "                      Username: admin"
    echo -e "                      Password: admin123"
    echo ""
    echo -e "${GREEN}Prometheus:${NC}           http://localhost:9090"
    echo -e "${GREEN}Tempo:${NC}                http://localhost:3200"
    echo -e "${GREEN}Loki:${NC}                 http://localhost:3100"
    echo -e "${GREEN}OTel Collector:${NC}       http://localhost:8888"
    echo ""
    echo -e "${GREEN}API Metrics:${NC}          http://localhost:3000/metrics"
    echo -e "${GREEN}API Health:${NC}           http://localhost:3000/health"
    echo ""
}

# Verify the setup
verify_setup() {
    print_info "Verifying setup..."
    echo ""

    # Check if all containers are running
    containers=(
        "chalkbyte-otel-collector"
        "chalkbyte-tempo"
        "chalkbyte-loki"
        "chalkbyte-promtail"
        "chalkbyte-prometheus"
        "chalkbyte-grafana"
        "chalkbyte-node-exporter"
    )

    all_running=true
    for container in "${containers[@]}"; do
        if docker ps --format '{{.Names}}' | grep -q "^${container}$"; then
            print_status "$container is running"
        else
            print_error "$container is not running"
            all_running=false
        fi
    done

    echo ""
    if $all_running; then
        print_status "All services are running successfully!"
    else
        print_error "Some services failed to start. Check logs with:"
        echo "  docker-compose -f $COMPOSE_FILE logs"
    fi
}

# Show next steps
show_next_steps() {
    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║          Next Steps                        ║${NC}"
    echo -e "${BLUE}╚════════════════════════════════════════════╝${NC}"
    echo ""
    echo "1. Start your Chalkbyte API:"
    echo "   ${YELLOW}cargo run${NC}"
    echo ""
    echo "2. Access Grafana and explore the dashboard:"
    echo "   ${YELLOW}http://localhost:3001${NC}"
    echo ""
    echo "3. Generate some test traffic:"
    echo "   ${YELLOW}curl http://localhost:3000/health${NC}"
    echo ""
    echo "4. View metrics endpoint:"
    echo "   ${YELLOW}curl http://localhost:3000/metrics${NC}"
    echo ""
    echo "5. Check logs:"
    echo "   ${YELLOW}docker-compose -f $COMPOSE_FILE logs -f [service-name]${NC}"
    echo ""
    echo "6. Stop the observability stack:"
    echo "   ${YELLOW}docker-compose -f $COMPOSE_FILE down${NC}"
    echo ""
    echo "For more information, see: docs/OBSERVABILITY.md"
    echo ""
}

# Main execution
main() {
    check_docker
    check_docker_compose
    create_directories
    check_env_file

    echo ""
    read -p "Do you want to start the observability stack now? (y/n) " -n 1 -r
    echo ""

    if [[ $REPLY =~ ^[Yy]$ ]]; then
        start_stack
        wait_for_services
        verify_setup
        display_urls
        show_next_steps
    else
        print_info "Setup completed. Run this script again to start the stack."
    fi
}

# Handle script arguments
case "${1:-}" in
    start)
        start_stack
        wait_for_services
        display_urls
        ;;
    stop)
        print_info "Stopping observability stack..."
        cd "$PROJECT_ROOT"
        docker-compose -f "$COMPOSE_FILE" down
        print_status "Observability stack stopped"
        ;;
    restart)
        print_info "Restarting observability stack..."
        cd "$PROJECT_ROOT"
        docker-compose -f "$COMPOSE_FILE" restart
        wait_for_services
        print_status "Observability stack restarted"
        display_urls
        ;;
    status)
        print_info "Checking service status..."
        cd "$PROJECT_ROOT"
        docker-compose -f "$COMPOSE_FILE" ps
        ;;
    logs)
        shift
        cd "$PROJECT_ROOT"
        docker-compose -f "$COMPOSE_FILE" logs -f "$@"
        ;;
    clean)
        print_warning "This will remove all containers and volumes!"
        read -p "Are you sure? (y/n) " -n 1 -r
        echo ""
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            cd "$PROJECT_ROOT"
            docker-compose -f "$COMPOSE_FILE" down -v
            print_status "Observability stack cleaned"
        fi
        ;;
    help|--help|-h)
        echo "Usage: $0 [command]"
        echo ""
        echo "Commands:"
        echo "  (none)    Run interactive setup"
        echo "  start     Start the observability stack"
        echo "  stop      Stop the observability stack"
        echo "  restart   Restart the observability stack"
        echo "  status    Show service status"
        echo "  logs      Follow logs (optionally specify service)"
        echo "  clean     Remove all containers and volumes"
        echo "  help      Show this help message"
        echo ""
        echo "Examples:"
        echo "  $0                    # Interactive setup"
        echo "  $0 start              # Start all services"
        echo "  $0 logs grafana       # View Grafana logs"
        echo "  $0 clean              # Clean everything"
        ;;
    *)
        main
        ;;
esac
