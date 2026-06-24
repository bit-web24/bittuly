#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# k8s.sh — Handy Kubernetes lifecycle commands for the bittuly local cluster
#
# Usage:
#   ./scripts/k8s.sh up        — Create cluster + deploy everything (first run)
#   ./scripts/k8s.sh start     — Start a stopped cluster (no rebuild)
#   ./scripts/k8s.sh stop      — Pause the cluster (containers stopped, data kept)
#   ./scripts/k8s.sh down      — Delete the cluster (keeps Docker images)
#   ./scripts/k8s.sh purge     — Delete cluster + remove all local Docker images
#   ./scripts/k8s.sh deploy    — Rebuild images + redeploy (like docker compose up --build)
#   ./scripts/k8s.sh status    — Show pod status across the cluster
#   ./scripts/k8s.sh logs      — Tail logs for a service  (e.g. k8s.sh logs auth-service)
#   ./scripts/k8s.sh restart   — Rolling restart a deployment (e.g. k8s.sh restart url-service)
#   ./scripts/k8s.sh jaeger    — Port-forward Jaeger UI  → http://localhost:16686
#   ./scripts/k8s.sh rabbitmq  — Port-forward RabbitMQ UI → http://localhost:15672
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

CLUSTER_NAME="bittuly-local"
NAMESPACE="bittuly"

# Colours
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
CYAN='\033[0;36m'; BOLD='\033[1m'; NC='\033[0m'

log()   { echo -e "${GREEN}==>${NC} $*"; }
warn()  { echo -e "${YELLOW}==>${NC} $*"; }
error() { echo -e "${RED}==>${NC} $*" >&2; }
info()  { echo -e "${CYAN}   ${NC} $*"; }
hr()    { echo -e "${CYAN}────────────────────────────────────────────${NC}"; }

cluster_exists() { kind get clusters 2>/dev/null | grep -q "^${CLUSTER_NAME}$"; }

# ─────────────────────────────────────────────────────────────────────────────
cmd_up() {
    if cluster_exists; then
        warn "Cluster '${CLUSTER_NAME}' already exists. Use 'deploy' to rebuild images."
        warn "To recreate from scratch run: ./scripts/k8s.sh down && ./scripts/k8s.sh up"
        exit 0
    fi

    log "Creating kind cluster '${CLUSTER_NAME}'..."
    kind create cluster --name "${CLUSTER_NAME}" --config k8s/base/kind-config.yaml

    log "Installing NGINX Ingress Controller..."
    kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/main/deploy/static/provider/kind/deploy.yaml
    kubectl wait --namespace ingress-nginx \
        --for=condition=available deployment/ingress-nginx-controller \
        --timeout=120s

    log "Building and deploying all services..."
    cmd_deploy
}

# ─────────────────────────────────────────────────────────────────────────────
cmd_deploy() {
    log "Building Docker images..."
    docker build -f services/auth-service/Dockerfile     -t bittuly/auth-service:local     .
    docker build -f services/url-service/Dockerfile      -t bittuly/url-service:local      .
    docker build -f services/consumer-service/Dockerfile -t bittuly/consumer-service:local .
    docker build -f web/Dockerfile                       -t bittuly/frontend-service:local  web/

    log "Loading custom application images into kind..."
    kind load docker-image bittuly/auth-service:local     --name "${CLUSTER_NAME}"
    kind load docker-image bittuly/url-service:local      --name "${CLUSTER_NAME}"
    kind load docker-image bittuly/consumer-service:local --name "${CLUSTER_NAME}"
    kind load docker-image bittuly/frontend-service:local --name "${CLUSTER_NAME}"

    log "Pre-loading third-party dependencies from local Docker cache to speed up boot..."
    for img in postgres:17-alpine redis:7-alpine rabbitmq:3-management jaegertracing/all-in-one:latest; do
        docker pull "$img" >/dev/null 2>&1 || true
        kind load docker-image "$img" --name "${CLUSTER_NAME}" 2>/dev/null || true
    done

    log "Applying Kubernetes manifests..."
    kubectl apply -f k8s/base/namespace.yaml
    kubectl apply -f k8s/base/configmap.yaml
    kubectl apply -f k8s/base/secret.yaml
    kubectl apply -f k8s/base/postgres-auth/init-configmap.yaml
    kubectl apply -f k8s/base/postgres-auth/postgres-auth.yaml
    kubectl apply -f k8s/base/postgres-urls/init-configmap.yaml
    kubectl apply -f k8s/base/postgres-urls/postgres-urls.yaml
    kubectl apply -f k8s/base/redis/redis.yaml
    kubectl apply -f k8s/base/rabbitmq/rabbitmq.yaml
    kubectl apply -f k8s/base/jaeger/jaeger.yaml

    log "Waiting for databases to be ready..."
    kubectl wait -n "${NAMESPACE}" --for=condition=available deployment/postgres-auth --timeout=300s
    kubectl wait -n "${NAMESPACE}" --for=condition=available deployment/postgres-urls --timeout=300s

    kubectl apply -f k8s/base/auth-service/auth-service.yaml
    kubectl apply -f k8s/base/url-service/url-service.yaml
    kubectl apply -f k8s/base/consumer-service/consumer-service.yaml
    kubectl apply -f k8s/base/frontend-service/frontend-service.yaml
    kubectl apply -f k8s/base/ingress.yaml

    hr
    echo -e "${BOLD}✅  Deployment complete!${NC}"
    hr
    info "Frontend:    http://localhost:8000"
    info "Auth API:    http://localhost:8000/api/auth/health"
    info "URL API:     http://localhost:8000/api/urls/health"
    info ""
    info "Jaeger UI:   ./scripts/k8s.sh jaeger"
    info "RabbitMQ UI: ./scripts/k8s.sh rabbitmq"
    info "Pod status:  ./scripts/k8s.sh status"
    hr
}

# ─────────────────────────────────────────────────────────────────────────────
# stop — pause the Docker containers that make up the cluster (data preserved)
cmd_stop() {
    if ! cluster_exists; then error "Cluster '${CLUSTER_NAME}' does not exist."; exit 1; fi
    log "Stopping cluster containers (data preserved)..."
    # kind doesn't have a native stop; we stop the underlying Docker containers
    docker ps --filter "label=io.x-k8s.kind.cluster=${CLUSTER_NAME}" -q \
        | xargs -r docker stop
    log "Cluster stopped. Run './scripts/k8s.sh start' to resume."
}

# ─────────────────────────────────────────────────────────────────────────────
# start — resume previously stopped cluster containers
cmd_start() {
    if ! cluster_exists; then error "Cluster '${CLUSTER_NAME}' does not exist. Run 'up' first."; exit 1; fi
    log "Starting cluster containers..."
    docker ps -a --filter "label=io.x-k8s.kind.cluster=${CLUSTER_NAME}" -q \
        | xargs -r docker start
    log "Waiting for API server to become ready..."
    # Give the API server a moment then check node readiness
    sleep 5
    kubectl wait --for=condition=ready node --all --timeout=60s 2>/dev/null || true
    log "Cluster is back online."
    info "Frontend: http://localhost:8000"
}

# ─────────────────────────────────────────────────────────────────────────────
# down — delete the cluster entirely (keeps Docker images for fast redeploy)
cmd_down() {
    if ! cluster_exists; then warn "Cluster '${CLUSTER_NAME}' does not exist."; exit 0; fi
    log "Deleting cluster '${CLUSTER_NAME}'..."
    kind delete cluster --name "${CLUSTER_NAME}"
    log "Cluster deleted. Docker images are still cached — 'up' will be fast."
}

# ─────────────────────────────────────────────────────────────────────────────
# purge — delete cluster AND all local bittuly Docker images (full reset)
cmd_purge() {
    cmd_down || true
    log "Removing bittuly Docker images..."
    docker images --filter "reference=bittuly/*" -q | xargs -r docker rmi -f
    log "Purge complete. Next 'up' will rebuild everything from scratch."
}

# ─────────────────────────────────────────────────────────────────────────────
cmd_status() {
    hr
    echo -e "${BOLD}  Nodes${NC}"
    hr
    kubectl get nodes 2>/dev/null || echo "  Cluster not running"
    hr
    echo -e "${BOLD}  Pods — namespace: ${NAMESPACE}${NC}"
    hr
    kubectl get pods -n "${NAMESPACE}" -o wide 2>/dev/null || echo "  No pods found"
    hr
    echo -e "${BOLD}  Services${NC}"
    hr
    kubectl get svc -n "${NAMESPACE}" 2>/dev/null || true
}

# ─────────────────────────────────────────────────────────────────────────────
cmd_logs() {
    local svc="${2:-}"
    if [ -z "$svc" ]; then
        error "Usage: ./scripts/k8s.sh logs <service-name>"
        info  "  e.g. ./scripts/k8s.sh logs auth-service"
        exit 1
    fi
    kubectl logs -n "${NAMESPACE}" -l "app=${svc}" -f --tail=100
}

# ─────────────────────────────────────────────────────────────────────────────
cmd_restart() {
    local svc="${2:-}"
    if [ -z "$svc" ]; then
        error "Usage: ./scripts/k8s.sh restart <deployment-name>"
        info  "  e.g. ./scripts/k8s.sh restart url-service"
        exit 1
    fi
    kubectl rollout restart deployment/"${svc}" -n "${NAMESPACE}"
    kubectl rollout status  deployment/"${svc}" -n "${NAMESPACE}"
}

# ─────────────────────────────────────────────────────────────────────────────
cmd_jaeger() {
    log "Port-forwarding Jaeger UI → http://localhost:16686 (Ctrl-C to stop)"
    kubectl port-forward -n "${NAMESPACE}" svc/jaeger 16686:16686
}

cmd_rabbitmq() {
    log "Port-forwarding RabbitMQ UI → http://localhost:15672 (Ctrl-C to stop)"
    kubectl port-forward -n "${NAMESPACE}" svc/rabbitmq 15672:15672
}

# ─────────────────────────────────────────────────────────────────────────────
usage() {
    hr
    echo -e "${BOLD}  bittuly k8s — local cluster manager${NC}"
    hr
    echo -e "  ${GREEN}up${NC}              Create cluster + deploy everything (first run)"
    echo -e "  ${GREEN}start${NC}           Resume a stopped cluster (no rebuild)"
    echo -e "  ${GREEN}stop${NC}            Pause the cluster (data preserved, like docker stop)"
    echo -e "  ${GREEN}down${NC}            Delete the cluster (images kept, fast redeploy)"
    echo -e "  ${GREEN}purge${NC}           Delete cluster + all Docker images (full reset)"
    echo -e "  ${GREEN}deploy${NC}          Rebuild images + redeploy all services"
    echo -e "  ${GREEN}status${NC}          Show nodes, pods, and services"
    echo -e "  ${GREEN}logs${NC} <svc>      Tail pod logs   (e.g. logs auth-service)"
    echo -e "  ${GREEN}restart${NC} <svc>   Rolling restart  (e.g. restart url-service)"
    echo -e "  ${GREEN}jaeger${NC}          Port-forward Jaeger UI  → :16686"
    echo -e "  ${GREEN}rabbitmq${NC}        Port-forward RabbitMQ UI → :15672"
    hr
}

# ─────────────────────────────────────────────────────────────────────────────
CMD="${1:-help}"
case "$CMD" in
    up)       cmd_up ;;
    start)    cmd_start ;;
    stop)     cmd_stop ;;
    down)     cmd_down ;;
    purge)    cmd_purge ;;
    deploy)   cmd_deploy ;;
    status)   cmd_status ;;
    logs)     cmd_logs "$@" ;;
    restart)  cmd_restart "$@" ;;
    jaeger)   cmd_jaeger ;;
    rabbitmq) cmd_rabbitmq ;;
    help|--help|-h) usage ;;
    *)
        error "Unknown command: '$CMD'"
        usage
        exit 1
        ;;
esac
