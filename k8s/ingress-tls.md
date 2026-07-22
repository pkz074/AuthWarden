# Ingress and TLS Plan

AuthWarden's Kubernetes ingress is intentionally a production-shaped placeholder. It should not be applied unchanged to a real cluster.

## Assumptions

- Ingress controller: NGINX Ingress Controller
- Certificate manager: cert-manager
- Cluster issuer: `letsencrypt-prod`
- Public host placeholder: `authwarden.example.com`
- TLS secret created by cert-manager: `authwarden-tls`

## Before Deploying

1. Replace `authwarden.example.com` in `ingress.yaml` with the real hostname.
2. Point DNS for that hostname to the ingress controller's external address.
3. Confirm the cluster has an ingress class named `nginx`.
4. Confirm cert-manager is installed.
5. Confirm a `ClusterIssuer` named `letsencrypt-prod` exists, or change the annotation to the issuer your cluster uses.
6. Apply the manifests and confirm cert-manager creates the `authwarden-tls` secret.

## Staging Recommendation

For first deployment, use a staging issuer before production Let's Encrypt:

```yaml
cert-manager.io/cluster-issuer: letsencrypt-staging
```

After the certificate flow works, switch back to:

```yaml
cert-manager.io/cluster-issuer: letsencrypt-prod
```

## Verification

After deployment:

```sh
kubectl get ingress -n authwarden
kubectl describe certificate -n authwarden
kubectl get secret authwarden-tls -n authwarden
curl -I https://authwarden.example.com/health
```

The expected result is:

- ingress has an external address
- certificate is `Ready=True`
- `authwarden-tls` exists
- `/health` returns `200 OK` over HTTPS
