# Device Management

Devices are the core resource in HWaaS. Each device represents a physical piece of hardware you can provision, monitor, and control through the API.

## Listing Devices

```bash
curl -H "Authorization: Bearer $HWAAS_API_KEY" \
  https://api.hwaas.example.com/devices
```

Response:

```json
{
  "devices": [
    {
      "id": "dev-a1b2c3",
      "activation": true
    },
    {
      "id": "dev-d4e5f6",
      "activation": false
    }
  ]
}
```

## Device Status

The `activation` field indicates whether a device is currently active and responding to commands.

| Status  | Meaning                          |
| ------- | -------------------------------- |
| `true`  | Device is online and available   |
| `false` | Device is offline or deactivated |

## Activating a Device

```typescript
const response = await fetch(
  "https://api.hwaas.example.com/devices/dev-a1b2c3/activate",
  {
    method: "POST",
    headers: {
      Authorization: `Bearer ${apiKey}`,
      "Content-Type": "application/json",
    },
  },
);

const device = await response.json();
console.log(`Device ${device.id} is now active: ${device.activation}`);
```

## Deactivating a Device

```typescript
await fetch("https://api.hwaas.example.com/devices/dev-a1b2c3/deactivate", {
  method: "POST",
  headers: {
    Authorization: `Bearer ${apiKey}`,
  },
});
```

## Auxiliary Devices

Some primary devices have auxiliary peripherals attached. These are returned as nested resources:

```bash
curl -H "Authorization: Bearer $HWAAS_API_KEY" \
  https://api.hwaas.example.com/devices/dev-a1b2c3/auxiliary
```

```json
{
  "auxiliary_devices": [
    {
      "id": "aux-001",
      "activation": true
    }
  ]
}
```

## Best Practices

- **Poll sparingly** — use webhooks when available instead of polling for device status changes
- **Handle offline devices gracefully** — commands sent to deactivated devices will return `409 Conflict`
- **Use idempotent requests** — activation and deactivation calls are idempotent, so retries are safe
