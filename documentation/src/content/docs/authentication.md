# Authentication

HWaaS supports multiple authentication methods to integrate with your infrastructure.

## API Key

The simplest method. Pass your key via the `Authorization` header:

```bash
curl -H "Authorization: Bearer YOUR_API_KEY" \
  https://api.hwaas.example.com/devices
```

API keys can be scoped to specific permissions and rotated from the dashboard.

## Key Locations

Depending on your integration, the API key can be sent in different locations:

| Location | Example                       |
| -------- | ----------------------------- |
| Header   | `Authorization: Bearer <key>` |
| Query    | `?api_key=<key>`              |
| Cookie   | `Set-Cookie: api_key=<key>`   |

The recommended approach is always the **header**, as query parameters may be logged and cookies can introduce CSRF risks.

## Generating Keys

```python
import requests

response = requests.post(
    "https://api.hwaas.example.com/auth/keys",
    headers={"Authorization": "Bearer ADMIN_KEY"},
    json={
        "name": "ci-pipeline",
        "scopes": ["devices:read", "devices:write"],
        "expires_in": 86400
    }
)

key = response.json()
print(f"New key: {key['token']}")
```

## Key Rotation

To rotate a key without downtime, create a new key before revoking the old one:

```bash
# Create new key
NEW_KEY=$(curl -s -X POST \
  -H "Authorization: Bearer $HWAAS_API_KEY" \
  https://api.hwaas.example.com/auth/keys | jq -r '.token')

# Update your services to use $NEW_KEY, then revoke the old one
curl -X DELETE \
  -H "Authorization: Bearer $NEW_KEY" \
  https://api.hwaas.example.com/auth/keys/old-key-id
```

## Rate Limits

Authenticated requests are rate-limited per API key:

- **Standard tier**: 100 requests/minute
- **Pro tier**: 1,000 requests/minute

When rate-limited, the API returns `429 Too Many Requests` with a `Retry-After` header.
