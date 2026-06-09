# Kubernetes Deployment for Kojacoord Proxy

This directory contains Kubernetes manifests for deploying Kojacoord Proxy in a Kubernetes cluster.

## Prerequisites

- Kubernetes cluster (v1.24+)
- kubectl configured to access your cluster
- StorageClass configured (for PVCs)
- Ingress controller (nginx recommended)
- cert-manager (for TLS certificates, optional)

## Quick Start

### 1. Create Namespace
```bash
kubectl apply -f namespace.yaml
```

### 2. Create ConfigMap
```bash
kubectl apply -f configmap.yaml
```

**Important:** Edit `configmap.yaml` to replace placeholder values:
- Update database URL with your MySQL connection string
- Change auth tokens to secure values
- Configure backend server addresses

### 3. Create Secrets (Optional)
For sensitive data like database passwords:
```bash
kubectl create secret generic kojacoord-secrets \
  --from-literal=db-password='your-password' \
  --from-literal=api-token='your-api-token' \
  -n kojacoord-system
```

Then update the ConfigMap to reference these secrets.

### 4. Deploy
```bash
kubectl apply -f deployment.yaml
kubectl apply -f service.yaml
kubectl apply -f ingress.yaml
kubectl apply -f horizontal-pod-autoscaler.yaml
```

### 5. Verify Deployment
```bash
kubectl get pods -n kojacoord-system
kubectl get services -n kojacoord-system
kubectl get ingress -n kojacoord-system
```

## Components

### Deployment
- **Replicas:** 3 (configurable)
- **Resource Limits:** 1Gi RAM, 1 CPU
- **Resource Requests:** 256Mi RAM, 250m CPU
- **Health Checks:** Liveness and readiness probes on HTTP API

### Services
- **LoadBalancer:** External access for Minecraft clients
- **ClusterIP:** Internal communication
- **Ports:**
  - 25577: Minecraft proxy
  - 8080: Server management API
  - 8081: HTTP API

### Persistent Volume Claims
- **kojacoord-data:** 1Gi for database and player data
- **kojacoord-plugins:** 500Mi for plugin files
- **kojacoord-logs:** 2Gi for log files

### Horizontal Pod Autoscaler
- **Min Replicas:** 2
- **Max Replicas:** 10
- **Metrics:** CPU (70%) and Memory (80%)

### Ingress
- **TLS:** Automatic with cert-manager
- **Routes:** 
  - `/api` → HTTP API
  - `/` → Minecraft proxy (for WebSocket support)

## Configuration

### Environment Variables
- `SQLX_OFFLINE`: Set to "true" if no database is available
- `RUST_LOG`: Logging level (debug, info, warn, error)

### Updating Configuration
```bash
kubectl edit configmap kojacoord-config -n kojacoord-system
kubectl rollout restart deployment kojacoord-proxy -n kojacoord-system
```

## Scaling

### Manual Scaling
```bash
kubectl scale deployment kojacoord-proxy --replicas=5 -n kojacoord-system
```

### Autoscaling
The HPA automatically scales based on CPU and memory usage. Adjust in `horizontal-pod-autoscaler.yaml`.

## Monitoring

### View Logs
```bash
kubectl logs -f deployment/kojacoord-proxy -n kojacoord-system
```

### View Pod Status
```bash
kubectl describe pod -l app=kojacoord-proxy -n kojacoord-system
```

### Check HPA Status
```bash
kubectl get hpa kojacoord-proxy-hpa -n kojacoord-system
```

## Troubleshooting

### Pods Not Starting
```bash
kubectl describe pod -l app=kojacoord-proxy -n kojacoord-system
kubectl logs deployment/kojacoord-proxy -n kojacoord-system
```

### Connection Issues
- Check service endpoints: `kubectl get endpoints kojacoord-proxy -n kojacoord-system`
- Verify firewall rules allow traffic on ports 25577, 8080, 8081
- Check ingress controller logs

### Database Connection
- Verify ConfigMap has correct database URL
- Ensure MySQL service is accessible from the cluster
- Check network policies if enabled

## Customization

### Adjust Resource Limits
Edit `deployment.yaml` under `resources` section.

### Change Replicas
Edit `deployment.yaml` under `spec.replicas`.

### Modify Storage
Edit PVC sizes in `deployment.yaml`.

### Add Custom Plugins
Mount plugin files to the plugins volume:
```bash
kubectl cp ./my-plugin.kpl kojacoord-proxy-0:/app/plugins -n kojacoord-system
kubectl rollout restart deployment kojacoord-proxy -n kojacoord-system
```

## Production Considerations

1. **Security:**
   - Use secrets for sensitive data
   - Enable RBAC
   - Use network policies
   - Enable pod security standards

2. **High Availability:**
   - Use multiple replicas (recommended 3+)
   - Configure pod anti-affinity
   - Use distributed storage

3. **Performance:**
   - Enable HPA for auto-scaling
   - Use node affinity for latency-sensitive workloads
   - Consider using SSD storage

4. **Monitoring:**
   - Install Prometheus and Grafana
   - Use metrics endpoint (port 9090)
   - Set up alerting

## Cleanup

```bash
kubectl delete -f ingress.yaml
kubectl delete -f horizontal-pod-autoscaler.yaml
kubectl delete -f service.yaml
kubectl delete -f deployment.yaml
kubectl delete -f configmap.yaml
kubectl delete -f namespace.yaml
```
