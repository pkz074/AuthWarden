# Kubernetes

These manifests deploy the AuthWarden application only. PostgreSQL and Redis are treated as external managed services or separately deployed cluster services.

## Files

- `namespace.yaml`: creates the `authwarden` namespace
- `configmap.yaml`: non-secret runtime config
- `secret.example.yaml`: example secret values for database, Redis, and JWT config
- `deployment.yaml`: AuthWarden app deployment
- `service.yaml`: internal service for the app
- `ingress.yaml`: public HTTP/TLS entrypoint placeholder
- `ingress-tls.md`: ingress, DNS, and TLS deployment notes
- `kustomization.yaml`: applies all manifests together

## Usage

Copy the example secret before applying:

```sh
cp k8s/secret.example.yaml k8s/secret.yaml
```

Edit `k8s/secret.yaml` with real values.

Apply with:

```sh
kubectl apply -k k8s
```

## Notes

- Replace `authwarden.example.com` in `ingress.yaml`.
- Replace the image tag if you do not want to deploy `latest`.
- The deployment starts with one replica because the app currently runs migrations on startup.
- The ingress assumes an NGINX ingress controller and cert-manager with a `letsencrypt-prod` ClusterIssuer.
- Read `ingress-tls.md` before applying the ingress to a real cluster.
- Do not commit real secrets.
