#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# deploy-local.sh — Build images and deploy all Bittuly services to kind
# Usage: ./scripts/deploy-local.sh
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

echo "==> 🔨 Building Docker images..."
docker build -f services/auth-service/Dockerfile     -t bittuly/auth-service:local     .
docker build -f services/url-service/Dockerfile      -t bittuly/url-service:local      .
docker build -f services/consumer-service/Dockerfile -t bittuly/consumer-service:local .
docker build -f web/Dockerfile                       -t bittuly/frontend-service:local web/

echo "==> 📦 Loading images into kind cluster..."
kind load docker-image bittuly/auth-service:local     --name bittuly-local
kind load docker-image bittuly/url-service:local      --name bittuly-local
kind load docker-image bittuly/consumer-service:local --name bittuly-local
kind load docker-image bittuly/frontend-service:local --name bittuly-local

echo "==> 🚀 Applying Kubernetes manifests..."

# Namespace first
kubectl apply -f k8s/base/namespace.yaml

# Secrets and config
kubectl apply -f k8s/base/configmap.yaml
kubectl apply -f k8s/base/secret.yaml

# Infrastructure: databases, cache, broker, tracing
kubectl apply -f k8s/base/postgres-auth/init-configmap.yaml
kubectl apply -f k8s/base/postgres-auth/postgres-auth.yaml
kubectl apply -f k8s/base/postgres-urls/init-configmap.yaml
kubectl apply -f k8s/base/postgres-urls/postgres-urls.yaml
kubectl apply -f k8s/base/redis/redis.yaml
kubectl apply -f k8s/base/rabbitmq/rabbitmq.yaml
kubectl apply -f k8s/base/jaeger/jaeger.yaml

echo "==> ⏳ Waiting for databases to be ready..."
kubectl wait --namespace bittuly \
  --for=condition=ready pod \
  --selector=app=postgres-auth \
  --timeout=120s

kubectl wait --namespace bittuly \
  --for=condition=ready pod \
  --selector=app=postgres-urls \
  --timeout=300s

kubectl wait --namespace bittuly \
  --for=condition=ready pod \
  --selector=app=rabbitmq \
  --timeout=300s || true  # non-fatal: readiness probe takes longer

# Application services
kubectl apply -f k8s/base/auth-service/auth-service.yaml
kubectl apply -f k8s/base/url-service/url-service.yaml
kubectl apply -f k8s/base/consumer-service/consumer-service.yaml
kubectl apply -f k8s/base/frontend-service/frontend-service.yaml

# Ingress
kubectl apply -f k8s/base/ingress.yaml

echo ""
echo "✅ Deployment complete!"
echo ""
echo "  API Gateway:      http://localhost:8000"
echo "  Jaeger UI:        kubectl port-forward -n bittuly svc/jaeger 16686:16686"
echo "  RabbitMQ UI:      kubectl port-forward -n bittuly svc/rabbitmq 15672:15672"
echo ""
echo "  Watch pods:       kubectl get pods -n bittuly --watch"
