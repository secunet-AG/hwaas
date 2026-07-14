---
layout: ../../layouts/DocsLayout.astro
title: Getting Started
---

# Getting Started

Welcome to the HWaaS (Hardware as a Service) platform. This guide will help you get up and running quickly.

## Prerequisites

- An active HWaaS account with API credentials
- A tool for making HTTP requests (e.g. `curl`, Postman, or your language's HTTP client)

## Quick Start

### 1. Obtain Your API Key

After signing up, retrieve your API key from the dashboard. You'll need it for all authenticated requests.

```bash
export HWAAS_API_KEY="your-api-key-here"
```

### 2. Make Your First Request

Verify your connection by listing available devices:

```bash
curl -H "Authorization: Bearer $HWAAS_API_KEY" \
  https://api.hwaas.example.com/devices
```

You should receive a JSON response with your available devices:

```json
{
  "devices": [
    {
      "id": "dev-a1b2c3",
      "activation": true
    }
  ]
}
```

### 3. Interact with a Device

Once you have a device ID, you can send commands to it:

```bash
curl -X POST \
  -H "Authorization: Bearer $HWAAS_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"command": "status"}' \
  https://api.hwaas.example.com/devices/dev-a1b2c3/commands
```

## Next Steps

- Learn about [Authentication](/docs/authentication) options
- Explore [Device Management](/docs/devices)
- Browse the full [API Reference](/api)
